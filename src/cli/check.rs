//! `run --check`: everything up to running the tests, reported as `sundgren.check/1` JSON.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{RunArgs, given, parse_env, resolve, resolve_preset};
use crate::patch::parse_patch;
use crate::run::presets::Preset;
use crate::run::{RunConfig, Workspace, default_python, is_dirty, repo_root, runner, select};
use crate::{EXIT_FAILURE, EXIT_OK, EXIT_PATCH, EXIT_USAGE, Error, Result, cancel};

/// One thing that would make the run fail or the comparison meaningless.
struct Problem {
    code: &'static str,
    message: String,
    /// What `run` would exit with.
    exit: i32,
}

#[derive(Default)]
struct Problems(Vec<Problem>);

impl Problems {
    fn add(&mut self, code: &'static str, exit: i32, message: impl Into<String>) {
        let exit = match exit {
            EXIT_USAGE | EXIT_PATCH => exit,
            _ => EXIT_FAILURE,
        };
        self.0.push(Problem {
            code,
            message: message.into(),
            exit,
        });
    }

    fn usage(&mut self, code: &'static str, message: impl Into<String>) {
        self.add(code, EXIT_USAGE, message);
    }

    /// The first problem with the exit code.
    fn decisive(&self) -> Option<&Problem> {
        let code = self.exit_code();
        self.0.iter().find(|p| p.exit == code)
    }

    /// Usage beats a patch that doesn't apply, which beats anything else.
    fn exit_code(&self) -> i32 {
        let has = |code| self.0.iter().any(|p| p.exit == code);
        if self.0.is_empty() {
            EXIT_OK
        } else if has(EXIT_USAGE) {
            EXIT_USAGE
        } else if has(EXIT_PATCH) {
            EXIT_PATCH
        } else {
            EXIT_FAILURE
        }
    }
}

/// Print the check report; an Err carries the exit code when there are problems.
pub fn cmd_check(args: &RunArgs) -> Result<()> {
    let report = check(args)?;
    let text = crate::report::dumps(&report.json);
    match &args.output.output {
        Some(path) => std::fs::write(path, text)?,
        None => std::io::Write::write_all(&mut std::io::stdout(), text.as_bytes())?,
    }
    match report.first {
        None => Ok(()),
        Some(msg) => Err(Error {
            code: report.exit,
            msg: format!("check failed: {msg}"),
        }),
    }
}

struct Report {
    json: Value,
    exit: i32,
    /// The message for stderr.
    first: Option<String>,
}

