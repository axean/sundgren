from __future__ import annotations

from pathlib import Path

import pytest

from sundgren.core.patch import FilePatch, Hunk, Patch, parse_patch

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures" / "patches" / "unit"
HEADER = ["diff --git a/f.py b/f.py", "index 1111111..2222222 100644", "--- a/f.py", "+++ b/f.py"]


def load(name: str) -> Patch:
    return parse_patch((FIXTURES / f"{name}.patch").read_text(encoding="utf-8"))


def text(*lines: str) -> str:
    return "\n".join(lines) + "\n"


def headers(patch: Patch) -> list[tuple[int, int, int, int]]:
    return [
        (h.old_start, h.old_count, h.new_start, h.new_count) for f in patch.files for h in f.hunks
    ]


def test_single_hunk() -> None:
    patch = load("single_hunk")
    assert patch.warnings == ()
    [fp] = patch.files
    assert (fp.old_path, fp.new_path, fp.binary) == ("shed/calc.py", "shed/calc.py", False)
    [hunk] = fp.hunks
    assert (hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count) == (7, 9, 7, 10)
    assert hunk.lines[:5] == (" x7 = 7", " x8 = 8", " x9 = 9", "-x10 = 10", "+x10 = 100")
    assert hunk.lines[-1] == " x15 = 15"


def test_multi_hunk() -> None:
    patch = load("multi_hunk")
    assert headers(patch) == [(1, 6, 1, 6), (12, 7, 12, 6), (25, 6, 24, 8)]
    assert patch.files[0].hunks[1].lines[3] == "-x15 = 15"


def test_pure_add_and_delete() -> None:
    added = load("pure_add").files[0].hunks[0].lines
    deleted = load("pure_delete").files[0].hunks[0].lines
    assert [line[0] for line in added] == list("   ++   ")
    assert [line[0] for line in deleted] == list("   --   ")


def test_new_file() -> None:
    [fp] = load("new_file").files
    assert (fp.old_path, fp.new_path, fp.path) == (None, "shed/util.py", "shed/util.py")
    assert fp.hunks == (Hunk(0, 0, 1, 3, ("+def util():", "+    return 42", "+")),)


def test_deleted_file() -> None:
    [fp] = load("deleted_file").files
    assert (fp.old_path, fp.new_path, fp.path) == ("shed/old.py", None, "shed/old.py")
    assert headers(load("deleted_file")) == [(1, 5, 0, 0)]


def test_rename_with_edits() -> None:
    patch = load("rename_with_edits")
    [fp] = patch.files
    assert (fp.old_path, fp.new_path) == ("shed/calc.py", "shed/core.py")
    assert headers(patch) == [(7, 7, 7, 7)]
    assert patch.touched_paths() == ("shed/calc.py", "shed/core.py")


def test_pure_rename_has_no_hunks() -> None:
    assert load("pure_rename").files == (FilePatch("shed/old.py", "shed/legacy.py", ()),)


def test_no_newline_marker_dropped() -> None:
    nonl, tail = load("no_newline_eof").files
    assert nonl.hunks[0].lines == (" p = 1", "-q = 2", "+q = 2")
    assert tail.hunks[0].lines == (" a = 1", " b = 2", "-c = 3", "+c = 3", "+d = 4")


def test_binary_files_differ() -> None:
    patch = load("binary")
    calc, logo = patch.files
    assert headers(patch) == [(1, 4, 1, 4)]
    assert logo == FilePatch("shed/logo.png", "shed/logo.png", (), binary=True)
    assert patch.warnings == ("shed/logo.png: binary diff ignored",)


def test_git_binary_patch_skipped() -> None:
    patch = load("git_binary")
    blob, calc, logo = patch.files
    assert blob == FilePatch(None, "shed/blob.bin", (), binary=True)
    assert calc.hunks[0].lines[-2:] == ("-x30 = 30", "+x30 = 3000")
    assert logo.binary and logo.old_path == logo.new_path == "shed/logo.png"
    assert patch.warnings == (
        "shed/blob.bin: binary diff ignored",
        "shed/logo.png: binary diff ignored",
    )


def test_multiple_files() -> None:
    files = load("multi_file").files
    assert [(f.old_path, f.new_path, len(f.hunks)) for f in files] == [
        ("shed/calc.py", "shed/calc.py", 1),
        ("shed/old.py", "shed/old.py", 1),
        (None, "shed/py.typed", 0),  # empty new file
        ("shed/run.sh", "shed/run.sh", 0),  # mode change only
        (None, "shed/util.py", 1),
    ]


