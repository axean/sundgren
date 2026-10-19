use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh directory under the system temp dir; removed by the caller.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sundgren-frontend-python-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn python() -> OsString {
    std::env::var_os("SUNDGREN_TEST_PYTHON").unwrap_or_else(|| "python3".into())
}

fn req(dir: &Path, name: &str) -> Request {
    Request {
        file: dir.join(name),
        path: format!("pkg/{name}"),
    }
}

#[test]
fn batch_keeps_going_past_bad_files() {
    let dir = scratch("batch");
    std::fs::write(dir.join("good.py"), "def f():\n    return 1\n").unwrap();
    std::fs::write(dir.join("bad.py"), "def f(:\n").unwrap();
    std::fs::write(dir.join("nul.py"), "x = 1\0\n").unwrap();
    std::fs::write(dir.join("crlf.py"), "@d\r\ndef g(): pass\r\n").unwrap();
    let names = ["good.py", "bad.py", "missing.py", "nul.py", "crlf.py"];
    let reqs: Vec<Request> = names.iter().map(|n| req(&dir, n)).collect();
    let out = parse_files(&python(), &reqs).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(out.len(), 5);
    let good = out[0].as_ref().unwrap();
    assert_eq!(good.path, "pkg/good.py");
    assert_eq!(good.nodes[1].id.as_str(), "pkg/good.py::f");
    assert!(good.producer.starts_with("sundgren-python-ast/1 "));
    assert!(matches!(out[1], Err(FileError::Syntax(_))), "{:?}", out[1]);
    assert!(
        matches!(out[2], Err(FileError::Unreadable(_))),
        "{:?}",
        out[2]
    );
    assert!(matches!(out[3], Err(FileError::Syntax(_))), "{:?}", out[3]);
    let crlf = out[4].as_ref().unwrap();
    assert_eq!(crlf.nodes[1].span.start.line, 1);
}

#[cfg(unix)]
#[test]
fn non_utf8_file_names_round_trip() {
    use std::os::unix::ffi::OsStrExt;
    let dir = scratch("names");
    let file = dir.join(OsStr::from_bytes(b"caf\xe9.py"));
    std::fs::write(&file, "def f(): pass\n").unwrap();
    let r = Request {
        file,
        path: "m.py".into(),
    };
    let out = parse_files(&python(), &[r]).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out[0].as_ref().unwrap().nodes.len(), 2);
}

#[cfg(unix)]
fn fake_python(dir: &Path, body: &str) -> OsString {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake-python");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.into_os_string()
}

#[cfg(unix)]
#[test]
fn a_crash_costs_one_file_and_restarts() {
    let dir = scratch("crash");
    // Ready, answers nothing, dies: every file crashes, each in its own process.
    let fake = fake_python(&dir, "echo '{\"ready\":\"x\"}'; echo boom >&2; exit 3");
    let reqs: Vec<Request> = ["a.py", "b.py", "c.py"]
        .iter()
        .map(|n| req(&dir, n))
        .collect();
    let out = parse_files(&fake, &reqs).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out, vec![Err(FileError::Crashed("boom".into())); 3]);
}

#[cfg(unix)]
#[test]
fn unusable_interpreters_are_run_errors() {
    let dir = scratch("unusable");
    let fake = fake_python(&dir, "echo 'SyntaxError: old python' >&2; exit 1");
    let r = req(&dir, "a.py");
    let err = parse_files(&fake, std::slice::from_ref(&r)).unwrap_err();
    assert!(
        matches!(&err, RunError::NotReady(e) if e.contains("old python")),
        "{err}"
    );
    let missing = dir.join("no-such-python");
    let err = parse_files(missing.as_os_str(), std::slice::from_ref(&r)).unwrap_err();
    assert!(matches!(err, RunError::Spawn(_)));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn no_requests_no_process() {
    let out = parse_files(OsStr::new("/nonexistent/python"), &[]).unwrap();
    assert!(out.is_empty());
}