fn check(args: &RunArgs) -> Result<Report> {
    let mut problems = Problems::default();
    if args.repeat < 1 {
        problems.usage("usage", "--repeat must be at least 1");
    }
    if args.cmd.is_empty()
        && (given(&args.lcov).is_some() || given(&args.junit).is_some() || args.shell)
    {
        problems.usage("usage", "--lcov, --junit and --shell only apply with --cmd");
    }
    let lcov_missing = !args.cmd.is_empty() && given(&args.lcov).is_none();
    if lcov_missing {
        problems.usage("usage", "--cmd needs --lcov");
    }
    let env = parse_env(&args.env).unwrap_or_else(|e| {
        problems.usage("usage", e.msg);
        Vec::new()
    });
    // No patch: the patched tree is a second checkout of the base.
    let patch_path = given(&args.patch).map(|p| PathBuf::from(resolve(p)));
    let patch_ok = patch_path.as_ref().is_none_or(|p| p.is_file());
    if let (false, Some(arg)) = (patch_ok, &args.patch) {
        problems.usage("patch_not_found", format!("patch not found: {arg}"));
    }
    let repo = match repo_root(Path::new(&args.repo)) {
        Ok(repo) => Some(repo),
        Err(_) => {
            problems.usage(
                "not_a_repo",
                format!("{} is not inside a git repository", args.repo),
            );
            None
        }
    };
    let preset = match &repo {
        Some(repo) if !lcov_missing => resolve_preset(args, repo)
            .map_err(|e| problems.usage("preset", e.msg))
            .ok(),
        _ => None,
    };
    let python = given(&args.python).map_or_else(default_python, str::to_string);
    let mut base_ref = Value::Null;
    let mut selection = Value::Null;
    let mut commands = Value::Null;
    if let Some(repo) = &repo {
        match is_dirty(repo) {
            Ok(false) => {}
            Ok(true) => problems.add(
                "dirty_tree",
                EXIT_FAILURE,
                "the working tree has uncommitted changes to tracked files: commit or stash them first",
            ),
            Err(e) => problems.add("error", e.code, e.msg),
        }
        if patch_ok {
            match Workspace::patched_only(repo, &args.base, patch_path.as_deref()) {
                Ok(ws) => {
                    base_ref = json!(ws.base_sha);
                    if let Some(preset) = &preset {
                        let cfg = config(args, repo, patch_path.as_deref(), preset, &python, &env);
                        match plan(&cfg, &ws) {
                            Ok((s, c)) => (selection, commands) = (s, c),
                            Err(e) => problems.add("error", e.code, e.msg),
                        }
                    }
                }
                Err(e) => {
                    cancel::check()?;
                    match e.code {
                        EXIT_USAGE => problems.usage(
                            "bad_base",
                            format!(
                                "base ref '{}' is not a commit in this repository",
                                args.base
                            ),
                        ),
                        EXIT_PATCH => {
                            problems.add("patch_does_not_apply", EXIT_PATCH, patch_message(&e.msg))
                        }
                        code => problems.add("error", code, e.msg),
                    }
                }
            }
        }
        if let Some(preset) = &preset {
            let out = tempfile::Builder::new().prefix("sundgren-").tempdir()?;
            match runner::run_checks(preset, &python, &env, repo, out.path()) {
                Ok(failures) => {
                    for msg in failures {
                        problems.add("check_failed", EXIT_FAILURE, msg);
                    }
                }
                Err(e) => problems.add("error", e.code, e.msg),
            }
        }
    }
    cancel::check()?;
    let json = json!({
        "schema": "sundgren.check/1",
        "ok": problems.0.is_empty(),
        "repo": repo.as_ref().map(|r| r.display().to_string()),
        "base_ref": base_ref,
        "preset": preset.as_ref().map(|p| p.name.clone()),
        "python": preset.as_ref().filter(|p| uses_python(p)).map(|_| python.clone()),
        "commands": commands,
        "selection": selection,
        "problems": problems.0.iter().map(|p| json!({"code": p.code, "message": p.message})).collect::<Vec<_>>(),
    });
    Ok(Report {
        json,
        exit: problems.exit_code(),
        first: problems.decisive().map(|p| p.message.clone()),
    })
}

fn config(
    args: &RunArgs,
    repo: &Path,
    patch_path: Option<&Path>,
    preset: &Preset,
    python: &str,
    env: &[(String, String)],
) -> RunConfig {
    RunConfig {
        repo: repo.to_path_buf(),
        base: args.base.clone(),
        patch_path: patch_path.map(Path::to_path_buf),
        workdir: None,
        keep_worktrees: false,
        preset: preset.clone(),
        python: python.to_owned(),
        env: env.to_vec(),
        tests: args.tests.clone(),
        all_tests: args.all_tests,
        repeat: args.repeat,
    }
}

/// The selection and commands `run` would use, from what git applied.
fn plan(cfg: &RunConfig, ws: &Workspace) -> Result<(Value, Value)> {
    let patch = parse_patch(&ws.applied_patch)
        .map_err(|e| Error::failure(format!("applied patch: {e}")))?;
    let selection = select(cfg, &patch, &ws.patched)?;
    let selection =
        crate::run::selectors::restrict_to_tree(&selection, &ws.patched).unwrap_or(selection);
    let commands = runner::display_commands(&cfg.preset, &cfg.python, &selection)?;
    Ok((
        json!({
            "tests": selection.tests,
            "run_all": selection.run_all,
            "reason": selection.reason,
        }),
        json!(commands),
    ))
}

/// git's reasons on one line.
fn patch_message(msg: &str) -> String {
    let reasons: Vec<&str> = msg.lines().skip(1).map(str::trim).collect();
    match reasons.is_empty() {
        true => "the patch does not apply to the base".into(),
        false => format!(
            "the patch does not apply to the base: {}",
            reasons.join("; ")
        ),
    }
}

fn uses_python(preset: &Preset) -> bool {
    let templates = preset
        .commands
        .iter()
        .chain(&preset.checks)
        .chain(preset.env.iter().map(|(_, v)| v));
    templates.into_iter().any(|t| t.contains("{python}"))
}

#[cfg(test)]
mod tests;
