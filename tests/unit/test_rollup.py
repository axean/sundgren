"""core.rollup: function statuses and the directory tree."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import asdict
from pathlib import Path

import pytest

from sundgren.core import symbols
from sundgren.core.diff import diff
from sundgren.core.linemap import LineMap, identity
from sundgren.core.model import FileCoverage, FunctionRecord, Snapshot
from sundgren.core.rollup import Stats, TreeNode, build_tree, function_deltas
from sundgren.core.symbols import Span

ID = identity("m.py")
F = Span("f", 1, 4, body_start=2)  # line 1 is the signature
G = Span("g", 6, 7, body_start=7)


def status(
    base: dict[int, int],
    patched: dict[int, int],
    *,
    lost: Iterable[int] = (),
    gained: Iterable[int] = (),
) -> tuple[str, int, int, int]:
    [fd] = function_deltas(
        [F], [F], ID, base, patched, lost=frozenset(lost), gained=frozenset(gained)
    )
    return fd.status, fd.lost, fd.gained, fd.executable


def test_went_dark_ignores_the_signature_line() -> None:
    base = {1: 1, 2: 1, 3: 1, 4: 1}
    patched = {1: 1, 2: 0, 3: 0, 4: 0}  # def line still runs at import
    assert status(base, patched, lost={2, 3, 4}) == ("went_dark", 3, 0, 3)


def test_lit_up() -> None:
    assert status({1: 1, 2: 0, 3: 0}, {1: 1, 2: 1, 3: 1}, gained={2, 3}) == ("lit_up", 0, 2, 2)


def test_partial_loss() -> None:
    assert status({2: 1, 3: 1}, {2: 1, 3: 0}, lost={3}) == ("partial_loss", 1, 0, 2)


def test_partial_gain() -> None:
    assert status({2: 1, 3: 0}, {2: 1, 3: 1}, gained={3}) == ("partial_gain", 0, 1, 2)


def test_mixed() -> None:
    assert status({2: 1, 3: 0}, {2: 0, 3: 1}, lost={2}, gained={3}) == ("mixed", 1, 1, 2)


def test_unchanged() -> None:
    assert status({2: 1, 3: 0}, {2: 1, 3: 0}) == ("unchanged", 0, 0, 2)


def test_added_and_removed() -> None:
    added = function_deltas([], [G], ID, {}, {6: 1, 7: 0})
    assert [(f.name, f.status, f.start, f.end, f.executable) for f in added] == [
        ("g", "added", 6, 7, 1)
    ]
    removed = function_deltas([G], [], ID, {6: 1, 7: 1}, {})
    assert [(f.name, f.status, f.start, f.end, f.executable) for f in removed] == [
        ("g", "removed", 6, 7, 1)
    ]


def test_no_spans_on_either_side() -> None:
    assert function_deltas(None, None, ID, {1: 1}, {1: 0}) == ()


def test_flaky_lines_do_not_decide_went_dark() -> None:
    [fd] = function_deltas(
        [F], [F], ID, {2: 1, 3: 1}, {2: 0, 3: 1}, lost=frozenset({2}), flaky_patched=frozenset({3})
    )
    assert fd.status == "went_dark"


def test_renamed_function_matches_by_overlap() -> None:
    # def line rewritten (old -> new name), body unchanged.
    lm = LineMap("m.py", "m.py", {}, frozenset({1}), frozenset({1}))
    [fd] = function_deltas(
        [Span("old", 1, 3, 2)],
        [Span("new", 1, 3, 2)],
        lm,
        {1: 1, 2: 1, 3: 1},
        {1: 1, 2: 0, 3: 0},
        lost=frozenset({2, 3}),
    )
    assert (fd.name, fd.status) == ("new", "went_dark")


def test_unknown_side_borrows_spans_through_the_linemap() -> None:
    lm = LineMap("m.py", "m.py", {}, frozenset(), frozenset({1, 2}))
    [fd] = function_deltas([F], None, lm, {2: 1, 3: 1}, {4: 0, 5: 0}, lost=frozenset({4, 5}))
    assert (fd.name, fd.status, fd.start, fd.end) == ("f", "went_dark", 3, 6)


def test_results_sorted_by_position() -> None:
    out = function_deltas([G, F], [G, F], ID, {}, {})
    assert [f.name for f in out] == ["f", "g"]


# --- through diff(): symbolizer with a real root, and lcov FN ---------------

SOURCE = "def f(a):\n    if a:\n        return 1\n    return 2\n"


class FakeSymbolizer:
    def __init__(self) -> None:
        self.seen: list[str] = []

    def functions(self, source: str) -> list[Span]:
        self.seen.append(source)
        return [Span("f", 1, 4, body_start=2)]


def _fc(lines: dict[int, int], functions: tuple[FunctionRecord, ...] = ()) -> FileCoverage:
    return FileCoverage("pkg/m.py", lines, {}, functions, False)


def _snap(fc: FileCoverage) -> Snapshot:
    return Snapshot({fc.path: fc}, ())


def test_registered_symbolizer_reads_source_from_root(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake = FakeSymbolizer()
    monkeypatch.setitem(symbols._REGISTRY, ".py", fake)
    for side in ("base", "patched"):
        (tmp_path / side / "pkg").mkdir(parents=True)
        (tmp_path / side / "pkg" / "m.py").write_text(SOURCE)
    d = diff(
        [_snap(_fc({1: 1, 2: 1, 3: 1, 4: 0}))],
        [_snap(_fc({1: 1, 2: 0, 3: 0, 4: 0}))],
        {},
        base_root=tmp_path / "base",
        patched_root=tmp_path / "patched",
    )
    assert fake.seen == [SOURCE, SOURCE]
    [fd] = d.files[0].functions
    assert (fd.name, fd.status, fd.lost, fd.executable) == ("f", "went_dark", 2, 3)


def test_lcov_fn_records_without_root(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(symbols._REGISTRY, ".py", FakeSymbolizer())  # unused: no root
    fns = (FunctionRecord("f", 1, 4, 1), FunctionRecord("g", 6, None, 1))
    d = diff(
        [_snap(_fc({1: 1, 2: 1, 3: 1, 4: 1, 6: 1, 7: 1}, fns))],
        [_snap(_fc({1: 1, 2: 1, 3: 0, 4: 1, 6: 1, 7: 0}, fns))],
        {},
    )
    got = [(f.name, f.status, f.start, f.end, f.lost) for f in d.files[0].functions]
    # FN line is the signature: g's body is line 7 only, so g went dark.
    assert got == [("f", "partial_loss", 1, 4, 1), ("g", "went_dark", 6, 7, 1)]


# --- tree --------------------------------------------------------------------


def test_stats_add() -> None:
    a = Stats(lost=1, executable_base=3, hit_patched=2)
    b = Stats(lost=2, gained=1, executable_base=1)
    assert a + b == Stats(lost=3, gained=1, executable_base=4, hit_patched=2)


def test_tree_sums_and_sorts() -> None:
    tree = build_tree(
        [
            ("pkg/sub/b.py", Stats(lost=1, executable_base=2)),
            ("pkg/a.py", Stats(gained=2, executable_patched=5)),
            ("top.py", Stats(new_miss=1)),
            ("pkg/sub/a.py", Stats(lost=4)),
        ]
    )
    assert (tree.name, tree.path, tree.kind) == ("", "", "dir")
    assert tree.stats == Stats(
        lost=5, gained=2, new_miss=1, executable_base=2, executable_patched=5
    )
    [pkg, top] = tree.children
    assert (pkg.path, pkg.kind, top.path, top.kind) == ("pkg", "dir", "top.py", "file")
    assert [c.name for c in pkg.children] == ["a.py", "sub"]
    sub = pkg.children[1]
    assert sub.path == "pkg/sub" and sub.stats.lost == 5
    assert [(c.name, c.path) for c in sub.children] == [
        ("a.py", "pkg/sub/a.py"),
        ("b.py", "pkg/sub/b.py"),
    ]
    assert all(c.children == () for c in sub.children)


def test_tree_stats_keys() -> None:
    assert list(asdict(Stats())) == [
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
    ]


def test_empty_tree() -> None:
    assert build_tree([]) == TreeNode("", "", "dir", Stats(), ())
