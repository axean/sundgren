//! The `run` subcommand: worktrees, preset commands on both sides, then diff. Owned by W5.
//!
//! Port of cli.py's `_cmd_run`, `_run_side`, `_wrong_tree_message`, `_resolve_preset`,
//! `_default_preset_name` and `_parse_env`, with the same messages and exit codes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sundgren_core::diff::{Delta, DiffOptions};
use sundgren_core::model::{Outcome, ReportWarning, Snapshot};
use sundgren_core::patch::Patch;
use sundgren_core::pytext::PyText;
use sundgren_core::{hashing, lcov, normalize};
use sundgren_run::guards::wrong_tree_paths;
use sundgren_run::presets::{Preset, TestsStyle};
use sundgren_run::pyfmt::repr_str;
use sundgren_run::pypath::py_str;
use sundgren_run::runner::{RunError, TemplateRunner, adhoc_preset};
use sundgren_run::selectors::{ChooseError, Selection, choose, restrict_to_tree};
use sundgren_run::template::default_python;
use sundgren_run::workspace::{Workspace, WorkspaceError, is_dirty, repo_root};

use crate::cli::{
    CliError, CliResult, EXIT_FAILURE, EXIT_OK, EXIT_PATCH, EXIT_USAGE, Error, Host, Json,
    PYTHON_MARKERS, RUST_MARKERS, ReadError, RunArgs, build_report, diff_snapshots, emit,
    load_all_presets, load_junit, meta, parse_patch_text, patch_meta, read_text, resolve, warning,
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
    let workdir = given(&args.workdir).map(|w| PathBuf::from(resolve(w)));

    // Python wraps the `with Workspace(...)` block in one try: its except clauses apply to
    // is_dirty, the worktree setup and everything run inside.
    let sides = (|| -> Result<Sides, Failure> {
        if is_dirty(&repo)? {
            warnings.push(warning(
                "dirty_tree",
                "working tree has uncommitted changes; the base is the ref",
            ));
        }
        let mut ws = Workspace::new(
            repo.clone(),
            args.base.clone(),
            patch_path.clone(),
            workdir,
            args.keep_worktrees,
        );
        ws.enter()?;
        // `with`: the worktrees go away however the body ends.
        let result = in_workspace(&ws, args, host, &runner, &preset, &repo, &mut warnings);
        ws.exit();
        result
    })();
    let sides = sides.map_err(|f| match f {
        Failure::Workspace(e) => workspace_error(e),
        Failure::Run(RunError::Failure { message, log_tail }) => CliError::new(
            format!("{message}\n--- log tail ---\n{log_tail}"),
            EXIT_FAILURE,
        )
        .into(),
        Failure::Run(RunError::Value(msg)) => Error::Host(msg.into()),
        Failure::Run(RunError::Io(e)) => e.into(),
        Failure::Other(e) => e,
    })?;

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

/// What the workspace block produces.
struct Sides {
    patch: Patch,
    delta: Delta,
    base_ref: String,
    base_sha: Vec<String>,
    patched_sha: Vec<String>,
    patched_selection: Selection,
}

/// Errors raised inside the try block, before the except clauses map them.
enum Failure {
    Workspace(WorkspaceError),
    Run(RunError),
    Other(Error),
}

impl From<WorkspaceError> for Failure {
    fn from(e: WorkspaceError) -> Self {
        Failure::Workspace(e)
    }
}

impl From<RunError> for Failure {
    fn from(e: RunError) -> Self {
        Failure::Run(e)
    }
}

impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Failure::Other(e)
    }
}

impl From<CliError> for Failure {
    fn from(e: CliError) -> Self {
        Failure::Other(e.into())
    }
}

