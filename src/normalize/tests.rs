use super::*;
use crate::model::{BranchKey, FunctionRecord};

#[test]
fn normalize_paths() {
    let cases: &[(&str, &[&str], &str, bool)] = &[
        ("/repo/src/a.py", &["/repo"], "src/a.py", false),
        // longest root wins, regardless of order
        (
            "/repo/sub/src/a.py",
            &["/repo", "/repo/sub"],
            "src/a.py",
            false,
        ),
        (
            "/repo/sub/src/a.py",
            &["/repo/sub", "/repo"],
            "src/a.py",
            false,
        ),
        // component boundary only
        ("/repo2/a.py", &["/repo"], "/repo2/a.py", true),
        ("/ci/build_x/a.rs", &["/ci/build"], "/ci/build_x/a.rs", true),
        // strip-prefix with and without trailing slash
        ("/ci/build/src/a.rs", &["/ci/build/"], "src/a.rs", false),
        ("/ci/build/src/a.rs", &["/ci/build"], "src/a.rs", false),
        ("shed_py/shed/calc.py", &["shed_py/"], "shed/calc.py", false),
        ("shed_py/shed/calc.py", &["shed_py"], "shed/calc.py", false),
        // relative stays relative, './' dropped
        ("shed/calc.py", &["/repo"], "shed/calc.py", false),
        ("./shed/calc.py", &[], "shed/calc.py", false),
        (".\\shed\\calc.py", &[], "shed/calc.py", false),
        // '..' resolved lexically
        ("/repo/tests/../src/a.py", &["/repo"], "src/a.py", false),
        ("src/../shed/a.py", &[], "shed/a.py", false),
        ("/../repo/a.py", &["/repo"], "a.py", false),
        ("/repo//src///a.py", &["/repo/"], "src/a.py", false),
        // backslashes and drive letters
        (
            "C:\\ci\\repo\\src\\lib.rs",
            &["C:\\ci\\repo"],
            "src/lib.rs",
            false,
        ),
        (
            "c:\\ci\\repo\\src\\lib.rs",
            &["C:/ci/repo/"],
            "src/lib.rs",
            false,
        ),
        ("src\\lib.rs", &[], "src/lib.rs", false),
        ("D:\\deps\\x.rs", &["C:/ci/repo"], "D:/deps/x.rs", true),
        ("C:/x.rs", &["/x"], "C:/x.rs", true),
        ("cd:/x", &[], "cd:/x", false),
        // external absolute: key is the cleaned path
        (
            "/usr/lib/python3.12/json/__init__.py",
            &["/repo"],
            "/usr/lib/python3.12/json/__init__.py",
            true,
        ),
        ("/opt/../opt/lib/x.py", &[], "/opt/lib/x.py", true),
        ("/repo", &["/repo"], "/repo", true),
        // escaping relative
        ("../other/x.py", &["/repo"], "../other/x.py", true),
        ("src/../../x.py", &[], "../x.py", true),
        // empty roots are ignored
        ("/repo/a.py", &["", ".", "./"], "/repo/a.py", true),
    ];
    for &(raw, roots, path, external) in cases {
        assert_eq!(
            normalize_path(raw, roots),
            (path.to_string(), external),
            "{raw}"
        );
    }
}

#[test]
fn symlinked_root() {
    let tmp = tempfile::tempdir().unwrap();
    let base = realpath(tmp.path().to_str().unwrap());
    std::fs::create_dir_all(format!("{base}/real/src")).unwrap();
    std::os::unix::fs::symlink(format!("{base}/real"), format!("{base}/link")).unwrap();
    let (real, link) = (format!("{base}/real"), format!("{base}/link"));
    // Root via the symlink, SF via the real path (llvm-cov resolves symlinks), and back.
    let ok = ("src/a.rs".to_string(), false);
    assert_eq!(normalize_path(&format!("{real}/src/a.rs"), &[&link]), ok);
    assert_eq!(normalize_path(&format!("{link}/src/a.rs"), &[&real]), ok);
    let elsewhere = format!("{base}/elsewhere.rs");
    assert_eq!(
        normalize_path(&elsewhere, &[&link]),
        (elsewhere.clone(), true)
    );
}

