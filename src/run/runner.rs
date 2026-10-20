//! Run a preset's commands in a worktree: placeholder expansion, N repeats, output files.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use super::presets::Preset;
use super::selectors::Selection;
use crate::{Error, Result};

const PLACEHOLDERS: [&str; 6] = ["python", "out", "worktree", "lcov", "junit", "tests"];
const LOG_TAIL_LINES: usize = 40;

/// Placeholder name to value.
type Values = BTreeMap<&'static str, String>;

/// An expanded command: an argv list, or a `/bin/sh -c` string in shell mode.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Argv(Vec<String>),
    Shell(String),
}

/// Outputs of one run.
#[derive(Debug, Clone)]
pub struct Run {
    pub lcov: PathBuf,
    pub junit: Option<PathBuf>,
}

/// Substitute known placeholders in one pass; unknown braces stay.
fn expand_value(template: &str, values: &Values) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let hit = PLACEHOLDERS
            .iter()
            .find(|name| after.starts_with(*name) && after[name.len()..].starts_with('}'));
        match hit.and_then(|name| values.get(name).map(|v| (name, v))) {
            Some((name, value)) => {
                out.push_str(value);
                rest = &after[name.len() + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// argv (a `{tests}` token becomes one entry per test), or a shell string with quoted tests.
fn expand_command(
    command: &str,
    values: &Values,
    tests: &[String],
    shell: bool,
) -> Result<Command> {
    let mut values = values.clone();
    if shell {
        let quoted: Vec<String> = tests.iter().map(|t| quote(t)).collect();
        values.insert("tests", quoted.join(" "));
        return Ok(Command::Shell(expand_value(command, &values)));
    }
    values.insert("tests", tests.join(" "));
    let tokens = shlex::split(command)
        .ok_or_else(|| Error::failure(format!("cannot split command: {command}")))?;
    let mut argv = Vec::new();
    for token in tokens {
        if token == "{tests}" {
            argv.extend(tests.iter().cloned());
        } else {
            argv.push(expand_value(&token, &values));
        }
    }
    Ok(Command::Argv(argv))
}

fn quote(s: &str) -> String {
    shlex::try_quote(s).map_or_else(|_| s.to_owned(), |q| q.into_owned())
}

/// The command as logged and reported; argv tokens outside `[\w@%+=:,./{}-]` are quoted.
fn shown(command: &Command) -> String {
    let plain = |t: &str| {
        !t.is_empty()
            && t.chars()
                .all(|c| c.is_alphanumeric() || "_@%+=:,./{}-".contains(c))
    };
    match command {
        Command::Shell(s) => s.clone(),
        Command::Argv(argv) => argv
            .iter()
            .map(|t| if plain(t) { t.clone() } else { quote(t) })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn tests_of(selection: &Selection) -> &[String] {
    if selection.run_all {
        &[]
    } else {
        &selection.tests
    }
}

/// The commands with only {python} and {tests} expanded, for the report's meta.
pub fn display_commands(
    preset: &Preset,
    python: &str,
    selection: &Selection,
) -> Result<Vec<String>> {
    let values: Values = [("python", python.to_owned())].into();
    preset
        .commands
        .iter()
        .map(|c| {
            Ok(shown(&expand_command(
                c,
                &values,
                tests_of(selection),
                preset.shell,
            )?))
        })
        .collect()
}

/// Run the preset `repeat` times in `worktree`, outputs under `out/run-N`.
pub fn run(
    preset: &Preset,
    python: &str,
    env_overrides: &[(String, String)],
    worktree: &Path,
    selection: &Selection,
    out: &Path,
    repeat: i64,
) -> Result<Vec<Run>> {
    (1..=repeat)
        .map(|i| {
            let run_dir = out.join(format!("run-{i}"));
            run_once(preset, python, env_overrides, worktree, selection, &run_dir)
        })
        .collect()
}

fn run_once(
    preset: &Preset,
    python: &str,
    env_overrides: &[(String, String)],
    worktree: &Path,
    selection: &Selection,
    run_dir: &Path,
) -> Result<Run> {
    std::fs::create_dir_all(run_dir)?;
    let tests = tests_of(selection);
    let mut values: Values = [
        ("python", python.to_owned()),
        ("out", run_dir.display().to_string()),
        ("worktree", worktree.display().to_string()),
    ]
    .into();
    let lcov_rel = expand_value(&preset.lcov, &values);
    values.insert("lcov", lcov_rel.clone());
    let junit_rel = match &preset.junit {
        Some(t) if !t.is_empty() => expand_value(t, &values),
        _ => String::new(),
    };
    values.insert("junit", junit_rel.clone());
    values.insert("tests", tests.join(" "));
    let env: Vec<(String, String)> = preset
        .env
        .iter()
        .chain(env_overrides)
        .map(|(name, template)| (name.clone(), expand_value(template, &values)))
        .collect();

    // Relative output paths are relative to the commands' cwd.
    let lcov = normpath(&worktree.join(&lcov_rel));
    let junit = (!junit_rel.is_empty()).then(|| normpath(&worktree.join(&junit_rel)));
    // Left over from an earlier repeat or a reused --workdir.
    for stale in [Some(&lcov), junit.as_ref()].into_iter().flatten() {
        if stale.is_file() {
            std::fs::remove_file(stale)?;
        }
    }
    let log = run_dir.join("log.txt");
    let mut fh = File::create(&log)?;
    for command in &preset.commands {
        let argv = expand_command(command, &values, tests, preset.shell)?;
        writeln!(fh, "$ {}", shown(&argv))?;
        let code = call(&argv, worktree, &env, &fh)?;
        writeln!(fh, "[exit {code}]")?;
    }
    drop(fh);
    if !lcov.is_file() {
        let name = run_dir.file_name().unwrap_or_default().to_string_lossy();
        let text = String::from_utf8_lossy(&std::fs::read(&log)?).into_owned();
        let lines: Vec<&str> = text.lines().collect();
        let tail = lines[lines.len().saturating_sub(LOG_TAIL_LINES)..].join("\n");
        return Err(Error::failure(format!(
            "{name}: no lcov output at {}\n--- log tail ---\n{tail}",
            lcov.display()
        )));
    }
    let lcov = keep_in(&lcov, run_dir)?;
    let junit = match junit {
        Some(j) if j.is_file() => Some(keep_in(&j, run_dir)?),
        _ => None,
    };
    Ok(Run { lcov, junit })
}

/// Run one command with its output into the log; a command that can't start is logged as 126/127.
/// Test failures are data, so the exit code is only logged.
fn call(argv: &Command, cwd: &Path, env: &[(String, String)], fh: &File) -> Result<i32> {
    let mut proc = match argv {
        Command::Shell(s) => {
            let mut p = Process::new("/bin/sh");
            p.arg("-c").arg(s);
            p
        }
        Command::Argv(a) if a.is_empty() => {
            writeln!(&*fh, "sundgren: empty command")?;
            return Ok(127);
        }
        Command::Argv(a) => {
            let mut p = Process::new(&a[0]);
            p.args(&a[1..]);
            p
        }
    };
    proc.current_dir(cwd)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(fh.try_clone()?)
        .stderr(fh.try_clone()?);
    match crate::cancel::spawn(&mut proc) {
        Ok(mut child) => {
            let status = crate::cancel::wait(&mut child)?;
            use std::os::unix::process::ExitStatusExt;
            Ok(status
                .code()
                .unwrap_or_else(|| -status.signal().unwrap_or(0)))
        }
        Err(e) => {
            writeln!(&*fh, "sundgren: cannot run command: {e}")?;
            Ok(if e.kind() == std::io::ErrorKind::NotFound {
                127
            } else {
                126
            })
        }
    }
}

/// Run the preset's checks in `cwd`; one plain sentence per failed check.
pub fn run_checks(
    preset: &Preset,
    python: &str,
    env_overrides: &[(String, String)],
    cwd: &Path,
    out: &Path,
) -> Result<Vec<String>> {
    let (values, env) = check_env(preset, python, env_overrides, cwd, out);
    let mut failures = Vec::new();
    for check in &preset.checks {
        let command = expand_command(check, &values, &[], preset.shell)?;
        let mut proc = match &command {
            Command::Shell(s) => {
                let mut p = Process::new("/bin/sh");
                p.arg("-c").arg(s);
                p
            }
            Command::Argv(a) if a.is_empty() => {
                failures.push(format!("preset {} has an empty check", preset.name));
                continue;
            }
            Command::Argv(a) => {
                let mut p = Process::new(&a[0]);
                p.args(&a[1..]);
                p
            }
        };
        proc.current_dir(cwd).envs(env.iter().map(|(k, v)| (k, v)));
        let result = crate::cancel::output(&mut proc);
        crate::cancel::check()?;
        if let Some(msg) = check_failure(&preset.name, python, &command, result) {
            failures.push(msg);
        }
    }
    Ok(failures)
}

/// Placeholder values and the expanded env for a command that isn't a test run.
fn check_env(
    preset: &Preset,
    python: &str,
    env_overrides: &[(String, String)],
    cwd: &Path,
    out: &Path,
) -> (Values, Vec<(String, String)>) {
    let values: Values = [
        ("python", python.to_owned()),
        ("out", out.display().to_string()),
        ("worktree", cwd.display().to_string()),
    ]
    .into();
    let env = preset
        .env
        .iter()
        .chain(env_overrides)
        .map(|(name, template)| (name.clone(), expand_value(template, &values)))
        .collect();
    (values, env)
}

/// The env the commands get in `worktree`, for a probe there.
pub fn probe_env(
    preset: &Preset,
    python: &str,
    env_overrides: &[(String, String)],
    worktree: &Path,
    out: &Path,
) -> Vec<(String, String)> {
    check_env(preset, python, env_overrides, worktree, out).1
}

/// What to tell the user about a check that didn't exit 0, or None if it did.
fn check_failure(
    preset: &str,
    python: &str,
    command: &Command,
    result: std::io::Result<std::process::Output>,
) -> Option<String> {
    let shown = shown(command);
    let out = match result {
        Ok(out) if out.status.success() => return None,
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let program = match command {
                Command::Argv(a) => a[0].clone(),
                Command::Shell(_) => "/bin/sh".into(),
            };
            return Some(format!(
                "{program} was not found, but the {preset} preset needs it: install it or add it to PATH"
            ));
        }
        Err(e) => return Some(format!("could not run `{shown}`: {e}")),
    };
    let text = String::from_utf8_lossy(&[out.stdout, out.stderr].concat()).into_owned();
    let module = regex::Regex::new(r"No module named '([\w.]+)'").expect("valid regex");
    if let Some(m) = module.captures(&text) {
        return Some(format!(
            "{} isn't installed for {python}: install it into that environment or use another interpreter",
            &m[1]
        ));
    }
    let code = out
        .status
        .code()
        .map_or_else(|| "a signal".to_owned(), |c| format!("exit {c}"));
    let last = text.lines().rev().map(str::trim).find(|l| !l.is_empty());
    Some(match last {
        Some(line) => format!(
            "the {preset} preset's check `{shown}` failed ({code}): {}",
            clip(line)
        ),
        None => format!("the {preset} preset's check `{shown}` failed ({code})"),
    })
}

fn clip(line: &str) -> String {
    const MAX: usize = 200;
    match line.char_indices().nth(MAX) {
        Some((i, _)) => format!("{}...", &line[..i]),
        None => line.to_owned(),
    }
}

/// Copy an output written outside the run dir into it, so repeats don't overwrite it.
fn keep_in(path: &Path, run_dir: &Path) -> Result<PathBuf> {
    if path.starts_with(run_dir) {
        return Ok(path.to_path_buf());
    }
    let target = run_dir.join(path.file_name().unwrap_or_default());
    std::fs::copy(path, &target)?;
    Ok(target)
}

/// Lexical `..` and `.` removal, like `os.path.normpath`.
pub fn normpath(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() && !out.has_root() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests;
