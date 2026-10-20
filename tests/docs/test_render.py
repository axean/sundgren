"""The report graphic (docs/_ext/sundgren_render.py) shows only numbers from the report."""

from __future__ import annotations

import importlib
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from typing import Any

import pytest

import docs_examples

sys.path.insert(0, str(docs_examples.ROOT / "docs" / "_ext"))
sr = importlib.import_module("sundgren_render")

DIRS = [p for p in docs_examples.example_dirs() if (p / "report.json").is_file()]


@dataclass
class El:
    tag: str
    attrs: dict[str, str]
    parent: El | None
    text: str = ""
    children: list[El] = field(default_factory=list["El"])

    @property
    def classes(self) -> set[str]:
        return set(self.attrs.get("class", "").split())

    def find(self, cls: str) -> list[El]:
        """Descendants with class `cls`, in document order."""
        out: list[El] = []
        for child in self.children:
            if cls in child.classes:
                out.append(child)
            out += child.find(cls)
        return out

    def one(self, cls: str) -> El:
        [el] = self.find(cls)
        return el


class _Tree(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.root = El("root", {}, None)
        self.open = self.root

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        el = El(tag, {k: v or "" for k, v in attrs}, self.open)
        self.open.children.append(el)
        self.open = el

    def handle_endtag(self, tag: str) -> None:
        assert self.open.tag == tag, f"</{tag}> closes <{self.open.tag}>"
        assert self.open.parent is not None
        self.open = self.open.parent

    def handle_data(self, data: str) -> None:
        el: El | None = self.open
        while el is not None:
            el.text += data
            el = el.parent


def parse(html: str) -> El:
    tree = _Tree()
    tree.feed(html)
    tree.close()
    assert tree.open is tree.root, "unclosed element"
    return tree.root


def load(path: Path) -> tuple[dict[str, Any], dict[str, str], str]:
    example = docs_examples.load(path)
    report = json.loads((path / "report.json").read_text())
    sources = {p: (path / "project" / p).read_text() for p in sr.changed_paths(report)}
    return report, sources, (path / example.patch).read_text()


def patched_tree(path: Path, tmp_path: Path) -> Path:
    """project/ with the patch applied by git, independently of the renderer's own alignment."""
    tree = tmp_path / "patched"
    shutil.copytree(path / "project", tree)
    patch = path / docs_examples.load(path).patch
    env = {
        **{k: v for k, v in os.environ.items() if not k.startswith("GIT_")},
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_CONFIG_NOSYSTEM": "1",
    }
    subprocess.run(["git", "apply", str(patch)], cwd=tree, env=env, check=True)
    return tree


# ---- the recorded examples ----


@pytest.mark.parametrize("path", DIRS, ids=[p.name for p in DIRS])
def test_example_graphic_matches_report(path: Path, tmp_path: Path) -> None:
    report, sources, patch_text = load(path)
    root = parse(sr.render(report, sources, patch_text))
    patched = patched_tree(path, tmp_path)
    shown = [f for f in report["files"] if sr.is_changed(f)]
    figures = root.find("sg-file")
    assert [fig.attrs["data-path"] for fig in figures] == [f["path"] for f in shown]
    assert shown, "the example shows no file"
    for f, fig in zip(shown, figures, strict=True):
        check_file(f, fig, patched, path / "project")
    check_summary(report, root)
    assert "<script" not in sr.render(report, sources, patch_text)


def check_file(f: dict[str, Any], fig: El, patched: Path, base: Path) -> None:
    rows = [r for r in fig.find("sg-row") if "sg-fnrow" not in r.classes]
    new_text = (patched / f["path"]).read_text().split("\n")
    base_text: list[str] = (base / f["base_path"]).read_text().split("\n") if f["base_path"] else []
    # Every patched line once, in order, with its text; deleted lines show base text.
    news = [int(r.attrs["data-line"]) for r in rows if "data-line" in r.attrs]
    assert news == list(range(1, len(news) + 1))
    assert len(news) == len(new_text) - (new_text[-1] == "")
    for r in rows:
        src = r.one("sg-src").text
        if "data-line" in r.attrs:
            assert src == new_text[int(r.attrs["data-line"]) - 1]
        else:
            assert src == base_text[int(r.attrs["data-base-line"]) - 1]

    by_key = {
        (
            r.attrs.get("data-line", ""),
            "" if "data-line" in r.attrs else r.attrs["data-base-line"],
        ): r
        for r in rows
    }
    entries = [*f["lines"], *({**e, "category": "flaky"} for e in f["flaky"])]
    with_entry: set[tuple[str, str]] = set()
    for e in entries:
        k = _key(e)
        row = by_key[k]
        with_entry.add(k)
        assert row.attrs["data-cat"] == e["category"]
        assert row.attrs["data-hits"] == ",".join(sr.fmt_count(h) for h in e["hits"])
        assert row.one("sg-hits").text == sr.fmt_pair(e["hits"])
        if e["line"] is not None and e["base_line"] is not None:
            assert row.attrs["data-base-line"] == str(e["base_line"])
    for k, row in by_key.items():
        if k not in with_entry:
            assert "data-cat" not in row.attrs and row.one("sg-hits").text == ""

    for k, row in by_key.items():
        expected = [b for b in f["branches"] if _key(b) == k]
        marks = row.find("sg-br")
        assert [m.attrs["class"] for m in marks] == [
            f"sg-br sg-b-{b['category']}" for b in expected
        ]
        assert [m.attrs["data-taken"] for m in marks] == [
            ",".join(sr.fmt_count(t) for t in b["taken"]) for b in expected
        ]
    if not f["branches"]:
        assert "no branch data" in fig.text

    badges = fig.find("sg-fn")
    assert [b.attrs["data-fn"] for b in badges] == [
        fn["name"] for fn in sorted(f["functions"], key=lambda fn: (fn["start"], -fn["end"]))
    ]
    for fn in f["functions"]:
        [badge] = [b for b in badges if b.attrs["data-fn"] == fn["name"]]
        assert badge.attrs["data-status"] == fn["status"]
        assert sr.FUNCTION_LABELS[fn["status"]] in badge.text
        assert f"lines {fn['start']}–{fn['end']}" in badge.text
        n = fn["lines"]
        if n["lost"]:
            assert f"lost {n['lost']} of {n['executable']}" in badge.text
        if n["gained"]:
            assert f"gained {n['gained']} of {n['executable']}" in badge.text
        if not n["lost"] and not n["gained"]:
            assert f"· {n['executable']} body line" in badge.text
    bars = fig.find("sg-fnbar")
    assert len(bars) == len(f["functions"])


def _key(e: dict[str, Any]) -> tuple[str, str]:
    """(line, "") for a patched-side entry, ("", base_line) for a deleted one."""
    return (str(e["line"]), "") if e["line"] is not None else ("", str(e["base_line"]))


def check_summary(report: dict[str, Any], root: El) -> None:
    summary, stats = report["summary"], report["tree"]["stats"]
    chips = {c.attrs["data-summary"]: int(c.attrs["data-n"]) for c in root.find("sg-chip")}
    assert chips == {
        f"{section}.{k}": n
        for section in ("lines", "branches", "functions")
        for k, n in summary[section].items()
        if n
    }
    for chip in root.find("sg-chip"):
        assert chip.text.startswith(f"{chip.attrs['data-n']} ")

    rows = {r.attrs["data-side"]: r for r in root.find("sg-meter-row")}
    for side in ("base", "patched"):
        pct = summary["coverage"][f"{side}_pct"]
        assert rows[side].attrs.get("data-pct") == (None if pct is None else str(pct))
        assert rows[side].one("sg-meter-pct").text == sr.fmt_pct(pct)
        hit, total = stats[f"hit_{side}"], stats[f"executable_{side}"]
        assert rows[side].one("sg-meter-lines").text == f"{hit}/{total} lines"

    tests = root.one("sg-tests")
    changed = [t for t in report["tests"] if t["base"] != t["patched"]]
    shown = tests.find("sg-test")
    assert [t.attrs["data-test"] for t in shown] == [t["id"] for t in changed]
    for t, el in zip(changed, shown, strict=True):
        assert [o.text for o in el.find("sg-outcome")] == [t["base"], t["patched"]]
    if not report["tests"]:
        assert "No test outcomes in this report." in tests.text


@pytest.mark.parametrize("path", DIRS, ids=[p.name for p in DIRS])
def test_example_graphic_agrees_with_text_report(path: Path) -> None:
    """The coverage line of report.txt prints the same percentages the meter shows."""
    report, sources, patch_text = load(path)
    root = parse(sr.render(report, sources, patch_text))
    pct = [el.text for el in root.find("sg-meter-pct")]
    assert f"coverage   {pct[0]} -> {pct[1]}" in (path / "report.txt").read_text()


# ---- align ----


def _patch(body: str) -> Any:
    [fp] = sr.parse_patch(f"--- a/f\n+++ b/f\n{body}")
    return fp


def test_align_context_add_delete() -> None:
    base = "a\nb\nc\nd\n"
    rows = sr.align(base, _patch("@@ -2,2 +2,2 @@\n-b\n+B\n c\n"))
    assert [(r.base, r.new, r.text, r.mark) for r in rows] == [
        (1, 1, "a", " "),
        (2, None, "b", "-"),
        (None, 2, "B", "+"),
        (3, 3, "c", " "),
        (4, 4, "d", " "),
    ]


def test_align_insert_at_top_and_new_file() -> None:
    rows = sr.align("a\n", _patch("@@ -0,0 +1 @@\n+z\n"))
    assert [(r.base, r.new, r.mark) for r in rows] == [(None, 1, "+"), (1, 2, " ")]
    rows = sr.align("", _patch("@@ -0,0 +1,2 @@\n+x\n+y\n"))
    assert [(r.base, r.new, r.text) for r in rows] == [(None, 1, "x"), (None, 2, "y")]


def test_align_without_patch_and_crlf() -> None:
    rows = sr.align("a\r\nb\r\n", None)
    assert [(r.base, r.new, r.text) for r in rows] == [(1, 1, "a\r"), (2, 2, "b\r")]


@pytest.mark.parametrize(
    "body", ["@@ -1 +1 @@\n-x\n+y\n", "@@ -5 +5 @@\n-a\n+b\n", "@@ -1,2 +1,2 @@\n a\n-zz\n+b\n"]
)
def test_align_rejects_patch_that_does_not_match(body: str) -> None:
    with pytest.raises(ValueError):
        sr.align("a\nb\n", _patch(body))


# ---- synthetic reports ----


def _report(
    files: list[dict[str, Any]], tests: list[dict[str, str]] | None = None
) -> dict[str, Any]:
    zero = dict.fromkeys(
        ("lost", "gained", "new_hit", "new_miss", "deleted_hit", "deleted_miss"), 0
    )
    return {
        "files": files,
        "tests": tests or [],
        "summary": {
            "lines": {
                "lost": 0,
                "gained": 2,
                "new_hit": 0,
                "new_miss": 0,
                "deleted_hit": 1,
                "deleted_miss": 0,
                "flaky": 1,
                "unmeasured": 0,
            },
            "branches": {"lost": 0, "gained": 0, "new": 0, "deleted": 1},
            "functions": {
                "went_dark": 0,
                "lit_up": 1,
                "partial_loss": 0,
                "partial_gain": 0,
                "mixed": 0,
                "added": 0,
                "removed": 0,
            },
            "coverage": {"base_pct": 50.0, "patched_pct": None},
        },
        "tree": {
            "stats": {
                **zero,
                "executable_base": 4,
                "hit_base": 2,
                "executable_patched": 0,
                "hit_patched": 0,
            }
        },
    }


def _file(**over: Any) -> dict[str, Any]:
    return {
        "path": "m.py",
        "base_path": "m.py",
        "lines": [],
        "branches": [],
        "functions": [],
        "flaky": [],
        **over,
    }


BASE = "def f():\n    x = 1\n    if x:\n        return <x>\n"
PATCH = "--- a/m.py\n+++ b/m.py\n@@ -2,2 +2,2 @@\n-    x = 1\n+    x = 2\n     if x:\n"


def test_synthetic_deleted_lines_branches_nested_functions_and_flaky() -> None:
    f = _file(
        lines=[
            {"line": 1, "base_line": 1, "category": "kept_hit", "hits": [1, 1]},
            {"line": None, "base_line": 2, "category": "deleted_hit", "hits": [1, None]},
            {"line": 2, "base_line": None, "category": "new_hit", "hits": [None, 3]},
            {"line": 4, "base_line": 4, "category": "gained", "hits": [0, 2]},
        ],
        flaky=[{"line": 3, "base_line": 3, "hits": [1, 0], "sides": ["patched"]}],
        branches=[
            {
                "line": None,
                "base_line": 2,
                "block": "0",
                "branch": "b0",
                "category": "branch_deleted",
                "taken": [1, None],
            }
        ],
        functions=[
            {
                "name": "f",
                "start": 1,
                "end": 4,
                "status": "lit_up",
                "lines": {"lost": 0, "gained": 1, "executable": 3},
            },
            {
                "name": "f.<locals>.g",
                "start": 3,
                "end": 4,
                "status": "unchanged",
                "lines": {"lost": 0, "gained": 0, "executable": 1},
            },
        ],
    )
    report = _report([f], tests=[{"id": "t", "base": "passed", "patched": "passed"}])
    root = parse(sr.render(report, {"m.py": BASE}, PATCH))
    rows = [r for r in root.find("sg-row") if "sg-fnrow" not in r.classes]
    assert [
        (r.attrs.get("data-line"), r.attrs.get("data-base-line"), r.attrs.get("data-cat"))
        for r in rows
    ] == [
        ("1", "1", "kept_hit"),
        (None, "2", "deleted_hit"),
        ("2", None, "new_hit"),
        ("3", "3", "flaky"),
        ("4", "4", "gained"),
    ]
    assert rows[1].one("sg-hits").text == "1 → –"
    assert rows[1].one("sg-br").attrs["data-taken"] == "1,–"
    assert rows[4].one("sg-src").text == "        return <x>"  # escaped and parsed back
    bars = root.find("sg-fnbar")
    assert [b.attrs["style"] for b in bars] == [
        "--sg-i:0;--sg-n:7;--sg-lane:0",  # header, 5 source rows and g's header
        "--sg-i:4;--sg-n:3;--sg-lane:1",
    ]
    assert "No test changed outcome (1 unchanged)." in root.one("sg-tests").text
    assert root.find("sg-meter-pct")[1].text == "n/a"
    legend = root.one("sg-legend").text
    for word in (
        "deleted hit",
        "new hit",
        "flaky",
        "gained",
        "branch deleted",
        "function lit up",
        "line added by the patch",
        "line deleted by the patch",
    ):
        assert word in legend


def test_unchanged_files_are_not_shown() -> None:
    kept = _file(lines=[{"line": 1, "base_line": 1, "category": "kept_hit", "hits": [1, 1]}])
    html = sr.render(_report([kept]), {}, "")
    assert "sg-file" not in html and "No file changed coverage." in html


@pytest.mark.parametrize(
    "over",
    [
        {"lines": [{"line": 9, "base_line": 9, "category": "lost", "hits": [1, 0]}]},
        {
            "functions": [
                {
                    "name": "f",
                    "start": 1,
                    "end": 9,
                    "status": "went_dark",
                    "lines": {"lost": 1, "gained": 0, "executable": 1},
                }
            ]
        },
    ],
)
def test_entries_outside_the_source_are_errors(over: dict[str, Any]) -> None:
    with pytest.raises(ValueError, match="not in the source"):
        sr.render(_report([_file(**over)]), {"m.py": BASE}, "")
