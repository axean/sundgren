use super::*;
use crate::model::FunctionRecord;

pub(crate) const P: &str = "pkg/a.py";
/// `None` for lines and counts in compact expectations.
const N: i64 = -1;

pub(crate) type Br<'a> = &'a [(LineNo, &'a str, &'a str, Option<Count>)];

pub(crate) fn fc(path: &str, lines: &[(LineNo, Count)], branches: Br) -> FileCoverage {
    FileCoverage {
        path: path.into(),
        lines: lines.iter().copied().collect(),
        branches: branches
            .iter()
            .map(|&(line, block, branch, t)| {
                let key = BranchKey {
                    line,
                    block: block.into(),
                    branch: branch.into(),
                };
                (key, t)
            })
            .collect(),
        ..Default::default()
    }
}

pub(crate) fn lines(l: &[(LineNo, Count)]) -> FileCoverage {
    fc(P, l, &[])
}

pub(crate) fn snap(files: &[FileCoverage]) -> Snapshot {
    Snapshot {
        files: files.iter().map(|f| (f.path.clone(), f.clone())).collect(),
        test_names: vec![],
    }
}

fn lmap(old: Option<&str>, new: Option<&str>, deleted: &[LineNo], added: &[LineNo]) -> LineMaps {
    let lm = LineMap {
        old_path: old.map(Into::into),
        new_path: new.map(Into::into),
        old_to_new: BTreeMap::new(),
        deleted: deleted.to_vec(),
        added: added.to_vec(),
    };
    [&lm.old_path, &lm.new_path]
        .into_iter()
        .flatten()
        .map(|p| (p.clone(), lm.clone()))
        .collect()
}

fn edit(deleted: &[LineNo], added: &[LineNo]) -> LineMaps {
    lmap(Some(P), Some(P), deleted, added)
}

pub(crate) fn d(base: &[FileCoverage], patched: &[FileCoverage], lms: &LineMaps) -> Delta {
    diff(&[snap(base)], &[snap(patched)], lms, DiffOptions::default()).unwrap()
}

fn one<'a>(delta: &'a Delta, path: &str) -> &'a FileDelta {
    let mut it = delta.files.iter().filter(|f| f.path == path);
    let f = it.next().expect("file in delta");
    assert!(it.next().is_none());
    f
}

fn o(x: Option<i64>) -> i64 {
    x.unwrap_or(N)
}

fn c(x: Option<Count>) -> i64 {
    x.map_or(N, |x| x as i64)
}

