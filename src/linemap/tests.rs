use super::*;
use crate::patch::{FilePatch, parse_patch};

const NAMES: [&str; 15] = [
    "binary",
    "deleted_file",
    "format_patch",
    "git_binary",
    "multi_file",
    "multi_hunk",
    "new_file",
    "no_newline_eof",
    "plain_diff",
    "pure_add",
    "pure_delete",
    "pure_rename",
    "quoted_paths",
    "rename_with_edits",
    "single_hunk",
];

fn maps(name: &str) -> LineMaps {
    let path = format!(
        "{}/tests/fixtures/patches/unit/{name}.patch",
        env!("CARGO_MANIFEST_DIR")
    );
    build_linemaps(&parse_patch(&std::fs::read_to_string(path).unwrap()).unwrap())
}

fn range(lo: LineNo, hi: LineNo) -> Vec<Option<LineNo>> {
    (lo..hi).map(Some).collect()
}

fn cat(parts: &[Vec<Option<LineNo>>]) -> Vec<Option<LineNo>> {
    parts.concat()
}

#[test]
fn changed_lines() {
    for (name, path, deleted, added) in [
        ("single_hunk", "shed/calc.py", &[10][..], &[10, 13][..]),
        ("multi_hunk", "shed/calc.py", &[3, 15], &[3, 27, 28]),
        ("pure_add", "shed/calc.py", &[], &[6, 7]),
        ("pure_delete", "shed/calc.py", &[8, 9], &[]),
        ("new_file", "shed/util.py", &[], &[1, 2, 3]),
        ("deleted_file", "shed/old.py", &[1, 2, 3, 4, 5], &[]),
        ("rename_with_edits", "shed/core.py", &[10], &[10]),
        ("no_newline_eof", "shed/tail.py", &[3], &[3, 4]),
        ("no_newline_eof", "shed/nonl.py", &[2], &[2]),
        ("format_patch", "shed/calc.py", &[5], &[5]),
        ("plain_diff", "calc.py", &[20], &[20]),
    ] {
        let lm = &maps(name)[path];
        assert_eq!((&lm.deleted[..], &lm.added[..]), (deleted, added), "{name}");
    }
}

#[test]
fn every_line_of_calc() {
    let n = || vec![None];
    for (name, to_new, to_old) in [
        (
            "single_hunk",
            cat(&[range(1, 10), n(), range(11, 13), range(14, 32)]),
            cat(&[range(1, 10), n(), range(11, 13), n(), range(13, 31)]),
        ),
        (
            "multi_hunk",
            cat(&[
                range(1, 3),
                n(),
                range(4, 15),
                n(),
                range(15, 27),
                range(29, 32),
            ]),
            cat(&[
                range(1, 3),
                n(),
                range(4, 15),
                range(16, 28),
                n(),
                n(),
                range(28, 31),
            ]),
        ),
        (
            "pure_add",
            cat(&[range(1, 6), range(8, 33)]),
            cat(&[range(1, 6), n(), n(), range(6, 31)]),
        ),
        (
            "pure_delete",
            cat(&[range(1, 8), n(), n(), range(8, 29)]),
            cat(&[range(1, 8), range(10, 31)]),
        ),
    ] {
        let lm = &maps(name)["shed/calc.py"];
        let got: Vec<_> = (1..=to_new.len() as LineNo).map(|l| lm.to_new(l)).collect();
        assert_eq!(got, to_new, "{name}");
        let got: Vec<_> = (1..=to_old.len() as LineNo).map(|l| lm.to_old(l)).collect();
        assert_eq!(got, to_old, "{name}");
    }
}

#[test]
fn new_and_deleted_file() {
    let lm = &maps("new_file")["shed/util.py"];
    assert_eq!(
        (lm.old_path.as_deref(), lm.new_path.as_deref()),
        (None, Some("shed/util.py"))
    );
    assert!((1..=3).all(|l| lm.to_old(l).is_none()));
    let lm = &maps("deleted_file")["shed/old.py"];
    assert_eq!(
        (lm.old_path.as_deref(), lm.new_path.as_deref()),
        (Some("shed/old.py"), None)
    );
    assert!((1..=5).all(|l| lm.to_new(l).is_none()));
}

