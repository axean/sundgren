//! TemplateRunner: run a preset's commands in a worktree, N repeats.

use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use crate::presets::{Preset, TestsStyle};
use crate::pyfmt::repr_str;
use crate::pypath::{self, py_str};
use crate::selectors::Selection;
use crate::shlex::{self, ShlexError};
use crate::template::{Command, Values, expand_command, expand_value};

pub const LOG_TAIL_LINES: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunArtifacts {
    pub lcov: PathBuf,
    pub junit: Option<PathBuf>,
    /// First non-zero return code, else 0; test failures are data.
    pub returncode: i32,
    pub log: PathBuf,
}

#[derive(Debug)]
pub enum RunError {
    /// lcov output missing after the commands ran.
    Failure {
        message: String,
        log_tail: String,
    },
    /// Bad arguments or a template that doesn't split (Python: ValueError).
    Value(String),
    Io(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Failure { message, .. } | RunError::Value(message) => f.write_str(message),
            RunError::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RunError {}

impl From<io::Error> for RunError {
    fn from(e: io::Error) -> Self {
        RunError::Io(e)
    }
}

impl From<ShlexError> for RunError {
    fn from(e: ShlexError) -> Self {
        RunError::Value(e.to_string())
    }
}

pub fn adhoc_preset(
    commands: Vec<String>,
    lcov: String,
    junit: Option<String>,
    shell: bool,
) -> Preset {
    Preset {
        name: "adhoc".into(),
        description: "--cmd".into(),
        commands,
        env: Vec::new(),
        lcov,
        junit,
        selector: None,
        tests_style: TestsStyle::Args,
        shell,
        hints: Vec::new(),
        source: "--cmd".into(),
    }
}

#[derive(Debug, Clone)]
pub struct TemplateRunner {
    pub preset: Preset,
    pub python: String,
    pub env_overrides: Vec<(String, String)>,
}

impl TemplateRunner {
    pub fn new(preset: Preset, python: String, env_overrides: Vec<(String, String)>) -> Self {
        TemplateRunner {
            preset,
            python,
            env_overrides,
        }
    }

    /// Commands with only {python} and {tests} expanded (deterministic, for meta).
    pub fn display_commands(&self, selection: &Selection) -> Result<Vec<String>, ShlexError> {
        let values: Values = [("python".to_string(), self.python.clone())].into();
        let tests = tests_of(selection);
        self.preset
            .commands
            .iter()
            .map(|c| {
                Ok(
                    match expand_command(c, &values, tests, self.preset.shell)? {
                        Command::Shell(s) => s,
                        Command::Argv(argv) => join_plain(&argv),
                    },
                )
            })
            .collect()
    }

    pub fn run(
        &self,
        worktree: &Path,
        selection: &Selection,
        out_dir: &Path,
        repeat: i64,
    ) -> Result<Vec<RunArtifacts>, RunError> {
        if repeat < 1 {
            return Err(RunError::Value(format!(
                "repeat must be >= 1, got {repeat}"
            )));
        }
        let worktree = pypath::absolute(worktree)?;
        let out_dir = pypath::absolute(out_dir)?;
        (1..=repeat)
            .map(|i| self.run_once(&worktree, selection, &out_dir.join(format!("run-{i}"))))
            .collect()
    }

    fn run_once(
        &self,
        worktree: &Path,
        selection: &Selection,
        run_dir: &Path,
    ) -> Result<RunArtifacts, RunError> {
        std::fs::create_dir_all(run_dir)?;
        let tests = tests_of(selection);
        let mut values: Values = [
            ("python".to_string(), self.python.clone()),
            ("out".to_string(), py_str(run_dir)),
            ("worktree".to_string(), py_str(worktree)),
        ]
        .into();
        let lcov_rel = expand_value(&self.preset.lcov, &values);
        values.insert("lcov".into(), lcov_rel.clone());
        let junit_rel = match &self.preset.junit {
            Some(t) if !t.is_empty() => expand_value(t, &values),
            _ => String::new(),
        };
        values.insert("junit".into(), junit_rel.clone());
        values.insert("tests".into(), tests.join(" "));
        let mut env: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os().collect();
        for (name, template) in self.preset.env.iter().chain(&self.env_overrides) {
            let value = expand_value(template, &values);
            env.retain(|(k, _)| k.as_os_str() != name.as_str());
            env.push((name.into(), value.into()));
        }

        // Relative output paths are relative to the commands' cwd.
        let lcov = pypath::normalize(&worktree.join(&lcov_rel));
        let junit = (!junit_rel.is_empty()).then(|| pypath::normalize(&worktree.join(&junit_rel)));
        // Left over from an earlier repeat or a reused --workdir.
        for stale in [Some(&lcov), junit.as_ref()].into_iter().flatten() {
            if stale.is_file() {
                std::fs::remove_file(stale)?;
            }
        }

        let log = run_dir.join("log.txt");
        let mut returncode = 0;
        {
            let mut fh = File::create(&log)?;
            for command in &self.preset.commands {
                let argv = expand_command(command, &values, tests, self.preset.shell)?;
                let shown = match &argv {
                    Command::Shell(s) => s.clone(),
                    Command::Argv(a) => shlex::join(a),
                };
                fh.write_all(format!("$ {shown}\n").as_bytes())?;
                fh.flush()?;
                let code = call(&argv, worktree, &env, &fh)?;
                fh.write_all(format!("[exit {code}]\n").as_bytes())?;
                fh.flush()?;
                if returncode == 0 {
                    returncode = code;
                }
            }
        }

        if !lcov.is_file() {
            let name = run_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return Err(RunError::Failure {
                message: format!("{name}: no lcov output at {}", py_str(&lcov)),
                log_tail: tail(&log)?,
            });
        }
        let lcov = keep_in(&lcov, run_dir)?;
        let junit = match junit {
            Some(j) if j.is_file() => Some(keep_in(&j, run_dir)?),
            _ => None,
        };
        Ok(RunArtifacts {
            lcov,
            junit,
            returncode,
            log,
        })
    }
}

fn tests_of(selection: &Selection) -> &[String] {
    if selection.run_all {
        &[]
    } else {
        &selection.tests
    }
}

/// `[\w@%+=:,./{}-]+` with Python's Unicode `\w`.
fn is_plain(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || "@%+=:,./{}-".contains(c))
}

fn join_plain(argv: &[String]) -> String {
    argv.iter()
        .map(|t| {
            if is_plain(t) {
                t.clone()
            } else {
                shlex::quote(t)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run one command with output into `fh`; a command that can't start is logged as 126/127.
fn call(
    argv: &Command,
    cwd: &Path,
    env: &[(std::ffi::OsString, std::ffi::OsString)],
    fh: &File,
) -> io::Result<i32> {
    let (mut proc, program) = match argv {
        Command::Shell(s) => {
            let mut p = Process::new("/bin/sh");
            p.arg("-c").arg(s);
            (p, "/bin/sh".to_string())
        }
        Command::Argv(a) if a.is_empty() => {
            (&*fh).write_all(b"sundgren: empty command\n")?;
            return Ok(127);
        }
        Command::Argv(a) => {
            let mut p = Process::new(&a[0]);
            p.args(&a[1..]);
            (p, a[0].clone())
        }
    };
    proc.current_dir(cwd)
        .env_clear()
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
            let msg = format!(
                "sundgren: cannot run command: {}\n",
                os_error_str(&e, &program)
            );
            (&*fh).write_all(msg.as_bytes())?;
            Ok(if e.kind() == io::ErrorKind::NotFound {
                127
            } else {
                126
            })
        }
    }
}

/// Python's OSError str: `[Errno 2] No such file or directory: 'prog'`.
fn os_error_str(e: &io::Error, filename: &str) -> String {
    match e.raw_os_error() {
        Some(code) => {
            let text = io::Error::from_raw_os_error(code).to_string();
            let strerror = text
                .strip_suffix(&format!(" (os error {code})"))
                .unwrap_or(&text);
            format!("[Errno {code}] {strerror}: {}", repr_str(filename))
        }
        None => e.to_string(),
    }
}

/// Copy an output written outside the run dir into it, so repeats don't overwrite it.
fn keep_in(path: &Path, run_dir: &Path) -> io::Result<PathBuf> {
    if path.starts_with(run_dir) {
        return Ok(path.to_path_buf());
    }
    let target = run_dir.join(path.file_name().unwrap_or_default());
    std::fs::copy(path, &target)?;
    Ok(target)
}

fn tail(log: &Path) -> io::Result<String> {
    let bytes = std::fs::read(log)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines = splitlines(&text);
    let start = lines.len().saturating_sub(LOG_TAIL_LINES);
    Ok(lines[start..].join("\n"))
}

/// `str.splitlines()`.
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let is_break = matches!(
            c,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if is_break {
            out.push(&text[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|&(_, n)| n == '\n') {
                chars.next();
                end += 1;
            }
            start = end;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitlines_like_python() {
        assert_eq!(
            splitlines("a\nb\r\nc\rd\x0ce\u{2028}f"),
            ["a", "b", "c", "d", "e", "f"]
        );
        assert_eq!(splitlines("a\n\nb\n"), ["a", "", "b"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn display_join_quotes_only_odd_tokens() {
        let argv: Vec<String> = ["py", "t/x y.py", "--junitxml={junit}", "é", ""]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(join_plain(&argv), "py 't/x y.py' --junitxml={junit} é ''");
    }

    #[test]
    fn os_error_like_python() {
        let e = io::Error::from_raw_os_error(2);
        assert_eq!(
            os_error_str(&e, "nope"),
            "[Errno 2] No such file or directory: 'nope'"
        );
    }
}
