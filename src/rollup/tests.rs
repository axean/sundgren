use super::*;
use crate::model::FunctionRecord;

fn span(name: &str, start: LineNo, end: LineNo, body: Option<LineNo>) -> Span {
    Span {
        name: name.into(),
        start,
        end,
        body_start: body,
    }
}

fn f() -> Span {
    span("f", 1, 4, Some(2)) // line 1 is the signature
}

fn g() -> Span {
    span("g", 6, 7, Some(7))
}

fn lm(deleted: &[LineNo], added: &[LineNo]) -> LineMap {
    LineMap {
        deleted: deleted.to_vec(),
        added: added.to_vec(),
        ..LineMap::identity("m.py")
    }
}

fn l(pairs: &[(LineNo, Count)]) -> Lines {
    pairs.iter().copied().collect()
}

fn set(v: &[LineNo]) -> BTreeSet<LineNo> {
    v.iter().copied().collect()
}

fn status(
    base: &[(LineNo, Count)],
    patched: &[(LineNo, Count)],
    lost: &[LineNo],
    gained: &[LineNo],
) -> (&'static str, i64, i64, i64) {
    let sets = LineSets {
        lost: set(lost),
        gained: set(gained),
        ..Default::default()
    };
    let out = function_deltas(
        Some(&[f()]),
        Some(&[f()]),
        &lm(&[], &[]),
        &l(base),
        &l(patched),
        &sets,
    );
    let [fd] = &out[..] else { panic!("{out:?}") };
    (fd.status.as_str(), fd.lost, fd.gained, fd.executable)
}

#[test]
fn statuses() {
    // The signature line still runs at import; it doesn't keep f lit.
    let full = [(1, 1), (2, 1), (3, 1), (4, 1)];
    let dark = [(1, 1), (2, 0), (3, 0), (4, 0)];
    assert_eq!(
        status(&full, &dark, &[2, 3, 4], &[]),
        ("went_dark", 3, 0, 3)
    );
    assert_eq!(
        status(
            &[(1, 1), (2, 0), (3, 0)],
            &[(1, 1), (2, 1), (3, 1)],
            &[],
            &[2, 3]
        ),
        ("lit_up", 0, 2, 2)
    );
    assert_eq!(
        status(&[(2, 1), (3, 1)], &[(2, 1), (3, 0)], &[3], &[]),
        ("partial_loss", 1, 0, 2)
    );
    assert_eq!(
        status(&[(2, 1), (3, 0)], &[(2, 1), (3, 1)], &[], &[3]),
        ("partial_gain", 0, 1, 2)
    );
    assert_eq!(
        status(&[(2, 1), (3, 0)], &[(2, 0), (3, 1)], &[2], &[3]),
        ("mixed", 1, 1, 2)
    );
    assert_eq!(
        status(&[(2, 1), (3, 0)], &[(2, 1), (3, 0)], &[], &[]),
        ("unchanged", 0, 0, 2)
    );
}

#[test]
fn added_removed_and_no_spans() {
    let id = lm(&[], &[]);
    let none = LineSets::default();
    let key = |v: Vec<FunctionDelta>| -> Vec<_> {
        v.into_iter()
            .map(|d| (d.name, d.status.as_str(), d.start, d.end, d.executable))
            .collect()
    };
    let added = function_deltas(
        Some(&[]),
        Some(&[g()]),
        &id,
        &l(&[]),
        &l(&[(6, 1), (7, 0)]),
        &none,
    );
    assert_eq!(key(added), [("g".to_string(), "added", 6, 7, 1)]);
    let removed = function_deltas(
        Some(&[g()]),
        Some(&[]),
        &id,
        &l(&[(6, 1), (7, 1)]),
        &l(&[]),
        &none,
    );
    assert_eq!(key(removed), [("g".to_string(), "removed", 6, 7, 1)]);
    assert!(function_deltas(None, None, &id, &l(&[(1, 1)]), &l(&[(1, 0)]), &none).is_empty());
    let sorted = function_deltas(
        Some(&[g(), f()]),
        Some(&[g(), f()]),
        &id,
        &l(&[]),
        &l(&[]),
        &none,
    );
    assert_eq!(
        sorted.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
        ["f", "g"]
    );
}

