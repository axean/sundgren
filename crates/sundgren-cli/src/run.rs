//! The `run` subcommand: worktrees, preset commands on both sides, then diff. Owned by W5.
//!
//! Port of cli.py's `_cmd_run`, `_resolve_preset`, `_default_preset_name` and `_parse_env`, with
//! the same messages and exit codes. The pipeline itself (`_run_side`, the workspace block) is
//! `sundgren_run::pipeline`.

use std::path::{Path, PathBuf};

use sundgren_core::model::ReportWarning;
use sundgren_core::pytext::PyText;
use sundgren_run::pipeline::{PairConfig, PipelineError, run_pair};
use sundgren_run::presets::Preset;
use sundgren_run::pyfmt::repr_str;
use sundgren_run::runner::{RunError, TemplateRunner, adhoc_preset};
use sundgren_run::template::default_python;
use sundgren_run::workspace::{WorkspaceError, repo_root};

use crate::cli::{
    CliError, CliResult, EXIT_FAILURE, EXIT_OK, EXIT_PATCH, EXIT_USAGE, Error, Host, Json,
    PYTHON_MARKERS, RUST_MARKERS, RunArgs, build_report, emit, junit_read_error, load_all_presets,
    meta, patch_meta, resolve,
};

/// A Python truthy `str | None` argument.
fn given(arg: &Option<String>) -> Option<&str> {
    arg.as_deref().filter(|s| !s.is_empty())
}

pub fn cmd_run(args: &RunArgs, host: &mut dyn Host) -> CliResult<i32> {
    let cmd = args.cmd.as_ref().filter(|c| !c.is_empty());
    if args.repeat < 1 {
        return Err(CliError::new("--repeat must be >= 1", EXIT_USAGE).into());
    }
    if cmd.is_none() && (given(&args.lcov).is_some() || given(&args.junit).is_some() || args.shell)
    {
        return Err(CliError::new(
            "--lcov, --junit and --shell only apply with --cmd",
            EXIT_USAGE,
        )
        .into());
    }
    let patch_path = PathBuf::from(resolve(&args.patch));
    if !patch_path.is_file() {
        return Err(CliError::new(format!("patch not found: {}", args.patch), EXIT_USAGE).into());
    }
    let repo = repo_root(Path::new(&args.repo)).map_err(|e| {
        CliError::new(
            format!("not a git repository: {} ({})", args.repo, os_or_git(&e)),
            EXIT_USAGE,
        )
    })?;

    let preset = resolve_preset(args, &repo, host)?;
    let env = parse_env(&args.env)?;
    let python = match given(&args.python) {
        Some(p) => p.to_owned(),
        None => default_python(|n| host.which(n)),
    };
    let mut warnings: Vec<ReportWarning> = Vec::new();
    let runner = TemplateRunner::new(preset.clone(), python, env);
    let cfg = PairConfig {
        repo: &repo,
        base: &args.base,
        patch_path: &patch_path,
        workdir: given(&args.workdir).map(|w| PathBuf::from(resolve(w))),
        keep_worktrees: args.keep_worktrees,
        preset: &preset,
        runner: &runner,
        tests: (!args.tests.is_empty()).then_some(args.tests.as_slice()),
        repeat: args.repeat,
    };
    let selectors = |id: &str| host.selector(id);
    let symbolizers = |path: &PyText| host.symbolizer(path);
    let sides = run_pair(&cfg, &selectors, &symbolizers, &mut warnings).map_err(pipeline_error)?;

    let commands = runner
        .display_commands(&sides.patched_selection)
        .map_err(|e| Error::Host(Box::new(e)))?;
    let selection = Json::object([
        ("tests", Json::from(sides.patched_selection.tests.clone())),
        ("run_all", Json::from(sides.patched_selection.run_all)),
        ("reason", Json::from(sides.patched_selection.reason.clone())),
    ]);
    let inputs = Json::object([
        ("base_lcov_sha256", Json::from(sides.base_sha)),
        ("patched_lcov_sha256", Json::from(sides.patched_sha)),
    ]);
    let meta = meta(
        host,
        vec![
            ("mode", Json::from("run")),
            ("base_ref", Json::from(sides.base_ref)),
            ("patch", patch_meta(&args.patch, &patch_path, &sides.patch)?),
            ("preset", Json::from(preset.name.clone())),
            ("commands", Json::from(commands)),
            ("selection", selection),
            ("repeat", Json::from(args.repeat)),
            ("inputs", inputs),
        ],
    )?;
    let report = build_report(&sides.delta, meta, &args.output, &warnings);
    emit(host, &report, &args.output, &preset.hints)?;
    Ok(EXIT_OK)
}

/// The except clauses for pipeline errors: the prototype's messages and exit codes.
fn pipeline_error(e: PipelineError) -> Error {
    match e {
        PipelineError::Workspace(e) => workspace_error(e),
        PipelineError::Run(RunError::Failure { message, log_tail }) => CliError::new(
            format!("{message}\n--- log tail ---\n{log_tail}"),
            EXIT_FAILURE,
        )
        .into(),
        PipelineError::Run(RunError::Value(msg)) => Error::Host(msg.into()),
        PipelineError::Run(RunError::Io(e)) => e.into(),
        PipelineError::Failure(msg) => CliError::new(msg, EXIT_FAILURE).into(),
        PipelineError::JunitRead(path, e) => junit_read_error(&path, &e),
        PipelineError::Io(e) => e.into(),
        PipelineError::Host(e) => Error::Host(e),
    }
}