impl From<std::io::Error> for Failure {
    fn from(e: std::io::Error) -> Self {
        Failure::Other(e.into())
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

fn in_workspace(
    ws: &Workspace,
    args: &RunArgs,
    host: &dyn Host,
    runner: &TemplateRunner,
    preset: &Preset,
    repo: &Path,
    warnings: &mut Vec<ReportWarning>,
) -> Result<Sides, Failure> {
    let (base, patched, workdir) = match (&ws.base, &ws.patched, &ws.workdir) {
        (Some(b), Some(p), Some(w)) => (b.as_path(), p.as_path(), w.as_path()),
        _ => unreachable!("Workspace::enter sets base, patched and workdir"),
    };
    // Use what git applied, not the user's hunk headers or path prefixes.
    let applied = PyText::from_surrogateescape_bytes(ws.applied_patch.as_deref().unwrap_or(&[]));
    let (patch, linemaps, patch_warnings) = parse_patch_text(&applied, "applied patch")?;
    warnings.extend(patch_warnings);
    if let Some(paths) = ws.offset_paths.as_ref().filter(|p| !p.is_empty()) {
        warnings.push(warning(
            "patch_offset",
            &format!(
                "patch applied at an offset in {}; line maps use the applied diff",
                paths.join(", ")
            ),
        ));
    }
    let tests = (!args.tests.is_empty()).then_some(args.tests.as_slice());
    let lookup = |id: &str| host.selector(id);
    let selection = choose(tests, preset.selector.as_deref(), &patch, patched, &lookup).map_err(
        |e| match e {
            ChooseError::Unknown(_) => Failure::from(CliError::new(e.to_string(), EXIT_FAILURE)),
            ChooseError::Host(e) => Failure::Other(Error::Host(e)),
        },
    )?;
    let out = workdir.join("out");
    let (base_snaps, base_junit, base_sha) = run_side(
        "base",
        runner,
        preset,
        &selection,
        base,
        &out.join("base"),
        args.repeat,
        repo,
        warnings,
    )?;
    let (patched_snaps, patched_junit, patched_sha) = run_side(
        "patched",
        runner,
        preset,
        &selection,
        patched,
        &out.join("patched"),
        args.repeat,
        repo,
        warnings,
    )?;
    let opts = DiffOptions {
        base_root: Some(base),
        patched_root: Some(patched),
        base_outcomes: base_junit.as_ref(),
        patched_outcomes: patched_junit.as_ref(),
    };
    let delta = diff_snapshots(host, &base_snaps, &patched_snaps, &linemaps, opts)?;
    let patched_selection = restrict_to_tree(&selection, patched).unwrap_or(selection);
    Ok(Sides {
        patch,
        delta,
        base_ref: ws.base_sha.clone().unwrap_or_default(),
        base_sha,
        patched_sha,
        patched_selection,
    })
}

type SideResult = (
    Vec<Snapshot>,
    Option<BTreeMap<String, Outcome>>,
    Vec<String>,
);

/// Run one side; returns its snapshots, first-run outcomes and lcov hashes.
#[allow(clippy::too_many_arguments)]
fn run_side(
    side: &str,
    runner: &TemplateRunner,
    preset: &Preset,
    selection: &Selection,
    worktree: &Path,
    out: &Path,
    repeat: i64,
    repo: &Path,
    warnings: &mut Vec<ReportWarning>,
) -> Result<SideResult, Failure> {
    let side_selection = if preset.tests_style == TestsStyle::Args {
        restrict_to_tree(selection, worktree)
    } else {
        Some(selection.clone())
    };
    let Some(side_selection) = side_selection else {
        // Every selected test is new in the patch: nothing ran on this side.
        warnings.push(warning(
            "nothing_to_run",
            &format!(
                "no selected tests exist on the {side} side; its coverage is treated as empty"
            ),
        ));
        return Ok((vec![Snapshot::default()], Some(BTreeMap::new()), Vec::new()));
    };

    let runs = runner.run(worktree, &side_selection, out, repeat)?;
    let mut snapshots = Vec::new();
    let mut hashes = Vec::new();
    let roots = [py_str(worktree)];
    for art in &runs {
        let raw = match read_text(&art.lcov) {
            Ok(text) => lcov::parse(&text).map_err(|e| e.to_string()),
            Err(ReadError::Decode(msg)) => Err(msg),
            Err(ReadError::Os(e)) => return Err(e.into()),
        };
        let raw = raw.map_err(|msg| {
            CliError::new(format!("{side}: bad lcov output: {msg}"), EXIT_FAILURE)
        })?;
        let wrong = wrong_tree_paths(raw.files.values().map(|fc| &fc.path), worktree, repo);
        if !wrong.is_empty() {
            return Err(
                CliError::new(wrong_tree_message(side, &wrong, preset), EXIT_FAILURE).into(),
            );
        }
        snapshots.push(normalize::normalize_snapshot(&raw, &roots));
        hashes.push(hashing::sha256_file(&art.lcov)?);
    }
    let junit = load_junit(runs[0].junit.as_deref(), warnings)?;
    Ok((snapshots, junit, hashes))
}

fn wrong_tree_message(side: &str, paths: &[String], preset: &Preset) -> String {
    let mut lines = vec![format!(
        "the {side} run measured your main checkout instead of its worktree \
         (editable install or stale build?):"
    )];
    lines.extend(paths.iter().take(20).map(|p| format!("  {p}")));
    if paths.len() > 20 {
        lines.push(format!("  ... and {} more", paths.len() - 20));
    }
    if !preset.hints.is_empty() {
        lines.push("hints:".into());
        lines.extend(preset.hints.iter().map(|h| format!("  - {h}")));
    }
    lines.join("\n")
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
