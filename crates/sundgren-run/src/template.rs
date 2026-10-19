//! Command templates: placeholder expansion, tokenwise {tests}.

use std::collections::BTreeMap;

use crate::shlex::{self, ShlexError};

pub const PLACEHOLDERS: [&str; 6] = ["python", "out", "worktree", "lcov", "junit", "tests"];

/// Placeholder name to value.
pub type Values = BTreeMap<String, String>;

/// An expanded command: an argv list, or a `/bin/sh -c` string in shell mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Argv(Vec<String>),
    Shell(String),
}

/// Plain substitution of known placeholders; unknown braces are left alone.
pub fn expand_value(template: &str, values: &Values) -> String {
    // One pass, so a value that itself contains "{out}" is not expanded again.
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let hit = PLACEHOLDERS
            .iter()
            .find(|name| after.starts_with(*name) && after[name.len()..].starts_with('}'));
        match hit.and_then(|name| values.get(*name).map(|v| (name, v))) {
            Some((name, value)) => {
                out.push_str(value);
                rest = &after[name.len() + 1..];
            }
            None => {
                let skip = hit.map_or(1, |name| name.len() + 2);
                out.push_str(&rest[i..i + skip]);
                rest = &rest[i + skip..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// argv list (shell=false) or a /bin/sh -c string (shell=true).
pub fn expand_command(
    command: &str,
    values: &Values,
    tests: &[String],
    shell: bool,
) -> Result<Command, ShlexError> {
    let mut values = values.clone();
    if shell {
        let quoted: Vec<String> = tests.iter().map(|t| shlex::quote(t)).collect();
        values.insert("tests".into(), quoted.join(" "));
        return Ok(Command::Shell(expand_value(command, &values)));
    }
    values.insert("tests".into(), tests.join(" "));
    let mut argv = Vec::new();
    for token in shlex::split(command)? {
        if token == "{tests}" {
            argv.extend(tests.iter().cloned());
        } else {
            argv.push(expand_value(&token, &values));
        }
    }
    Ok(Command::Argv(argv))
}

/// `python` on the user's PATH (falls back to python3), not the running interpreter.
pub fn default_python(which: impl Fn(&str) -> Option<String>) -> String {
    which("python")
        .or_else(|| which("python3"))
        .unwrap_or_else(|| "python".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> Values {
        [
            ("python", "/opt/py bin/python"),
            ("out", "/tmp/out"),
            ("worktree", "/wt"),
            ("lcov", "/tmp/l"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
    }

    fn argv(cmd: &str, tests: &[&str]) -> Vec<String> {
        let tests: Vec<String> = tests.iter().map(|t| t.to_string()).collect();
        match expand_command(cmd, &values(), &tests, false).unwrap() {
            Command::Argv(a) => a,
            Command::Shell(_) => unreachable!(),
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
        let tricky: Values = [
            ("python".into(), "{out}".into()),
            ("out".into(), "x".into()),
        ]
        .into_iter()
        .collect();
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
    }

    #[test]
    fn default_python_order() {
        assert_eq!(default_python(|n| Some(format!("/u/{n}"))), "/u/python");
        let only3 = |n: &str| (n == "python3").then(|| "/u/python3".to_string());
        assert_eq!(default_python(only3), "/u/python3");
        assert_eq!(default_python(|_| None), "python");
    }
}