/// The except clauses for workspace errors.
fn workspace_error(e: WorkspaceError) -> Error {
    match e {
        WorkspaceError::PatchApply(msg) => {
            CliError::new(format!("patch does not apply:\n{msg}"), EXIT_PATCH).into()
        }
        WorkspaceError::BadRef(msg) => CliError::new(msg, EXIT_USAGE).into(),
        WorkspaceError::Git(msg) => CliError::new(format!("git: {msg}"), EXIT_FAILURE).into(),
        WorkspaceError::Io(e) => e.into(),
    }
}

/// `str(exc)` for the GitError or OSError of `repo_root`.
fn os_or_git(e: &WorkspaceError) -> String {
    match e {
        WorkspaceError::Io(e) => match e.raw_os_error() {
            Some(code) => format!("[Errno {code}] {}", crate::cli::strerror(e)),
            None => e.to_string(),
        },
        other => other.to_string(),
    }
}

fn resolve_preset(args: &RunArgs, repo: &Path, host: &dyn Host) -> CliResult<Preset> {
    if let Some(cmd) = args.cmd.as_ref().filter(|c| !c.is_empty()) {
        let Some(lcov) = given(&args.lcov) else {
            return Err(CliError::new("--cmd needs --lcov", EXIT_USAGE).into());
        };
        return Ok(adhoc_preset(
            cmd.clone(),
            lcov.to_owned(),
            args.junit.clone(),
            args.shell,
        ));
    }
    let mut presets = load_all_presets(host, repo)?;
    let name = match given(&args.preset) {
        Some(n) => n.to_owned(),
        None => default_preset_name(repo)?,
    };
    match presets.remove(&name) {
        Some(p) => Ok(p),
        None => {
            let known: Vec<&str> = presets.keys().map(String::as_str).collect();
            Err(CliError::new(
                format!(
                    "unknown preset {} (known: {})",
                    repr_str(&name),
                    known.join(", ")
                ),
                EXIT_USAGE,
            )
            .into())
        }
    }
}

fn default_preset_name(repo: &Path) -> CliResult<String> {
    let python = PYTHON_MARKERS.iter().any(|m| repo.join(m).exists());
    let rust = RUST_MARKERS.iter().any(|m| repo.join(m).exists());
    match (python, rust) {
        (true, false) => Ok("pytest".into()),
        (false, true) => Ok("cargo-llvm-cov".into()),
        _ => Err(CliError::new(
            "can't pick a default preset here; pass --preset or --cmd",
            EXIT_USAGE,
        )
        .into()),
    }
}

/// `--env K=V` items as an ordered map: a repeated key keeps its first position, last value.
fn parse_env(items: &[String]) -> CliResult<Vec<(String, String)>> {
    let mut env: Vec<(String, String)> = Vec::new();
    for item in items {
        let Some((key, value)) = item.split_once('=').filter(|(k, _)| !k.is_empty()) else {
            return Err(CliError::new(
                format!("--env expects K=V, got {}", repr_str(item)),
                EXIT_USAGE,
            )
            .into());
        };
        match env.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.to_owned(),
            None => env.push((key.to_owned(), value.to_owned())),
        }
    }
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sundgren_run::pipeline::wrong_tree_message;

    fn cli_err(r: CliResult<impl std::fmt::Debug>) -> (String, i32) {
        match r {
            Err(Error::Cli(e)) => (e.message, e.code),
            other => panic!("expected CliError, got {other:?}"),
        }
    }

    #[test]
    fn env_parsing_like_a_dict() {
        let items = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            parse_env(&items(&["A=1", "B=x=y", "A=2", "C="])).unwrap(),
            [("A", "2"), ("B", "x=y"), ("C", "")].map(|(k, v)| (k.to_owned(), v.to_owned()))
        );
        assert_eq!(
            cli_err(parse_env(&items(&["=1"]))),
            ("--env expects K=V, got '=1'".into(), EXIT_USAGE)
        );
        assert_eq!(
            cli_err(parse_env(&items(&["NOEQ"]))),
            ("--env expects K=V, got 'NOEQ'".into(), EXIT_USAGE)
        );
    }

    #[test]
    fn default_preset_by_markers() {
        let dir = std::env::temp_dir().join(format!("sundgren-run-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let none = cli_err(default_preset_name(&dir));
        assert_eq!(none.1, EXIT_USAGE);
        std::fs::write(dir.join("pytest.ini"), "").unwrap();
        assert_eq!(default_preset_name(&dir).unwrap(), "pytest");
        std::fs::write(dir.join("Cargo.toml"), "").unwrap();
        assert_eq!(cli_err(default_preset_name(&dir)), none);
        std::fs::remove_file(dir.join("pytest.ini")).unwrap();
        assert_eq!(default_preset_name(&dir).unwrap(), "cargo-llvm-cov");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn wrong_tree_message_truncates_and_adds_hints() {
        let mut preset = adhoc_preset(vec!["x".into()], "l".into(), None, false);
        let paths: Vec<String> = (0..22).map(|i| format!("/repo/f{i}.py")).collect();
        let msg = wrong_tree_message("base", &paths, &preset);
        assert!(msg.starts_with(
            "the base run measured your main checkout instead of its worktree \
             (editable install or stale build?):\n  /repo/f0.py\n"
        ));
        assert!(msg.ends_with("  /repo/f19.py\n  ... and 2 more"));
        preset.hints = vec!["reinstall".into()];
        let msg = wrong_tree_message("patched", &paths[..1], &preset);
        assert!(msg.ends_with("  /repo/f0.py\nhints:\n  - reinstall"));
    }
}