type Cat = (i64, i64, &'static str, (i64, i64));

/// (line, base_line, category, hits) in report order; N stands for None.
fn cats(f: &FileDelta) -> Vec<Cat> {
    let pair = |p: Pair| (c(p.0), c(p.1));
    f.lines
        .iter()
        .map(|e| (o(e.line), o(e.base_line), e.category.as_str(), pair(e.hits)))
        .collect()
}

type BCat = (i64, i64, String, &'static str, (i64, i64));

fn bcats(f: &FileDelta) -> Vec<BCat> {
    f.branches
        .iter()
        .map(|b| {
            let taken = (c(b.taken.0), c(b.taken.1));
            (
                o(b.line),
                o(b.base_line),
                b.branch.clone(),
                b.category.as_str(),
                taken,
            )
        })
        .collect()
}

fn bc(line: i64, base: i64, br: &str, cat: &'static str, t: (i64, i64)) -> BCat {
    (line, base, br.to_string(), cat, t)
}

fn codes(delta: &Delta) -> Vec<&str> {
    delta.warnings.iter().map(|w| w.code.as_str()).collect()
}

fn tree(files: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for f in files {
        let path = dir.path().join(f);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x\n").unwrap();
    }
    dir
}

fn with_roots(
    base: &[Snapshot],
    patched: &[Snapshot],
    lms: &LineMaps,
    base_root: Option<&Path>,
    patched_root: Option<&Path>,
) -> Delta {
    let opts = DiffOptions {
        base_root,
        patched_root,
        ..Default::default()
    };
    diff(base, patched, lms, opts).unwrap()
}

// --- line classification ---

#[test]
fn simple_categories() {
    let cases: [(Count, Count, &str); 4] = [
        (2, 5, "kept_hit"),
        (0, 0, "kept_miss"),
        (3, 0, "lost"),
        (0, 2, "gained"),
    ];
    for (b, p, cat) in cases {
        let delta = d(&[lines(&[(1, b)])], &[lines(&[(1, p)])], &LineMaps::new());
        let f = one(&delta, P);
        assert_eq!(cats(f), [(1, 1, cat, (b as i64, p as i64))]);
        assert_eq!(f.changed(), !cat.starts_with("kept"));
    }
    let delta = d(&[lines(&[(1, 3)])], &[lines(&[(1, 0)])], &LineMaps::new());
    assert_eq!(one(&delta, P).stats.lost, 1);
}

#[test]
fn absent_on_one_side_is_unmeasured_with_warning() {
    let delta = d(
        &[lines(&[(1, 1), (2, 1)])],
        &[lines(&[(1, 1), (3, 0)])],
        &LineMaps::new(),
    );
    assert_eq!(
        cats(one(&delta, P)),
        [
            (1, 1, "kept_hit", (1, 1)),
            (2, 2, "unmeasured", (1, N)),
            (3, 3, "unmeasured", (N, 0))
        ]
    );
    assert_eq!(codes(&delta), ["unmeasured"]);
    assert!(delta.warnings[0].message.starts_with("pkg/a.py: 2 line(s)"));
}

#[test]
fn added_lines_are_new_hit_and_new_miss() {
    let delta = d(
        &[lines(&[(1, 1), (2, 1)])],
        &[lines(&[(1, 1), (2, 1), (3, 0), (4, 1)])],
        &edit(&[], &[2, 3]),
    );
    let f = one(&delta, P);
    assert_eq!(
        cats(f),
        [
            (1, 1, "kept_hit", (1, 1)),
            (2, N, "new_hit", (N, 1)),
            (3, N, "new_miss", (N, 0)),
            (4, 2, "kept_hit", (1, 1))
        ]
    );
    assert_eq!((f.stats.new_hit, f.stats.new_miss), (1, 1));
}

#[test]
fn deleted_lines_sort_last() {
    let delta = d(
        &[lines(&[(1, 1), (2, 1), (3, 0), (4, 0)])],
        &[lines(&[(1, 1), (2, 0)])],
        &edit(&[2, 3], &[]),
    );
    assert_eq!(
        cats(one(&delta, P)),
        [
            (1, 1, "kept_hit", (1, 1)),
            (2, 4, "kept_miss", (0, 0)),
            (N, 2, "deleted_hit", (1, N)),
            (N, 3, "deleted_miss", (0, N))
        ]
    );
}

#[test]
fn changed_line_is_deleted_plus_added() {
    let delta = d(
        &[lines(&[(1, 1), (2, 1)])],
        &[lines(&[(1, 1), (2, 0)])],
        &edit(&[2], &[2]),
    );
    assert_eq!(
        cats(one(&delta, P)),
        [
            (1, 1, "kept_hit", (1, 1)),
            (2, N, "new_miss", (N, 0)),
            (N, 2, "deleted_hit", (1, N))
        ]
    );
}

#[test]
fn shifted_lines_are_not_lost_or_gained() {
    let delta = d(
        &[lines(&[(1, 1), (2, 0)])],
        &[lines(&[(3, 1), (4, 0)])],
        &edit(&[], &[1, 2]),
    );
    assert_eq!(
        cats(one(&delta, P)),
        [(3, 1, "kept_hit", (1, 1)), (4, 2, "kept_miss", (0, 0))]
    );
}

// --- branches ---

#[test]
fn branch_lost_gained_kept() {
    let base = fc(
        P,
        &[(1, 1)],
        &[
            (1, "0", "0", Some(2)),
            (1, "0", "1", Some(0)),
            (1, "0", "2", Some(1)),
        ],
    );
    let patched = fc(
        P,
        &[(1, 1)],
        &[
            (1, "0", "0", Some(0)),
            (1, "0", "1", Some(4)),
            (1, "0", "2", Some(1)),
        ],
    );
    let delta = d(&[base], &[patched], &LineMaps::new());
    assert_eq!(
        bcats(one(&delta, P)),
        [
            bc(1, 1, "0", "branch_lost", (2, 0)),
            bc(1, 1, "1", "branch_gained", (0, 4)),
            bc(1, 1, "2", "branch_kept_hit", (1, 1))
        ]
    );
}

#[test]
fn branch_dash_is_not_taken() {
    let base = fc(
        P,
        &[(1, 1), (2, 1)],
        &[(1, "0", "0", Some(1)), (2, "0", "0", None)],
    );
    let patched = fc(
        P,
        &[(1, 1), (2, 1)],
        &[(1, "0", "0", None), (2, "0", "0", None)],
    );
    let delta = d(&[base], &[patched], &LineMaps::new());
    assert_eq!(
        bcats(one(&delta, P)),
        [
            bc(1, 1, "0", "branch_lost", (1, N)),
            bc(2, 2, "0", "branch_kept_miss", (N, N))
        ]
    );
}

#[test]
fn branch_on_added_and_deleted_lines() {
    let base = fc(P, &[(1, 1), (2, 1)], &[(2, "0", "0", Some(1))]);
    let patched = fc(P, &[(1, 1), (2, 1)], &[(2, "0", "0", Some(0))]);
    let delta = d(&[base], &[patched], &edit(&[2], &[2]));
    assert_eq!(
        bcats(one(&delta, P)),
        [
            bc(2, N, "0", "branch_new", (N, 0)),
            bc(N, 2, "0", "branch_deleted", (1, N))
        ]
    );
}

#[test]
fn coverage_py_branch_ids_follow_shifted_targets() {
    let base = fc(
        P,
        &[(6, 1), (7, 1), (8, 1)],
        &[
            (6, "0", "jump to line 7", Some(1)),
            (6, "0", "jump to line 8", Some(1)),
        ],
    );
    let patched = fc(
        P,
        &[(8, 1), (9, 1), (10, 1)],
        &[
            (8, "0", "jump to line 9", Some(1)),
            (8, "0", "jump to line 10", Some(1)),
        ],
    );
    let delta = d(&[base], &[patched], &edit(&[], &[1, 2]));
    let f = one(&delta, P);
    // Natural order: line 9 before line 10.
    assert_eq!(
        bcats(f),
        [
            bc(8, 6, "jump to line 9", "branch_kept_hit", (1, 1)),
            bc(8, 6, "jump to line 10", "branch_kept_hit", (1, 1))
        ]
    );
    assert!(!f.changed());
}

#[test]
fn branch_to_changed_target_is_deleted_plus_new() {
    let ids = [
        "jump to line 7",
        "jump to line 8",
        "return from function 'f'",
    ];
    let br: Vec<_> = ids.iter().map(|i| (6, "0", *i, Some(1))).collect();
    let base = fc(P, &[(6, 1), (7, 1), (8, 1)], &br);
    let patched = fc(P, &[(6, 1), (7, 0), (8, 1)], &br);
    let delta = d(&[base], &[patched], &edit(&[7], &[7]));
    assert_eq!(
        bcats(one(&delta, P)),
        [
            bc(6, 6, "jump to line 7", "branch_deleted", (1, N)),
            bc(6, 6, "jump to line 7", "branch_new", (N, 1)),
            bc(6, 6, "jump to line 8", "branch_kept_hit", (1, 1)),
            bc(6, 6, "return from function 'f'", "branch_kept_hit", (1, 1))
        ]
    );
}

#[test]
fn branch_without_counterpart_line_data_is_unmeasured() {
    let base = fc(P, &[(1, 1)], &[(1, "0", "0", Some(1))]);
    let delta = d(&[base], &[lines(&[(1, 1)])], &LineMaps::new());
    assert_eq!(
        bcats(one(&delta, P)),
        [bc(1, 1, "0", "branch_unmeasured", (1, N))]
    );
    assert_eq!(codes(&delta), ["unmeasured"]);
}

// --- files measured on one side only ---

#[test]
fn inferred_base_when_file_exists_in_base_tree() {
    let root = tree(&[P]);
    let patched = fc(P, &[(1, 1), (2, 0)], &[(1, "0", "0", Some(1))]);
    let delta = with_roots(
        &[snap(&[])],
        &[snap(&[patched])],
        &LineMaps::new(),
        Some(root.path()),
        None,
    );
    let f = one(&delta, P);
    assert_eq!(
        (f.inferred_side, f.base_path.as_deref()),
        (Some(Side::Base), Some(P))
    );
    assert_eq!(
        cats(f),
        [(1, 1, "gained", (0, 1)), (2, 2, "kept_miss", (0, 0))]
    );
    assert_eq!(bcats(f), [bc(1, 1, "0", "branch_gained", (0, 1))]);
    assert_eq!(codes(&delta), ["inferred_file"]);
    assert_eq!((f.stats.executable_base, f.stats.hit_base), (2, 0));
}

#[test]
fn inferred_patched_maps_lines_through_the_patch() {
    let root = tree(&[P]);
    let base = fc(P, &[(1, 1), (2, 0)], &[]);
    let delta = with_roots(
        &[snap(&[base])],
        &[snap(&[])],
        &LineMaps::new(),
        None,
        Some(root.path()),
    );
    let f = one(&delta, P);
    assert_eq!(f.inferred_side, Some(Side::Patched));
    assert_eq!(
        cats(f),
        [(1, 1, "lost", (1, 0)), (2, 2, "kept_miss", (0, 0))]
    );

    let base = fc(P, &[(1, 1), (2, 1)], &[(1, "0", "jump to line 2", Some(1))]);
    let delta = with_roots(
        &[snap(&[base])],
        &[snap(&[])],
        &edit(&[], &[1]),
        None,
        Some(root.path()),
    );
    let f = one(&delta, P);
    assert_eq!(cats(f), [(2, 1, "lost", (1, 0)), (3, 2, "lost", (1, 0))]);
    assert_eq!(
        bcats(f),
        [bc(2, 1, "jump to line 3", "branch_lost", (1, 0))]
    );
}

#[test]
fn nothing_executable_means_nothing_inferred() {
    let init = "pkg/__init__.py";
    let root = tree(&[init]);
    let delta = with_roots(
        &[snap(&[])],
        &[snap(&[fc(init, &[], &[])])],
        &LineMaps::new(),
        Some(root.path()),
        None,
    );
    let f = one(&delta, init);
    assert_eq!(
        (f.inferred_side, f.base_path.as_deref()),
        (None, Some(init))
    );
    assert!(f.lines.is_empty() && delta.warnings.is_empty());
}

#[test]
fn inferred_side_has_no_counterpart_for_branches_to_new_lines() {
    let root = tree(&[P]);
    let br = [
        (1, "0", "jump to line 2", Some(1)),
        (1, "0", "jump to line 3", Some(0)),
    ];
    let patched = fc(P, &[(1, 1), (2, 1), (3, 0)], &br);
    let delta = with_roots(
        &[snap(&[])],
        &[snap(&[patched])],
        &edit(&[], &[2, 3]),
        Some(root.path()),
        None,
    );
    let got: Vec<_> = bcats(one(&delta, P)).into_iter().map(|b| b.3).collect();
    assert_eq!(got, ["branch_new", "branch_new"]);
    assert_eq!(codes(&delta), ["inferred_file"]);
}

#[test]
fn missing_from_tree_means_new_or_deleted_file() {
    let empty = tree(&[]);
    let delta = with_roots(
        &[snap(&[])],
        &[snap(&[lines(&[(1, 1), (2, 0)])])],
        &LineMaps::new(),
        Some(empty.path()),
        None,
    );
    let f = one(&delta, P);
    assert_eq!((f.base_path.as_deref(), f.inferred_side), (None, None));
    assert_eq!(
        cats(f),
        [(1, N, "new_hit", (N, 1)), (2, N, "new_miss", (N, 0))]
    );
    assert!(delta.warnings.is_empty());

    let delta = with_roots(
        &[snap(&[lines(&[(1, 1)])])],
        &[snap(&[])],
        &LineMaps::new(),
        None,
        Some(empty.path()),
    );
    let f = one(&delta, P);
    assert_eq!((f.path.as_str(), f.base_path.as_deref()), (P, Some(P)));
    assert_eq!(cats(f), [(N, 1, "deleted_hit", (1, N))]);
    assert!(f.source_changed);
}

#[test]
fn root_unknown_uses_patch_for_new_and_deleted_files() {
    let delta = d(&[], &[lines(&[(1, 0)])], &lmap(None, Some(P), &[], &[]));
    let f = one(&delta, P);
    assert_eq!(f.base_path, None);
    assert_eq!(cats(f), [(1, N, "new_miss", (N, 0))]);
    assert!(delta.warnings.is_empty());

    let delta = d(&[lines(&[(1, 1)])], &[], &lmap(Some(P), None, &[], &[]));
    assert_eq!(cats(one(&delta, P)), [(N, 1, "deleted_hit", (1, N))]);
    assert!(delta.warnings.is_empty());
}

#[test]
fn root_unknown_without_patch_info_is_unmeasured() {
    let delta = d(&[], &[lines(&[(1, 1), (2, 0)])], &LineMaps::new());
    let f = one(&delta, P);
    assert_eq!(f.inferred_side, None);
    assert!(
        f.lines
            .iter()
            .all(|e| e.category == LineCategory::Unmeasured)
    );
    assert_eq!(codes(&delta), ["unmeasured"]);
    let msg = &delta.warnings[0].message;
    assert!(
        msg.contains("not in base coverage and the base tree is unknown"),
        "{msg}"
    );

    let delta = d(&[fc("pkg/__init__.py", &[], &[])], &[], &LineMaps::new());
    assert!(one(&delta, "pkg/__init__.py").lines.is_empty());
    assert!(delta.warnings.is_empty());
}

// --- flakiness ---

#[test]
fn flaky_base_line_is_excluded_from_lost() {
    let base = [
        snap(&[lines(&[(1, 1), (2, 1)])]),
        snap(&[lines(&[(1, 1), (2, 0)])]),
    ];
    let patched = [snap(&[lines(&[(1, 0), (2, 0)])])];
    let delta = diff(&base, &patched, &LineMaps::new(), DiffOptions::default()).unwrap();
    let f = one(&delta, P);
    assert_eq!(cats(f), [(1, 1, "lost", (1, 0))]);
    let fl = &f.flaky[0];
    assert_eq!(
        (fl.line, fl.base_line, fl.hits),
        (Some(2), Some(2), (Some(1), Some(0)))
    );
    assert_eq!(fl.sides, [Side::Base]);
    assert_eq!((f.stats.lost, f.stats.hit_base), (1, 2));
}

#[test]
fn flaky_patched_line_and_missing_file_in_a_repeat() {
    let base = [snap(&[lines(&[(1, 0)])])];
    let patched = [snap(&[lines(&[(1, 1)])]), snap(&[])];
    let delta = diff(&base, &patched, &LineMaps::new(), DiffOptions::default()).unwrap();
    let f = one(&delta, P);
    assert!(f.lines.is_empty() && f.changed());
    assert_eq!(
        (f.flaky[0].line, &f.flaky[0].sides[..]),
        (Some(1), &[Side::Patched][..])
    );
}

#[test]
fn flaky_branch() {
    let br = |a, b| {
        fc(
            P,
            &[(1, 1)],
            &[(1, "0", "0", Some(a)), (1, "0", "1", Some(b))],
        )
    };
    let base = [snap(&[br(1, 1)]), snap(&[br(0, 1)])];
    let delta = diff(
        &base,
        &[snap(&[br(0, 0)])],
        &LineMaps::new(),
        DiffOptions::default(),
    )
    .unwrap();
    let got: Vec<_> = one(&delta, P)
        .branches
        .iter()
        .map(|b| (b.branch.as_str(), b.category.as_str()))
        .collect();
    assert_eq!(got, [("0", "branch_flaky"), ("1", "branch_lost")]);
}

// --- pairing, renames, external files ---

#[test]
fn rename_pairs_old_and_new_path() {
    let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[], &[]);
    let delta = d(
        &[fc("pkg/old.py", &[(1, 1), (2, 1)], &[])],
        &[fc("pkg/new.py", &[(1, 1), (2, 0)], &[])],
        &lms,
    );
    let [f] = &delta.files[..] else { panic!() };
    assert_eq!(
        (f.path.as_str(), f.base_path.as_deref()),
        ("pkg/new.py", Some("pkg/old.py"))
    );
    assert_eq!(
        cats(f),
        [(1, 1, "kept_hit", (1, 1)), (2, 2, "lost", (1, 0))]
    );
    assert!(!f.source_changed);
}

#[test]
fn rename_with_edits() {
    let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[2], &[2, 3]);
    let delta = d(
        &[fc("pkg/old.py", &[(1, 1), (2, 1), (3, 0)], &[])],
        &[fc("pkg/new.py", &[(1, 1), (2, 0), (3, 1), (4, 1)], &[])],
        &lms,
    );
    let [f] = &delta.files[..] else { panic!() };
    assert_eq!(
        cats(f),
        [
            (1, 1, "kept_hit", (1, 1)),
            (2, N, "new_miss", (N, 0)),
            (3, N, "new_hit", (N, 1)),
            (4, 3, "gained", (0, 1)),
            (N, 2, "deleted_hit", (1, N))
        ]
    );
    assert!(f.source_changed);
}

