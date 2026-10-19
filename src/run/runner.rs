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
    match proc.status() {
        Ok(status) => {
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
mod tests {
    use super::*;
    use crate::run::presets::adhoc_preset;

    fn values() -> Values {
        [
            ("python", "/opt/py bin/python"),
            ("out", "/tmp/out"),
            ("worktree", "/wt"),
            ("lcov", "/tmp/l"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_owned()))
        .collect()
    }

    fn argv(cmd: &str, tests: &[&str]) -> Vec<String> {
        let tests: Vec<String> = tests.iter().map(|t| t.to_string()).collect();
        match expand_command(cmd, &values(), &tests, false).unwrap() {
            Command::Argv(a) => a,
            Command::Shell(_) => unreachable!(),
        }
    }

    fn sel(tests: &[&str]) -> Selection {
        Selection {
            tests: tests.iter().map(|t| t.to_string()).collect(),
            run_all: tests.is_empty(),
            reason: String::new(),
        }
    }

    #[test]
    fn expand_value_cases() {
        let v = values();
        assert_eq!(expand_value("{out}/x:{worktree}", &v), "/tmp/out/x:/wt");
        assert_eq!(
            expand_value("${HOME}/{foo}/{out}", &v),
            "${HOME}/{foo}//tmp/out"
        );
        assert_eq!(expand_value("{junit}", &v), "{junit}");
        assert_eq!(expand_value("{{out}}{", &v), "{/tmp/out}{");
        // One pass: an expanded value is not expanded again.
        let tricky: Values = [("python", "{out}".into()), ("out", "x".into())].into();
        assert_eq!(expand_value("{python}", &tricky), "{out}");
    }

    #[test]
    fn expand_command_cases() {
        assert_eq!(argv("pytest {tests}", &[]), ["pytest"]);
        assert_eq!(
            argv("pytest {tests} -q", &["a.py", "b c.py"]),
            ["pytest", "a.py", "b c.py", "-q"]
        );
        assert_eq!(
            argv("run --only={tests}", &["a", "b"]),
            ["run", "--only=a b"]
        );
        assert_eq!(
            argv("{python} -m x --o={out}/f", &[]),
            ["/opt/py bin/python", "-m", "x", "--o=/tmp/out/f"]
        );
        let tests = vec!["a b.py".to_string(), "c.py".to_string()];
        assert_eq!(
            expand_command("pytest {tests} > {out}/log", &values(), &tests, true).unwrap(),
            Command::Shell("pytest 'a b.py' c.py > /tmp/out/log".into())
        );
        assert!(expand_command("x 'unclosed", &values(), &[], false).is_err());
    }

    #[test]
    fn display_commands_expand_only_python_and_tests() {
        let preset = adhoc_preset(
            vec!["{python} -m pytest {tests} --o={out}".into()],
            "l".into(),
            None,
            false,
        );
        assert_eq!(
            display_commands(&preset, "py", &sel(&["t/a b.py"])).unwrap(),
            ["py -m pytest 't/a b.py' --o={out}"]
        );
        assert_eq!(
            display_commands(&preset, "py", &sel(&[])).unwrap(),
            ["py -m pytest --o={out}"]
        );
    }

    #[test]
    fn run_writes_outputs_per_repeat_and_logs() {
        let wt = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let mut preset = adhoc_preset(
            vec![
                "sh -c 'echo SF:x > {lcov}; echo $V > {out}/v; exit 3'".into(),
                "no-such-cmd-xyz".into(),
            ],
            "{out}/l.info".into(),
            None,
            false,
        );
        preset.env = vec![("V".into(), "{worktree}".into())];
        let runs = run(&preset, "py", &[], wt.path(), &sel(&[]), out.path(), 2).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].lcov, out.path().join("run-2/l.info"));
        let v = std::fs::read_to_string(out.path().join("run-1/v")).unwrap();
        assert_eq!(v.trim(), wt.path().display().to_string());
        let log = std::fs::read_to_string(out.path().join("run-1/log.txt")).unwrap();
        assert!(log.contains("[exit 3]\n"), "{log}");
        assert!(log.contains("[exit 127]\n"), "{log}");
    }

    #[test]
    fn missing_lcov_fails_with_log_tail() {
        let wt = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let preset = adhoc_preset(
            vec!["echo hello".into()],
            "missing.info".into(),
            None,
            false,
        );
        let e = run(&preset, "py", &[], wt.path(), &sel(&[]), out.path(), 1).unwrap_err();
        assert_eq!(e.code, crate::EXIT_FAILURE);
        assert!(e.msg.starts_with("run-1: no lcov output at "), "{}", e.msg);
        assert!(
            e.msg
                .ends_with("--- log tail ---\n$ echo hello\nhello\n[exit 0]"),
            "{}",
            e.msg
        );
    }

    #[test]
    fn outputs_outside_the_run_dir_are_copied_in() {
        let wt = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let preset = adhoc_preset(
            vec!["sh -c 'echo x > cov.info; echo j > j.xml'".into()],
            "cov.info".into(),
            Some("j.xml".into()),
            false,
        );
        let runs = run(&preset, "py", &[], wt.path(), &sel(&[]), out.path(), 1).unwrap();
        assert_eq!(runs[0].lcov, out.path().join("run-1/cov.info"));
        assert_eq!(
            runs[0].junit.as_deref(),
            Some(out.path().join("run-1/j.xml").as_path())
        );
    }

    #[test]
    fn normpath_is_lexical() {
        assert_eq!(normpath(Path::new("/a/b/../c/./d")), Path::new("/a/c/d"));
        assert_eq!(normpath(Path::new("/a/../../b")), Path::new("/b"));
    }
}