#[test]
fn flaky_lines_do_not_decide_went_dark() {
    let sets = LineSets {
        lost: set(&[2]),
        flaky_patched: set(&[3]),
        ..Default::default()
    };
    let out = function_deltas(
        Some(&[f()]),
        Some(&[f()]),
        &lm(&[], &[]),
        &l(&[(2, 1), (3, 1)]),
        &l(&[(2, 0), (3, 1)]),
        &sets,
    );
    assert_eq!(out[0].status, FunctionStatus::WentDark);
}

#[test]
fn renamed_function_matches_by_overlap() {
    let sets = LineSets {
        lost: set(&[2, 3]),
        ..Default::default()
    };
    let out = function_deltas(
        Some(&[span("old", 1, 3, Some(2))]),
        Some(&[span("new", 1, 3, Some(2))]),
        &lm(&[1], &[1]),
        &l(&[(1, 1), (2, 1), (3, 1)]),
        &l(&[(1, 1), (2, 0), (3, 0)]),
        &sets,
    );
    let [fd] = &out[..] else { panic!("{out:?}") };
    assert_eq!(
        (fd.name.as_str(), fd.status),
        ("new", FunctionStatus::WentDark)
    );
}

#[test]
fn unknown_side_borrows_spans_through_the_linemap() {
    let sets = LineSets {
        lost: set(&[4, 5]),
        ..Default::default()
    };
    let out = function_deltas(
        Some(&[f()]),
        None,
        &lm(&[], &[1, 2]),
        &l(&[(2, 1), (3, 1)]),
        &l(&[(4, 0), (5, 0)]),
        &sets,
    );
    let [fd] = &out[..] else { panic!("{out:?}") };
    assert_eq!(
        (fd.name.as_str(), fd.status, fd.start, fd.end),
        ("f", FunctionStatus::WentDark, 3, 6)
    );
}

// --- lcov FN spans ---

fn fns(funcs: &[(&str, LineNo, Option<LineNo>)], lines: &[(LineNo, Count)]) -> Vec<Span> {
    let fc = FileCoverage {
        path: "m.rs".into(),
        lines: l(lines),
        functions: funcs
            .iter()
            .map(|&(name, start, end)| FunctionRecord {
                name: name.into(),
                start,
                end,
                hits: Some(1),
            })
            .collect(),
        ..Default::default()
    };
    lcov_spans(Some(&fc)).unwrap()
}

#[test]
fn lcov_spans_cases() {
    let s = span;
    assert_eq!(
        fns(&[("f", 2, Some(5)), ("g", 7, Some(9))], &[(2, 1), (9, 1)]),
        [s("f", 2, 5, Some(3)), s("g", 7, 9, Some(8))]
    );
    // A missing end runs to the next start, the last one to the last executable line.
    assert_eq!(
        fns(
            &[("g", 8, None), ("f", 3, None)],
            &[(3, 1), (4, 1), (8, 1), (9, 1)]
        ),
        [s("f", 3, 7, Some(4)), s("g", 8, 9, Some(9))]
    );
    assert_eq!(
        fns(&[("f", 3, None)], &[(1, 1), (3, 1), (4, 0), (12, 0)]),
        [s("f", 3, 12, Some(4))]
    );
    assert_eq!(fns(&[("f", 3, None)], &[]), [s("f", 3, 3, None)]);
    assert_eq!(fns(&[("f", 30, None)], &[(1, 1)]), [s("f", 30, 30, None)]);
    // Same start (outer fn and closure): run to the next distinct start.
    assert_eq!(
        fns(
            &[("a", 2, None), ("b", 2, None), ("c", 6, None)],
            &[(2, 1), (7, 1)]
        ),
        [
            s("a", 2, 5, Some(3)),
            s("b", 2, 5, Some(3)),
            s("c", 6, 7, Some(7))
        ]
    );
    assert_eq!(fns(&[("f", 5, Some(3))], &[(5, 1)]), [s("f", 5, 5, None)]);
    assert_eq!(
        fns(
            &[("f", 1, Some(4)), ("g", 6, None), ("h", 10, None)],
            &[(11, 1)]
        ),
        [
            s("f", 1, 4, Some(2)),
            s("g", 6, 9, Some(7)),
            s("h", 10, 11, Some(11))
        ]
    );
    // The FN line is the signature: only the body may go dark.
    let [divide] = &fns(&[("divide", 1, None)], &[(1, 1), (2, 0), (3, 0), (5, 0)])[..] else {
        panic!()
    };
    assert_eq!(divide.body_lines().collect::<Vec<_>>(), [2, 3, 4, 5]);
    assert_eq!(lcov_spans(None), None);
    assert_eq!(lcov_spans(Some(&FileCoverage::default())), None);
}