def test_format_patch_preamble_skipped() -> None:
    patch = load("format_patch")
    assert [f.path for f in patch.files] == ["shed/calc.py"]
    assert headers(patch) == [(2, 7, 2, 7)]
    assert patch.warnings == ()


def test_quoted_and_spaced_paths() -> None:
    cafe, spaced, renamed = load("quoted_paths").files
    assert cafe.old_path == cafe.new_path == "shed/café.py"
    assert headers(Patch((cafe,))) == [(1, 1, 1, 1)]  # counts default to 1
    assert spaced.old_path == spaced.new_path == "shed/foo bar.py"  # trailing tab stripped
    assert (renamed.old_path, renamed.new_path) == ("shed/run.sh", "shed/run\tme.sh")


def test_plain_unified_diff_with_timestamps() -> None:
    [fp] = load("plain_diff").files
    assert (fp.old_path, fp.new_path) == ("calc.py", "calc.py")
    assert fp.hunks[0].lines[3:5] == ("-x20 = 20", "+x20 = 200")


def test_plain_new_and_deleted_via_dev_null() -> None:
    patch = parse_patch(
        text("--- /dev/null", "+++ b/n.py", "@@ -0,0 +1 @@", "+n")
        + text("--- a/d.py", "+++ /dev/null", "@@ -1 +0,0 @@", "-d")
    )
    assert [(f.old_path, f.new_path) for f in patch.files] == [(None, "n.py"), ("d.py", None)]


def test_empty_input() -> None:
    assert parse_patch("") == Patch((), ())


def test_text_without_diff_warns() -> None:
    assert parse_patch("just a commit message\n").warnings == ("no file diffs found in patch",)


def test_body_lines_follow_header_counts() -> None:
    # A deleted "-- x" line reads as "--- x"; an empty line is context with its space stripped.
    patch = parse_patch(text(*HEADER, "@@ -1,3 +1,2 @@", "--- x", "", " y", "-- ", "2.53.0"))
    assert patch.files[0].hunks[0].lines == ("--- x", " ", " y")


def test_mid_hunk_no_newline_marker() -> None:
    body = ["@@ -1 +1,2 @@", "-a", "\\ No newline at end of file", "+a", "+b"]
    assert parse_patch(text(*HEADER, *body)).files[0].hunks[0].lines == ("-a", "+a", "+b")


def test_crlf_header_paths() -> None:
    patch = parse_patch("--- a/w.py\r\n+++ b/w.py\r\n@@ -1 +1 @@\r\n-a\r\n+b\r\n")
    assert patch.files[0].path == "w.py"
    assert patch.files[0].hunks[0].lines == ("-a\r", "+b\r")


def test_offset_header_warns() -> None:
    body = ["@@ -1 +1,2 @@", " a", "+b", "@@ -10 +10 @@", " c"]
    assert parse_patch(text(*HEADER, *body)).warnings == (
        "f.py: hunk at line 8 is offset from its header",
    )


@pytest.mark.parametrize(
    ("header", "path"),
    [
        ("diff --git i/f.py w/f.py", "f.py"),  # diff.mnemonicPrefix
        ("diff --git old/foo bar.py new/foo bar.py", "foo bar.py"),  # custom prefixes, spaces
        ("diff --git f.py f.py", "f.py"),  # --no-prefix, top-level file
        ('diff --git "i/caf\\303\\251.py" "w/caf\\303\\251.py"', "café.py"),
    ],
)
def test_git_header_strips_any_prefix(header: str, path: str) -> None:
    # Like git apply -p1: the first path component goes, whatever it is.
    patch = parse_patch(text(header, "old mode 100644", "new mode 100755"))
    assert patch.files == (FilePatch(path, path, ()),)


def test_side_lines_strip_any_prefix() -> None:
    lines = [
        "diff --git i/s/f.py w/s/f.py",
        "--- i/s/f.py",
        "+++ w/s/f.py",
        "@@ -1 +1 @@",
        "-a",
        "+b",
    ]
    assert parse_patch(text(*lines)).touched_paths() == ("s/f.py",)


