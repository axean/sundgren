use super::*;
use crate::patch::parse_patch;

fn sel(tests: &[&str]) -> Selection {
    Selection {
        tests: tests.iter().map(|t| t.to_string()).collect(),
        run_all: false,
        reason: "r".into(),
    }
}

/// A repo with these files, and a patch touching `paths` (old, new; None = /dev/null).
fn select(files: &[&str], paths: &[(Option<&str>, Option<&str>)]) -> Selection {
    let dir = tempfile::tempdir().unwrap();
    for f in files {
        let p = dir.path().join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "x\n").unwrap();
    }
    let mut text = String::new();
    for (old, new) in paths {
        let a = old.map_or("/dev/null".into(), |p| format!("a/{p}"));
        let b = new.map_or("/dev/null".into(), |p| format!("b/{p}"));
        let hunk = match (old, new) {
            (None, _) => "@@ -0,0 +1 @@\n+x\n",
            (_, None) => "@@ -1 +0,0 @@\n-x\n",
            _ => "@@ -1 +1 @@\n-x\n+y\n",
        };
        text += &format!("--- {a}\n+++ {b}\n{hunk}");
    }
    let patch = parse_patch(&text).unwrap();
    pytest_files(&patch, dir.path())
}

#[test]
fn pytest_files_cases() {
    let both = |p| (Some(p), Some(p));
    let got = select(
        &["tests/test_b.py", "tests/a_test.py"],
        &[both("tests/test_b.py"), both("tests/a_test.py")],
    );
    assert_eq!(
        got,
        Selection {
            tests: vec!["tests/a_test.py".into(), "tests/test_b.py".into()],
            run_all: false,
            reason: "touched test files".into(),
        }
    );
    let got = select(&["pkg/x.py"], &[both("pkg/x.py")]);
    assert!(got.run_all);
    assert_eq!(got.reason, "pkg/x.py changed; running all tests");
    let got = select(&[], &[both("pyproject.toml")]);
    assert_eq!(got.reason, "pyproject.toml changed; running all tests");
    // Non-Python files are ignored.
    let got = select(
        &["tests/test_a.py"],
        &[both("README.md"), both("tests/test_a.py")],
    );
    assert_eq!(got.tests, ["tests/test_a.py"]);
    let got = select(&[], &[both("README.md")]);
    assert_eq!(got.reason, "no touched test files; running all tests");
    let got = select(&[], &[(None, Some("tests/test_new.py"))]);
    assert_eq!(
        got.reason,
        "test file tests/test_new.py added; running all tests"
    );
    let got = select(&[], &[(Some("tests/test_old.py"), None)]);
    assert_eq!(
        got.reason,
        "test file tests/test_old.py deleted; running all tests"
    );
}

#[test]
fn choose_precedence() {
    let patch = Patch::default();
    let here = Path::new(".");
    let got = choose(&["x".into()], Some("nope"), &patch, here).unwrap();
    assert_eq!(got.reason, "--tests given");
    assert_eq!(
        choose(&[], None, &patch, here).unwrap().reason,
        "no selector; running all tests"
    );
    let err = choose(&[], Some("nope"), &patch, here).unwrap_err();
    assert_eq!(
        (err.code, err.msg.as_str()),
        (crate::EXIT_FAILURE, "unknown selector 'nope'")
    );
}

#[test]
fn restrict_drops_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("t")).unwrap();
    std::fs::write(dir.path().join("t/a.py"), "").unwrap();
    let got = restrict_to_tree(&sel(&["t/a.py::x", "t/b.py"]), dir.path()).unwrap();
    assert_eq!(got.tests, ["t/a.py::x"]);
    assert_eq!(got.reason, "r; 1 missing on this side");
    assert_eq!(restrict_to_tree(&sel(&["t/b.py"]), dir.path()), None);
    assert_eq!(
        restrict_to_tree(&sel(&["t/a.py"]), dir.path()),
        Some(sel(&["t/a.py"]))
    );
}