#[test]
fn rename_with_only_base_measured() {
    let root = tree(&["pkg/new.py"]);
    let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[], &[]);
    let base = [snap(&[fc("pkg/old.py", &[(1, 1)], &[])])];
    let delta = with_roots(&base, &[snap(&[])], &lms, None, Some(root.path()));
    let [f] = &delta.files[..] else { panic!() };
    let got = (f.path.as_str(), f.base_path.as_deref(), f.inferred_side);
    assert_eq!(got, ("pkg/new.py", Some("pkg/old.py"), Some(Side::Patched)));
    assert_eq!(cats(f), [(1, 1, "lost", (1, 0))]);
}

#[test]
fn patch_says_deleted_but_snapshot_has_it() {
    let delta = d(
        &[lines(&[(1, 1)])],
        &[lines(&[(1, 0)])],
        &lmap(Some(P), None, &[], &[]),
    );
    assert_eq!(cats(one(&delta, P)), [(1, 1, "lost", (1, 0))]);
}

#[test]
fn colliding_base_file_is_ignored_with_warning() {
    let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[], &[]);
    let delta = d(
        &[
            fc("pkg/old.py", &[(1, 1)], &[]),
            fc("pkg/new.py", &[(1, 1)], &[]),
        ],
        &[fc("pkg/new.py", &[(1, 1)], &[])],
        &lms,
    );
    let got: Vec<_> = delta
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.base_path.as_deref()))
        .collect();
    assert_eq!(got, [("pkg/new.py", Some("pkg/old.py"))]);
    assert_eq!(codes(&delta), ["unpaired_file"]);
}