@pytest.mark.parametrize(
    ("minus", "plus", "path"),
    [  # what `git apply --numstat` reports for each pair
        ("calc.py.orig", "calc.py", "calc.py"),
        ("calc.py", "calc.py.new", "calc.py"),
        ("a/x.py", "b/y.py", "y.py"),
        ("old/shed/calc.py", "new/shed/calc.py", "shed/calc.py"),
    ],
)
def test_plain_diff_never_renames(minus: str, plus: str, path: str) -> None:
    patch = parse_patch(
        text(f"--- {minus}\t2026-01-01", f"+++ {plus}\t2026-01-01", "@@ -1 +1 @@", "-a", "+b")
    )
    assert [(f.old_path, f.new_path) for f in patch.files] == [(path, path)]


def test_unknown_header_lines_skipped() -> None:
    patch = parse_patch(
        text(HEADER[0], "x-future-header: 1", *HEADER[1:], "@@ -1 +1 @@", "-a", "+b")
    )
    assert [(f.path, len(f.hunks)) for f in patch.files] == [("f.py", 1)]


def test_crlf_markers() -> None:
    binary = [
        "diff --git a/x.bin b/x.bin",
        "index 1..2 100644",
        "GIT binary patch",
        "literal 3",
        "K",
    ]
    patch = parse_patch("\r\n".join(binary) + "\r\n")
    assert patch.files == (FilePatch("x.bin", "x.bin", (), binary=True),)
    empty_context = "--- a/f\r\n+++ b/f\r\n@@ -1,2 +1,2 @@\r\n\r\n-a\r\n+b\r\n"
    assert parse_patch(empty_context).files[0].hunks[0].lines == (" ", "-a\r", "+b\r")


def test_copy_warns() -> None:
    patch = parse_patch(
        text("diff --git a/s.py b/t.py", "similarity index 100%", "copy from s.py", "copy to t.py")
    )
    assert patch.files == (FilePatch("s.py", "t.py", ()),)
    assert patch.warnings == ("t.py: copy of s.py treated as a rename",)


@pytest.mark.parametrize(
    ("body", "message"),
    [
        (["@@ -1,2 +1,x @@", " a"], "line 5: malformed hunk header"),
        (["@@ -1,3 +1,3 @@", " a", " b"], "line 7: patch ends inside a hunk"),
        (["@@ -1 +1 @@", "-a", "+b", "+c"], "line 8: hunk has more lines than its header says"),
        (["@@ -1 +1 @@", "-a", "+b", " c"], "line 8: hunk has more lines than its header says"),
        (["@@ -1 +1 @@", "-a", "-b", "+c"], "line 7: hunk has more lines than its header says"),
        (["@@ -1 +1 @@", "?a", "+b"], "line 6: unexpected line in hunk"),
        (["@@ -0,1 +1 @@", "-a", "+b"], "line 5: hunk range starts at line 0"),
        (
            ["@@ -5 +5 @@", " a", "@@ -3 +3 @@", " b"],
            "line 7: hunk overlaps or precedes the previous one",
        ),
    ],
)
def test_malformed_hunks(body: list[str], message: str) -> None:
    with pytest.raises(ValueError, match=f"^{message}$"):
        parse_patch(text(*HEADER, *body))


@pytest.mark.parametrize(
    ("lines", "message"),
    [
        (["diff --git a/f.py b/f.py", "--- a/f.py", "@@ -1 +1 @@"], "line 2: '---' line not"),
        (["diff --cc f.py"], "line 1: combined diffs are not supported"),
        (
            [
                "diff --git a/f.py b/f.py",
                "new file mode 100644",
                *HEADER[2:],
                "@@ -1 +1 @@",
                "-a",
                "+b",
            ],
            "line 1: new file has a hunk with old lines",
        ),
        (['diff --git "a/f.py b/f.py', 'rename from "x\\q"'], "line 2: bad quoted path"),
        (["--- /dev/null", "+++ /dev/null", "@@ -0,0 +0,0 @@"], "line 1: both sides are"),
        (["diff --git a/x b/y b/z", "old mode 100644"], "line 1: cannot determine file path"),
        (["diff --git a/f.py b/f.py", "index 1..2", "@@ -1 +1 @@"], "line 3: hunk without '---'"),
    ],
)
def test_malformed_files(lines: list[str], message: str) -> None:
    with pytest.raises(ValueError, match=f"^{message}"):
        parse_patch(text(*lines))


def test_patch_series_rejected() -> None:
    once = text(*HEADER, "@@ -1 +1 @@", "-a", "+b")
    with pytest.raises(ValueError, match=r"^line 8: f\.py is patched twice"):
        parse_patch(once + once)