#[test]
fn realpath_follows_links() {
    let tmp = tempfile::tempdir().unwrap();
    let base = realpath(tmp.path().to_str().unwrap());
    std::fs::create_dir_all(format!("{base}/real/sub")).unwrap();
    std::os::unix::fs::symlink(format!("{base}/real"), format!("{base}/link")).unwrap();
    std::os::unix::fs::symlink("loop", format!("{base}/loop")).unwrap();
    let rp = |p: &str| realpath(&format!("{base}/{p}"));
    assert_eq!(
        rp("link/sub/missing/x"),
        format!("{base}/real/sub/missing/x")
    );
    assert_eq!(rp("link/../link/sub"), format!("{base}/real/sub"));
    assert_eq!(rp("loop/x"), format!("{base}/loop/x"));
    assert_eq!(realpath("/"), "/");
    assert_eq!(realpath("//a/../.."), "/");
}

fn fc(path: &str, lines: &[(i64, i128)], functions: Vec<FunctionRecord>) -> FileCoverage {
    FileCoverage {
        path: path.into(),
        lines: lines.iter().copied().collect(),
        functions,
        ..Default::default()
    }
}

#[test]
fn snapshot_merges_collisions() {
    let f = |hits| FunctionRecord {
        name: "f".into(),
        start: 1,
        end: Some(3),
        hits: Some(hits),
    };
    let snap = Snapshot {
        files: [
            fc("/repo/shed/calc.py", &[(5, 1), (1, 1)], vec![f(1)]),
            fc("./shed/calc.py", &[(5, 2), (2, 0)], vec![f(2)]),
            fc("/usr/lib/x.py", &[(1, 1)], vec![]),
            fc("../up.py", &[(1, 0)], vec![]),
        ]
        .into_iter()
        .map(|fc| (fc.path.clone(), fc))
        .collect(),
        test_names: vec!["t1".into(), "t2".into()],
    };
    let out = normalize_snapshot(&snap, &["/repo"]);
    assert_eq!(out.test_names, ["t1", "t2"]);
    let keys: Vec<_> = out.files.keys().collect();
    assert_eq!(keys, ["../up.py", "/usr/lib/x.py", "shed/calc.py"]);
    let calc = &out.files["shed/calc.py"];
    assert_eq!(calc.path, "shed/calc.py");
    assert!(!calc.external);
    assert_eq!(calc.lines, [(1, 1), (2, 0), (5, 3)].into_iter().collect());
    assert_eq!(calc.functions, [f(3)]);
    assert!(out.files["/usr/lib/x.py"].external);
    assert!(out.files["../up.py"].external);
}

#[test]
fn snapshot_merges_branches() {
    let key = BranchKey {
        line: 2,
        block: "0".into(),
        branch: "0".into(),
    };
    let mut a = fc("/r/x.c", &[], vec![]);
    a.branches.insert(key.clone(), None);
    let mut b = fc("x.c", &[], vec![]);
    b.branches.insert(key.clone(), Some(4));
    let snap = Snapshot {
        files: [("/r/x.c".into(), a), ("x.c".into(), b)]
            .into_iter()
            .collect(),
        test_names: vec![],
    };
    let out = normalize_snapshot(&snap, &["/r"]);
    assert_eq!(out.files["x.c"].branches[&key], Some(4));
}

#[test]
fn fixtures() {
    let load = |name: &str| {
        let path = format!("{}/tests/fixtures/lcov/{name}", env!("CARGO_MANIFEST_DIR"));
        crate::lcov::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    let snap = load("llvm_cov.lcov");
    let out = normalize_snapshot(&snap, &["/home/runner/work/shed/shed_rs/"]);
    assert_eq!(
        out.files.keys().collect::<Vec<_>>(),
        ["src/lib.rs", "src/util.rs"]
    );
    assert!(out.files.values().all(|fc| !fc.external));
    let raw = &snap.files["/home/runner/work/shed/shed_rs/src/lib.rs"];
    assert_eq!(out.files["src/lib.rs"].lines, raw.lines);
    assert_eq!(out.files["src/lib.rs"].functions, raw.functions);
    // Idempotent.
    let once = normalize_snapshot(&load("tarpaulin.lcov"), &["/builds/shed/shed_rs"]);
    assert_eq!(once.files.keys().collect::<Vec<_>>(), ["src/lib.rs"]);
    assert_eq!(normalize_snapshot(&once, &["/builds/shed/shed_rs"]), once);
}
