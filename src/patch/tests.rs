use super::*;

const HEADER: [&str; 4] = [
    "diff --git a/f.py b/f.py",
    "index 1111111..2222222 100644",
    "--- a/f.py",
    "+++ b/f.py",
];

fn load(name: &str) -> Patch {
    let path = format!(
        "{}/tests/fixtures/patches/unit/{name}.patch",
        env!("CARGO_MANIFEST_DIR")
    );
    parse_patch(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn text(lines: &[&str]) -> String {
    lines.join("\n") + "\n"
}

fn with_header(body: &[&str]) -> String {
    text(&[&HEADER[..], body].concat())
}

fn headers(patch: &Patch) -> Vec<[LineNo; 4]> {
    let hunks = patch.files.iter().flat_map(|f| &f.hunks);
    hunks
        .map(|h| [h.old_start, h.old_count, h.new_start, h.new_count])
        .collect()
}

fn paths(patch: &Patch) -> Vec<(Option<&str>, Option<&str>)> {
    let files = patch.files.iter();
    files
        .map(|f| (f.old_path.as_deref(), f.new_path.as_deref()))
        .collect()
}

fn fp(old: Option<&str>, new: Option<&str>, binary: bool) -> FilePatch {
    FilePatch {
        old_path: old.map(Into::into),
        new_path: new.map(Into::into),
        hunks: vec![],
        binary,
    }
}

#[test]
fn single_hunk() {
    let patch = load("single_hunk");
    assert!(patch.warnings.is_empty());
    assert_eq!(
        paths(&patch),
        [(Some("shed/calc.py"), Some("shed/calc.py"))]
    );
    assert!(!patch.files[0].binary);
    assert_eq!(headers(&patch), [[7, 9, 7, 10]]);
    let lines = &patch.files[0].hunks[0].lines;
    assert_eq!(
        lines[..5],
        [" x7 = 7", " x8 = 8", " x9 = 9", "-x10 = 10", "+x10 = 100"]
    );
    assert_eq!(lines.last().unwrap(), " x15 = 15");
}

#[test]
fn multi_hunk() {
    let patch = load("multi_hunk");
    assert_eq!(
        headers(&patch),
        [[1, 6, 1, 6], [12, 7, 12, 6], [25, 6, 24, 8]]
    );
    assert_eq!(patch.files[0].hunks[1].lines[3], "-x15 = 15");
}

#[test]
fn pure_add_and_delete() {
    let marks = |p: Patch| -> String {
        let lines = &p.files[0].hunks[0].lines;
        lines.iter().map(|l| &l[..1]).collect()
    };
    assert_eq!(marks(load("pure_add")), "   ++   ");
    assert_eq!(marks(load("pure_delete")), "   --   ");
}

#[test]
fn new_and_deleted_file() {
    let patch = load("new_file");
    assert_eq!(paths(&patch), [(None, Some("shed/util.py"))]);
    assert_eq!(patch.files[0].path(), "shed/util.py");
    assert_eq!(headers(&patch), [[0, 0, 1, 3]]);
    assert_eq!(
        patch.files[0].hunks[0].lines,
        ["+def util():", "+    return 42", "+"]
    );
    let patch = load("deleted_file");
    assert_eq!(paths(&patch), [(Some("shed/old.py"), None)]);
    assert_eq!(patch.files[0].path(), "shed/old.py");
    assert_eq!(headers(&patch), [[1, 5, 0, 0]]);
}

#[test]
fn renames() {
    let patch = load("rename_with_edits");
    assert_eq!(
        paths(&patch),
        [(Some("shed/calc.py"), Some("shed/core.py"))]
    );
    assert_eq!(headers(&patch), [[7, 7, 7, 7]]);
    assert_eq!(patch.touched_paths(), ["shed/calc.py", "shed/core.py"]);
    let patch = load("pure_rename");
    assert_eq!(
        patch.files,
        [fp(Some("shed/old.py"), Some("shed/legacy.py"), false)]
    );
}

#[test]
fn no_newline_marker_dropped() {
    let patch = load("no_newline_eof");
    assert_eq!(
        patch.files[0].hunks[0].lines,
        [" p = 1", "-q = 2", "+q = 2"]
    );
    assert_eq!(
        patch.files[1].hunks[0].lines,
        [" a = 1", " b = 2", "-c = 3", "+c = 3", "+d = 4"]
    );
    let body = [
        "@@ -1 +1,2 @@",
        "-a",
        "\\ No newline at end of file",
        "+a",
        "+b",
    ];
    let patch = parse_patch(&with_header(&body)).unwrap();
    assert_eq!(patch.files[0].hunks[0].lines, ["-a", "+a", "+b"]);
}

#[test]
fn binary_diffs() {
    let patch = load("binary");
    assert_eq!(headers(&patch), [[1, 4, 1, 4]]);
    let logo = fp(Some("shed/logo.png"), Some("shed/logo.png"), true);
    assert_eq!(patch.files[1], logo);
    assert_eq!(patch.warnings, ["shed/logo.png: binary diff ignored"]);

    let patch = load("git_binary");
    assert_eq!(patch.files[0], fp(None, Some("shed/blob.bin"), true));
    let calc = &patch.files[1].hunks[0].lines;
    assert_eq!(calc[calc.len() - 2..], ["-x30 = 30", "+x30 = 3000"]);
    assert_eq!(patch.files[2], logo);
    assert_eq!(
        patch.warnings,
        [
            "shed/blob.bin: binary diff ignored",
            "shed/logo.png: binary diff ignored"
        ]
    );
}

#[test]
fn multiple_files() {
    let patch = load("multi_file");
    let got: Vec<_> = patch
        .files
        .iter()
        .map(|f| (f.old_path.as_deref(), f.new_path.as_deref(), f.hunks.len()))
        .collect();
    let want = [
        (Some("shed/calc.py"), Some("shed/calc.py"), 1),
        (Some("shed/old.py"), Some("shed/old.py"), 1),
        (None, Some("shed/py.typed"), 0), // empty new file
        (Some("shed/run.sh"), Some("shed/run.sh"), 0), // mode change only
        (None, Some("shed/util.py"), 1),
    ];
    assert_eq!(got, want);
}

#[test]
fn format_patch_preamble_skipped() {
    let patch = load("format_patch");
    assert_eq!(patch.files.len(), 1);
    assert_eq!(patch.files[0].path(), "shed/calc.py");
    assert_eq!(headers(&patch), [[2, 7, 2, 7]]);
    assert!(patch.warnings.is_empty());
}

#[test]
fn quoted_and_spaced_paths() {
    let patch = load("quoted_paths");
    assert_eq!(
        paths(&patch),
        [
            (Some("shed/café.py"), Some("shed/café.py")),
            (Some("shed/foo bar.py"), Some("shed/foo bar.py")), // trailing tab stripped
            (Some("shed/run.sh"), Some("shed/run\tme.sh")),
        ]
    );
    assert_eq!(headers(&patch)[0], [1, 1, 1, 1]); // counts default to 1
}

#[test]
fn plain_diffs() {
    let patch = load("plain_diff");
    assert_eq!(paths(&patch), [(Some("calc.py"), Some("calc.py"))]);
    assert_eq!(
        patch.files[0].hunks[0].lines[3..5],
        ["-x20 = 20", "+x20 = 200"]
    );
    let added = text(&["--- /dev/null", "+++ b/n.py", "@@ -0,0 +1 @@", "+n"]);
    let deleted = text(&["--- a/d.py", "+++ /dev/null", "@@ -1 +0,0 @@", "-d"]);
    let patch = parse_patch(&(added + &deleted)).unwrap();
    assert_eq!(paths(&patch), [(None, Some("n.py")), (Some("d.py"), None)]);
}

#[test]
fn plain_diff_never_renames() {
    // What `git apply --numstat` reports for each pair.
    for (minus, plus, path) in [
        ("calc.py.orig", "calc.py", "calc.py"),
        ("calc.py", "calc.py.new", "calc.py"),
        ("a/x.py", "b/y.py", "y.py"),
        ("old/shed/calc.py", "new/shed/calc.py", "shed/calc.py"),
    ] {
        let patch = parse_patch(&text(&[
            &format!("--- {minus}\t2026-01-01"),
            &format!("+++ {plus}\t2026-01-01"),
            "@@ -1 +1 @@",
            "-a",
            "+b",
        ]))
        .unwrap();
        assert_eq!(paths(&patch), [(Some(path), Some(path))], "{minus} {plus}");
    }
}

#[test]
fn empty_and_text_only_input() {
    assert_eq!(parse_patch("").unwrap(), Patch::default());
    assert!(parse_patch(" \u{3000}\n").unwrap().warnings.is_empty());
    assert_eq!(
        parse_patch("just a commit message\n").unwrap().warnings,
        ["no file diffs found in patch"]
    );
}

#[test]
fn body_lines_follow_header_counts() {
    // A deleted "-- x" line reads as "--- x"; an empty line is context with its space
    // stripped.
    let body = ["@@ -1,3 +1,2 @@", "--- x", "", " y", "-- ", "2.53.0"];
    let patch = parse_patch(&with_header(&body)).unwrap();
    assert_eq!(patch.files[0].hunks[0].lines, ["--- x", " ", " y"]);
}

#[test]
fn crlf() {
    let patch = parse_patch("--- a/w.py\r\n+++ b/w.py\r\n@@ -1 +1 @@\r\n-a\r\n+b\r\n").unwrap();
    assert_eq!(patch.files[0].path(), "w.py");
    assert_eq!(patch.files[0].hunks[0].lines, ["-a\r", "+b\r"]);
    let binary = [
        "diff --git a/x.bin b/x.bin",
        "index 1..2 100644",
        "GIT binary patch",
        "literal 3",
        "K",
    ];
    let patch = parse_patch(&(binary.join("\r\n") + "\r\n")).unwrap();
    assert_eq!(patch.files, [fp(Some("x.bin"), Some("x.bin"), true)]);
    let empty_context = "--- a/f\r\n+++ b/f\r\n@@ -1,2 +1,2 @@\r\n\r\n-a\r\n+b\r\n";
    let patch = parse_patch(empty_context).unwrap();
    assert_eq!(patch.files[0].hunks[0].lines, [" ", "-a\r", "+b\r"]);
}

#[test]
fn offset_header_warns() {
    let body = ["@@ -1 +1,2 @@", " a", "+b", "@@ -10 +10 @@", " c"];
    assert_eq!(
        parse_patch(&with_header(&body)).unwrap().warnings,
        ["f.py: hunk at line 8 is offset from its header"]
    );
}

#[test]
fn git_header_strips_any_prefix() {
    // Like git apply -p1: the first path component goes, whatever it is.
    for (header, path) in [
        ("diff --git i/f.py w/f.py", "f.py"), // diff.mnemonicPrefix
        ("diff --git old/foo bar.py new/foo bar.py", "foo bar.py"),
        ("diff --git f.py f.py", "f.py"), // --no-prefix, top-level file
        (
            "diff --git \"i/caf\\303\\251.py\" \"w/caf\\303\\251.py\"",
            "café.py",
        ),
    ] {
        let patch = parse_patch(&text(&[header, "old mode 100644", "new mode 100755"]));
        assert_eq!(patch.unwrap().files, [fp(Some(path), Some(path), false)]);
    }
    let lines = [
        "diff --git i/s/f.py w/s/f.py",
        "--- i/s/f.py",
        "+++ w/s/f.py",
        "@@ -1 +1 @@",
        "-a",
        "+b",
    ];
    assert_eq!(
        parse_patch(&text(&lines)).unwrap().touched_paths(),
        ["s/f.py"]
    );
}

#[test]
fn quoted_bytes_that_are_not_utf8() {
    let patch = parse_patch("diff --git \"a/\\377\" \"b/\\377\"\nold mode 1\n").unwrap();
    assert_eq!(patch.files[0].path(), "\u{fffd}");
}

#[test]
fn unknown_header_lines_skipped() {
    let lines = [&HEADER[..1], &["x-future-header: 1"], &HEADER[1..]].concat();
    let patch = parse_patch(&text(&[&lines[..], &["@@ -1 +1 @@", "-a", "+b"]].concat()));
    let patch = patch.unwrap();
    assert_eq!(patch.files.len(), 1);
    assert_eq!(patch.files[0].hunks.len(), 1);
}

#[test]
fn copy_warns() {
    let patch = parse_patch(&text(&[
        "diff --git a/s.py b/t.py",
        "similarity index 100%",
        "copy from s.py",
        "copy to t.py",
    ]))
    .unwrap();
    assert_eq!(patch.files, [fp(Some("s.py"), Some("t.py"), false)]);
    assert_eq!(patch.warnings, ["t.py: copy of s.py treated as a rename"]);
}

#[test]
fn binary_line_split_is_greedy() {
    assert_eq!(
        match_binary("Binary files a and b and c differ"),
        Some(("a and b", "c"))
    );
    assert_eq!(match_binary("Binary files  and x differ"), None);
}

#[test]
fn malformed_hunks() {
    for (body, message) in [
        (
            &["@@ -1,2 +1,x @@", " a"][..],
            "line 5: malformed hunk header",
        ),
        (
            &["@@ -1,3 +1,3 @@", " a", " b"],
            "line 7: patch ends inside a hunk",
        ),
        (
            &["@@ -1 +1 @@", "-a", "+b", "+c"],
            "line 8: hunk has more lines than its header says",
        ),
        (
            &["@@ -1 +1 @@", "-a", "+b", " c"],
            "line 8: hunk has more lines than its header says",
        ),
        (
            &["@@ -1 +1 @@", "-a", "-b", "+c"],
            "line 7: hunk has more lines than its header says",
        ),
        (
            &["@@ -1 +1 @@", "?a", "+b"],
            "line 6: unexpected line in hunk",
        ),
        (
            &["@@ -0,1 +1 @@", "-a", "+b"],
            "line 5: hunk range starts at line 0",
        ),
        (
            &["@@ -5 +5 @@", " a", "@@ -3 +3 @@", " b"],
            "line 7: hunk overlaps or precedes the previous one",
        ),
    ] {
        assert_eq!(parse_patch(&with_header(body)).unwrap_err(), message);
    }
}

#[test]
fn malformed_files() {
    let new_with_old = [
        &["diff --git a/f.py b/f.py", "new file mode 100644"][..],
        &HEADER[2..],
        &["@@ -1 +1 @@", "-a", "+b"],
    ]
    .concat();
    for (lines, message) in [
        (
            &["diff --git a/f.py b/f.py", "--- a/f.py", "@@ -1 +1 @@"][..],
            "line 2: '---' line not",
        ),
        (
            &["diff --cc f.py"],
            "line 1: combined diffs are not supported",
        ),
        (&new_with_old, "line 1: new file has a hunk with old lines"),
        (
            &["diff --git \"a/f.py b/f.py", "rename from \"x\\q\""],
            "line 2: bad quoted path",
        ),
        (
            &["--- /dev/null", "+++ /dev/null", "@@ -0,0 +0,0 @@"],
            "line 1: both sides are",
        ),
        (
            &["diff --git a/x b/y b/z", "old mode 100644"],
            "line 1: cannot determine file path",
        ),
        (
            &["diff --git a/f.py b/f.py", "index 1..2", "@@ -1 +1 @@"],
            "line 3: hunk without '---'",
        ),
    ] {
        let err = parse_patch(&text(lines)).unwrap_err();
        assert!(err.starts_with(message), "{err}");
    }
}

#[test]
fn patch_series_rejected() {
    let once = with_header(&["@@ -1 +1 @@", "-a", "+b"]);
    let err = parse_patch(&(once.clone() + &once)).unwrap_err();
    assert!(err.starts_with("line 8: f.py is patched twice"), "{err}");
}
