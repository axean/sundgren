use super::*;

pub fn fc(lines: &[(LineNo, Count)], funcs: Vec<FunctionRecord>) -> FileCoverage {
    FileCoverage {
        path: "a.py".into(),
        lines: lines.iter().copied().collect(),
        functions: funcs,
        ..Default::default()
    }
}

fn func(name: &str, start: LineNo, end: Option<LineNo>, hits: Option<Count>) -> FunctionRecord {
    FunctionRecord {
        name: name.into(),
        start,
        end,
        hits,
    }
}

fn key(l: LineNo, b: &str, r: &str) -> BranchKey {
    BranchKey {
        line: l,
        block: b.into(),
        branch: r.into(),
    }
}

#[test]
fn branch_key_order() {
    let mut v = vec![
        key(2, "0", "0"),
        key(1, "1", "0"),
        key(1, "0", "1"),
        key(1, "0", "0"),
    ];
    v.sort();
    let want = [
        key(1, "0", "0"),
        key(1, "0", "1"),
        key(1, "1", "0"),
        key(2, "0", "0"),
    ];
    assert_eq!(v, want);
}

#[test]
fn merge() {
    let k = key(1, "0", "0");
    let mut a = fc(&[(1, 1), (2, 0)], vec![func("f", 1, None, None)]);
    a.branches.insert(k.clone(), None);
    a.external = true;
    let mut b = fc(
        &[(2, 2), (3, 0)],
        vec![func("f", 1, Some(4), Some(2)), func("e", 1, None, Some(0))],
    );
    b.branches.insert(k.clone(), Some(3));
    let m = merge_file_coverage(&a, &b);
    assert_eq!(m.lines, [(1, 1), (2, 2), (3, 0)].into_iter().collect());
    assert_eq!(m.branches[&k], Some(3));
    assert_eq!(
        m.functions,
        vec![func("e", 1, None, Some(0)), func("f", 1, Some(4), Some(2))]
    );
    assert!(!m.external);
}

#[test]
fn merge_taken_dash_is_identity() {
    assert_eq!(add_taken(None, None), None);
    assert_eq!(add_taken(None, Some(0)), Some(0));
    assert_eq!(add_taken(Some(2), Some(3)), Some(5));
}

#[test]
fn outcome_names() {
    for s in [
        "passed", "failed", "skipped", "xfailed", "xpassed", "error", "not_run",
    ] {
        assert_eq!(Outcome::parse(s).unwrap().as_str(), s);
    }
    assert_eq!(Outcome::parse("nope"), None);
}

#[test]
fn to_json() {
    let mut f = fc(&[(10, 1), (2, 0)], vec![func("f", 1, None, Some(1))]);
    f.branches.insert(key(2, "0", "1"), None);
    let snap = Snapshot {
        files: [("a.py".to_string(), f)].into_iter().collect(),
        test_names: vec!["t".into()],
    };
    let v = snapshot_json(&snap);
    assert_eq!(v["test_names"], json!(["t"]));
    assert_eq!(v["files"][0]["lines"], json!({"2": 0, "10": 1}));
    assert_eq!(v["files"][0]["branches"][0]["taken"], Value::Null);
    assert_eq!(v["files"][0]["functions"][0]["end"], Value::Null);
    let mut big = Snapshot::default();
    big.files
        .insert("b".into(), fc(&[(-1, 18446744073709551615)], vec![]));
    let out = serde_json::to_string(&snapshot_json(&big)).unwrap();
    assert!(
        out.contains(r#""lines":{"-1":18446744073709551615}"#),
        "{out}"
    );
}