#[test]
fn renames_keyed_by_both_paths() {
    let m = maps("rename_with_edits");
    assert_eq!(m["shed/calc.py"], m["shed/core.py"]);
    let lm = &m["shed/core.py"];
    assert_eq!(lm.old_path.as_deref(), Some("shed/calc.py"));
    assert_eq!(
        (lm.to_new(9), lm.to_new(10), lm.to_new(30), lm.to_old(10)),
        (Some(9), None, Some(30), None)
    );
    let m = maps("pure_rename");
    assert_eq!(
        m.keys().collect::<Vec<_>>(),
        ["shed/legacy.py", "shed/old.py"]
    );
    let lm = &m["shed/old.py"];
    assert!(!lm.is_identity());
    assert_eq!((lm.to_new(7), lm.to_old(7)), (Some(7), Some(7)));
}

#[test]
fn binary_and_mode_only_are_identity_shaped() {
    assert_eq!(
        maps("binary")["shed/logo.png"],
        LineMap::identity("shed/logo.png")
    );
    let m = maps("multi_file");
    assert!(m["shed/run.sh"].is_identity());
    let typed = &m["shed/py.typed"];
    assert_eq!(typed.old_path, None);
    assert!(typed.added.is_empty());
}

#[test]
fn quoted_paths_are_keys() {
    let m = maps("quoted_paths");
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "shed/café.py",
            "shed/foo bar.py",
            "shed/run\tme.sh",
            "shed/run.sh"
        ]
    );
}

#[test]
fn linemap_for_falls_back_to_identity() {
    let m = maps("single_hunk");
    assert!(matches!(linemap_for(&m, "shed/calc.py"), Cow::Borrowed(_)));
    assert_eq!(
        *linemap_for(&m, "shed/other.py"),
        LineMap::identity("shed/other.py")
    );
}

#[test]
fn swap_new_path_wins() {
    // a.py -> b.py and b.py -> a.py: each key resolves to the map that produces it.
    let fp = |old: &str, new: &str| FilePatch {
        old_path: Some(old.into()),
        new_path: Some(new.into()),
        hunks: vec![],
        binary: false,
    };
    let patch = Patch {
        files: vec![fp("a.py", "b.py"), fp("b.py", "a.py")],
        warnings: vec![],
    };
    let m = build_linemaps(&patch);
    assert_eq!(m["a.py"].old_path.as_deref(), Some("b.py"));
    assert_eq!(m["b.py"].old_path.as_deref(), Some("a.py"));
}

#[test]
fn offset_hunk_placed_by_old_side() {
    // The second header claims +10, but git apply puts old line 10 at 11.
    let text = "--- a/f.py\n+++ b/f.py\n@@ -1 +1,2 @@\n a\n+b\n@@ -10 +10 @@\n c\n";
    let lm = &build_linemaps(&parse_patch(text).unwrap())["f.py"];
    assert_eq!(
        (lm.to_new(10), lm.to_old(11), lm.to_old(10)),
        (Some(11), Some(10), Some(9))
    );
}

#[test]
fn shifts_around_hunks() {
    let text = "--- a/f\n+++ b/f\n@@ -2 +2,2 @@\n-b\n+B\n+C\n@@ -5,0 +7 @@\n+x\n";
    let lm = &build_linemaps(&parse_patch(text).unwrap())["f"];
    let got: Vec<_> = (1..=7).map(|l| lm.to_new(l)).collect();
    assert_eq!(
        got,
        [Some(1), None, Some(4), Some(5), Some(6), Some(8), Some(9)]
    );
    assert_eq!((lm.to_old(7), lm.to_old(8)), (None, Some(6)));
}

#[test]
fn explicit_pairs_match_counting_rule() {
    for name in NAMES {
        for lm in maps(name).values() {
            let bare = LineMap {
                old_to_new: BTreeMap::new(),
                ..lm.clone()
            };
            for (&old, &new) in &lm.old_to_new {
                assert_eq!((bare.to_new(old), lm.to_old(new)), (Some(new), Some(old)));
            }
        }
    }
}
