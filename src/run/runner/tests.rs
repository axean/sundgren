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
    let runs = run(
        &preset,
        "py",
        &[],
        wt.path(),
        &sel(&[]),
        out.path(),
        2,
        None,
    )
    .unwrap();
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
    let e = run(
        &preset,
        "py",
        &[],
        wt.path(),
        &sel(&[]),
        out.path(),
        1,
        None,
    )
    .unwrap_err();
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
    let runs = run(
        &preset,
        "py",
        &[],
        wt.path(),
        &sel(&[]),
        out.path(),
        1,
        None,
    )
    .unwrap();
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