#[test]
fn external_files_are_counted_not_listed() {
    let mut ext = fc("/usr/lib/python3/os.py", &[(1, 1)], &[]);
    ext.external = true;
    let files = [ext, lines(&[(1, 1)])];
    let delta = d(&files, &files, &LineMaps::new());
    assert_eq!(delta.external_files, 1);
    assert_eq!(
        delta
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        [P]
    );
    let children = delta.tree.children.as_ref().unwrap();
    assert_eq!(
        children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["pkg"]
    );
}

#[test]
fn files_sorted_and_tree_covers_unchanged_files() {
    let files = ["b.py", "a/z.py", "a/c.py"].map(|p| fc(p, &[(1, 1)], &[]));
    let delta = d(&files, &files, &LineMaps::new());
    let paths: Vec<_> = delta.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["a/c.py", "a/z.py", "b.py"]);
    let children = delta.tree.children.as_ref().unwrap();
    assert_eq!(
        children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["a", "b.py"]
    );
    assert_eq!(delta.tree.stats.executable_base, 3);
}

// --- sha256 and source_changed ---

#[test]
fn sha256_from_roots() {
    let (b, p) = (tree(&[P]), tree(&[P]));
    let run = || {
        let s = [snap(&[lines(&[(1, 1)])])];
        with_roots(&s, &s, &LineMaps::new(), Some(b.path()), Some(p.path()))
    };
    let delta = run();
    let f = one(&delta, P);
    let digest = crate::sha256_file(&b.path().join(P)).unwrap();
    assert_eq!(f.sha256, (Some(digest.clone()), Some(digest)));
    assert!(!f.source_changed);
    std::fs::write(p.path().join(P), "changed\n").unwrap();
    assert!(one(&run(), P).source_changed);
}