#[test]
fn body_lines() {
    assert_eq!(span("f", 3, 7, Some(5)).body_lines(), 5..=7);
    assert_eq!(span("f", 3, 7, None).body_lines(), 3..=7);
}

// --- tree ---

fn stats(lost: i64, gained: i64, new_miss: i64, exec_b: i64, exec_p: i64) -> Stats {
    Stats {
        lost,
        gained,
        new_miss,
        executable_base: exec_b,
        executable_patched: exec_p,
        ..Default::default()
    }
}

#[test]
fn stats_add() {
    let a = Stats {
        hit_patched: 2,
        ..stats(1, 0, 0, 3, 0)
    };
    let b = stats(2, 1, 0, 1, 0);
    assert_eq!(
        a + b,
        Stats {
            hit_patched: 2,
            ..stats(3, 1, 0, 4, 0)
        }
    );
}

#[test]
fn tree_sums_and_sorts() {
    let tree = build_tree([
        ("pkg/sub/b.py", stats(1, 0, 0, 2, 0)),
        ("pkg/a.py", stats(0, 2, 0, 0, 5)),
        ("top.py", stats(0, 0, 1, 0, 0)),
        ("pkg/sub/a.py", stats(4, 0, 0, 0, 0)),
    ]);
    assert_eq!(
        (tree.name.as_str(), tree.path.as_str(), tree.kind),
        ("", "", NodeKind::Dir)
    );
    assert_eq!(tree.stats, stats(5, 2, 1, 2, 5));
    let [pkg, top] = &tree.children.as_ref().unwrap()[..] else {
        panic!()
    };
    assert_eq!((pkg.path.as_str(), pkg.kind), ("pkg", NodeKind::Dir));
    assert_eq!((top.path.as_str(), top.kind), ("top.py", NodeKind::File));
    let pkg_children = pkg.children.as_ref().unwrap();
    assert_eq!(
        pkg_children
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["a.py", "sub"]
    );
    let sub = &pkg_children[1];
    assert_eq!((sub.path.as_str(), sub.stats.lost), ("pkg/sub", 5));
    let leaves: Vec<_> = sub
        .children
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| (c.name.as_str(), c.path.as_str(), c.children.is_none()))
        .collect();
    assert_eq!(
        leaves,
        [
            ("a.py", "pkg/sub/a.py", true),
            ("b.py", "pkg/sub/b.py", true)
        ]
    );
}

#[test]
fn file_and_dir_may_share_a_name() {
    let tree = build_tree([("a", Stats::default()), ("a/b", Stats::default())]);
    let kinds: Vec<_> = tree
        .children
        .unwrap()
        .iter()
        .map(|c| (c.name.clone(), c.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("a".to_string(), NodeKind::Dir),
            ("a".to_string(), NodeKind::File)
        ]
    );
}

#[test]
fn empty_tree() {
    let tree = build_tree([]);
    let want = TreeNode {
        name: String::new(),
        path: String::new(),
        kind: NodeKind::Dir,
        stats: Stats::default(),
        children: Some(vec![]),
    };
    assert_eq!(tree, want);
    let keys: Vec<String> = serde_json::to_value(Stats::default())
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let mut want = [
        "lost",
        "gained",
        "new_hit",
        "new_miss",
        "deleted_hit",
        "deleted_miss",
        "executable_base",
        "executable_patched",
        "hit_base",
        "hit_patched",
    ];
    want.sort();
    assert_eq!(keys, want);
}