#[test]
fn source_changed_from_linemap_when_roots_unknown() {
    let delta = d(&[lines(&[(1, 1)])], &[lines(&[(2, 1)])], &edit(&[], &[1]));
    let f = one(&delta, P);
    assert_eq!(f.sha256, (None, None));
    assert!(f.source_changed);
}

// --- test outcomes ---

#[test]
fn outcomes() {
    let o = |items: &[(&str, Outcome)]| -> Outcomes {
        items.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    };
    let base = o(&[("t::a", Outcome::Passed), ("t::b", Outcome::Passed)]);
    let patched = o(&[("t::a", Outcome::Skipped), ("t::c", Outcome::Failed)]);
    let opts = DiffOptions {
        base_outcomes: Some(&base),
        patched_outcomes: Some(&patched),
        ..Default::default()
    };
    let delta = diff(&[snap(&[])], &[snap(&[])], &LineMaps::new(), opts).unwrap();
    let got: Vec<_> = delta
        .tests
        .iter()
        .map(|t| (t.id.as_str(), t.base.as_str(), t.patched.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("t::a", "passed", "skipped"),
            ("t::b", "passed", "not_run"),
            ("t::c", "not_run", "failed")
        ]
    );

    assert!(d(&[], &[], &LineMaps::new()).tests.is_empty());
    let opts = DiffOptions {
        base_outcomes: Some(&base),
        ..Default::default()
    };
    let delta = diff(&[snap(&[])], &[snap(&[])], &LineMaps::new(), opts).unwrap();
    assert!(delta.tests.is_empty());
    assert_eq!(codes(&delta), ["tests_one_side"]);
}

// --- helpers ---

#[test]
fn natural_order_and_remap() {
    assert_eq!(natural_cmp("line 9", "line 10"), Ordering::Less);
    assert_eq!(natural_cmp("007", "7"), Ordering::Equal);
    assert_eq!(natural_cmp("a", "a1"), Ordering::Less);
    let shift = |l: LineNo| (l != 5).then_some(l + 1);
    assert_eq!(
        remap_branch_id("jump to line 7", &shift).as_deref(),
        Some("jump to line 8")
    );
    assert_eq!(remap_branch_id("line 5", &shift), None);
    assert_eq!(remap_branch_id("0", &shift).as_deref(), Some("0"));
    assert_eq!(
        remap_branch_id("baseline 7", &shift).as_deref(),
        Some("baseline 7")
    );
}

// --- real coverage.py output ---

#[test]
fn real_skip_divide() {
    let fix = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let read = |p: &str| std::fs::read_to_string(fix.join(p)).unwrap();
    let run = "shed_py_runs/skip_divide";
    let patch = crate::patch::parse_patch(&read("patches/skip_divide.patch")).unwrap();
    let base_out = crate::junit::parse_junit(&read(&format!("{run}/base.xml"))).unwrap();
    let patched_out = crate::junit::parse_junit(&read(&format!("{run}/patched.xml"))).unwrap();
    let shed = fix.join("shed_py");
    let opts = DiffOptions {
        base_root: Some(&shed),
        patched_root: Some(&shed),
        base_outcomes: Some(&base_out),
        patched_outcomes: Some(&patched_out),
    };
    let base = crate::lcov::parse(&read(&format!("{run}/base.lcov"))).unwrap();
    let patched = crate::lcov::parse(&read(&format!("{run}/patched.lcov"))).unwrap();
    let delta = diff(
        &[base],
        &[patched],
        &crate::linemap::build_linemaps(&patch),
        opts,
    )
    .unwrap();
    let changed_tests: Vec<_> = delta
        .tests
        .iter()
        .filter(|t| t.base != t.patched)
        .map(|t| (t.id.as_str(), t.base.as_str(), t.patched.as_str()))
        .collect();
    assert_eq!(
        changed_tests,
        [("tests.test_calc::test_divide", "passed", "skipped")]
    );
    let calc = "shed/calc.py";
    let changed: Vec<_> = delta
        .files
        .iter()
        .filter(|f| f.changed())
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(changed, [calc]);
    let f = one(&delta, calc);
    assert!(!f.source_changed);
    assert_eq!(
        f.sha256.0,
        Some(crate::sha256_file(&shed.join(calc)).unwrap())
    );
    let lost: Vec<_> = cats(f)
        .into_iter()
        .filter(|e| !e.2.starts_with("kept_"))
        .collect();
    assert_eq!(lost, [6, 7, 8].map(|l| (l, l, "lost", (1, 0))));
    let br: Vec<_> = bcats(f)
        .into_iter()
        .filter(|b| b.3 != "branch_kept_hit")
        .collect();
    assert_eq!(
        br,
        [
            bc(6, 6, "jump to line 7", "branch_lost", (1, N)),
            bc(6, 6, "jump to line 8", "branch_lost", (1, N))
        ]
    );
    let funcs: Vec<_> = f
        .functions
        .iter()
        .map(|g| (g.name.as_str(), g.status.as_str(), g.start, g.end, g.lost))
        .collect();
    assert_eq!(
        funcs,
        [
            ("add", "unchanged", 2, 3, 0),
            ("divide", "went_dark", 5, 8, 3),
            ("classify", "unchanged", 10, 15, 0)
        ]
    );
    assert!(delta.warnings.is_empty());
}

#[test]
fn functions_from_lcov_fn_records() {
    let fns = vec![
        FunctionRecord {
            name: "f".into(),
            start: 1,
            end: Some(4),
            hits: Some(1),
        },
        FunctionRecord {
            name: "g".into(),
            start: 6,
            end: None,
            hits: Some(1),
        },
    ];
    let mut base = lines(&[(1, 1), (2, 1), (3, 1), (4, 1), (6, 1), (7, 1)]);
    let mut patched = lines(&[(1, 1), (2, 1), (3, 0), (4, 1), (6, 1), (7, 0)]);
    base.functions = fns.clone();
    patched.functions = fns;
    let delta = d(&[base], &[patched], &LineMaps::new());
    let got: Vec<_> = delta.files[0]
        .functions
        .iter()
        .map(|g| (g.name.as_str(), g.status.as_str(), g.start, g.end, g.lost))
        .collect();
    // The FN line is the signature: g's body is line 7 only, so g went dark.
    assert_eq!(
        got,
        [("f", "partial_loss", 1, 4, 1), ("g", "went_dark", 6, 7, 1)]
    );
}
