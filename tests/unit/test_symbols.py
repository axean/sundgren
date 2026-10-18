from __future__ import annotations

from pathlib import Path

import pytest

from sundgren.core import symbols
from sundgren.core.model import FileCoverage, FunctionRecord
from sundgren.core.symbols import (
    LcovFnSymbolizer,
    Span,
    get_symbolizer,
    register_symbolizer,
    resolve_spans,
    spans_by_name,
)


class FixedSymbolizer:
    def __init__(self, spans: list[Span]) -> None:
        self.spans = spans
        self.sources: list[str] = []

    def functions(self, source: str) -> list[Span]:
        self.sources.append(source)
        return list(self.spans)


class BrokenSymbolizer:
    def functions(self, source: str) -> list[Span]:
        raise SyntaxError("nope")


def fc(
    functions: tuple[FunctionRecord, ...] = (), lines: dict[int, int] | None = None
) -> FileCoverage:
    return FileCoverage(
        path="pkg/mod.fake",
        lines=lines if lines is not None else {},
        branches={},
        functions=functions,
        external=False,
    )


def fn(name: str, start: int, end: int | None = None, hits: int | None = 1) -> FunctionRecord:
    return FunctionRecord(name, start, end, hits)


LCOV_FC = fc((fn("b", 10, 12), fn("a", 1)), {1: 1, 2: 1, 10: 0, 11: 0, 12: 0})
LCOV_SPANS = [Span("a", 1, 9, 2), Span("b", 10, 12, 11)]


# Span


def test_body_lines_with_body_start() -> None:
    assert Span("f", 3, 7, 5).body_lines == range(5, 8)


def test_body_lines_without_body_start() -> None:
    assert Span("f", 3, 7).body_lines == range(3, 8)
    assert list(Span("f", 4, 4).body_lines) == [4]


# registry


def test_register_and_get_by_extension(monkeypatch: pytest.MonkeyPatch) -> None:
    placeholder = FixedSymbolizer([])
    for ext in (".fake", ".alt"):
        monkeypatch.setitem(symbols._REGISTRY, ext, placeholder)  # undone after the test
    sym = FixedSymbolizer([])
    register_symbolizer((".fake", ".alt"), sym)
    assert get_symbolizer("pkg/mod.fake") is sym
    assert get_symbolizer("deep/dir/x.alt") is sym


def test_get_symbolizer_unknown_extension() -> None:
    assert get_symbolizer("pkg/mod.nosuchext") is None
    assert get_symbolizer("Makefile") is None


def test_get_symbolizer_uses_last_suffix_only(monkeypatch: pytest.MonkeyPatch) -> None:
    sym = FixedSymbolizer([])
    monkeypatch.setitem(symbols._REGISTRY, ".fake", sym)
    assert get_symbolizer("a.b/c.tar.fake") is sym
    assert get_symbolizer("a.fake/c") is None
    assert get_symbolizer("pkg/mod.FAKE") is None  # case-sensitive


# LcovFnSymbolizer


def test_lcov_spans_explicit_end() -> None:
    got = LcovFnSymbolizer().spans(fc((fn("f", 2, 5), fn("g", 7, 9)), {2: 1, 9: 1}))
    assert got == [Span("f", 2, 5, 3), Span("g", 7, 9, 8)]


def test_lcov_missing_end_runs_to_next_start() -> None:
    got = LcovFnSymbolizer().spans(fc((fn("g", 8), fn("f", 3)), {3: 1, 4: 1, 8: 1, 9: 1}))
    assert got == [Span("f", 3, 7, 4), Span("g", 8, 9, 9)]


def test_lcov_last_function_ends_at_last_executable_line() -> None:
    got = LcovFnSymbolizer().spans(fc((fn("f", 3),), {1: 1, 3: 1, 4: 0, 12: 0}))
    assert got == [Span("f", 3, 12, 4)]


def test_lcov_last_function_without_lines_is_one_line() -> None:
    assert LcovFnSymbolizer().spans(fc((fn("f", 3),), {})) == [Span("f", 3, 3)]
    assert LcovFnSymbolizer().spans(fc((fn("f", 30),), {1: 1})) == [Span("f", 30, 30)]


def test_lcov_same_start_skips_to_next_distinct_start() -> None:
    # e.g. an outer and a closure reported on the same line
    got = LcovFnSymbolizer().spans(fc((fn("a", 2), fn("b", 2), fn("c", 6)), {2: 1, 7: 1}))
    assert got == [Span("a", 2, 5, 3), Span("b", 2, 5, 3), Span("c", 6, 7, 7)]


def test_lcov_body_skips_signature_line() -> None:
    # tarpaulin keeps `fn divide` hit when no test calls it; only the body may go dark
    (span,) = LcovFnSymbolizer().spans(fc((fn("divide", 1),), {1: 1, 2: 0, 3: 0, 5: 0}))
    assert list(span.body_lines) == [2, 3, 4, 5]


def test_lcov_end_before_start_is_clamped() -> None:
    assert LcovFnSymbolizer().spans(fc((fn("f", 5, 3),), {5: 1})) == [Span("f", 5, 5)]


def test_lcov_mixed_explicit_and_missing_ends() -> None:
    got = LcovFnSymbolizer().spans(fc((fn("f", 1, 4), fn("g", 6), fn("h", 10)), {11: 1}))
    assert got == [Span("f", 1, 4, 2), Span("g", 6, 9, 7), Span("h", 10, 11, 11)]


# resolve_spans


@pytest.fixture
def tree(tmp_path: Path) -> Path:
    (tmp_path / "pkg").mkdir()
    (tmp_path / "pkg" / "mod.fake").write_text("source text\n", encoding="utf-8")
    return tmp_path


def test_resolve_prefers_registered_symbolizer(monkeypatch: pytest.MonkeyPatch, tree: Path) -> None:
    sym = FixedSymbolizer([Span("z", 5, 6), Span("y", 1, 9), Span("x", 1, 3)])
    monkeypatch.setitem(symbols._REGISTRY, ".fake", sym)
    got = resolve_spans("pkg/mod.fake", tree, LCOV_FC)
    assert got == [Span("x", 1, 3), Span("y", 1, 9), Span("z", 5, 6)]  # sorted (start, end, name)
    assert sym.sources == ["source text\n"]


def test_resolve_symbolizer_empty_result_is_kept(
    monkeypatch: pytest.MonkeyPatch, tree: Path
) -> None:
    monkeypatch.setitem(symbols._REGISTRY, ".fake", FixedSymbolizer([]))
    assert resolve_spans("pkg/mod.fake", tree, LCOV_FC) == []


def test_resolve_without_root_uses_lcov(monkeypatch: pytest.MonkeyPatch) -> None:
    sym = FixedSymbolizer([Span("x", 1, 3)])
    monkeypatch.setitem(symbols._REGISTRY, ".fake", sym)
    assert resolve_spans("pkg/mod.fake", None, LCOV_FC) == LCOV_SPANS
    assert sym.sources == []


def test_resolve_missing_file_uses_lcov(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setitem(symbols._REGISTRY, ".fake", FixedSymbolizer([Span("x", 1, 3)]))
    assert resolve_spans("pkg/mod.fake", tmp_path, LCOV_FC) == LCOV_SPANS


def test_resolve_undecodable_file_uses_lcov(monkeypatch: pytest.MonkeyPatch, tree: Path) -> None:
    (tree / "pkg" / "mod.fake").write_bytes(b"\xff\xfe\x00bad")
    monkeypatch.setitem(symbols._REGISTRY, ".fake", FixedSymbolizer([Span("x", 1, 3)]))
    assert resolve_spans("pkg/mod.fake", tree, LCOV_FC) == LCOV_SPANS


def test_resolve_syntax_error_uses_lcov(monkeypatch: pytest.MonkeyPatch, tree: Path) -> None:
    monkeypatch.setitem(symbols._REGISTRY, ".fake", BrokenSymbolizer())
    assert resolve_spans("pkg/mod.fake", tree, LCOV_FC) == LCOV_SPANS


def test_resolve_syntax_error_without_lcov_functions_is_none(
    monkeypatch: pytest.MonkeyPatch, tree: Path
) -> None:
    monkeypatch.setitem(symbols._REGISTRY, ".fake", BrokenSymbolizer())
    assert resolve_spans("pkg/mod.fake", tree, fc()) is None
    assert resolve_spans("pkg/mod.fake", tree, None) is None


def test_resolve_unregistered_extension_uses_lcov(tree: Path) -> None:
    assert resolve_spans("pkg/mod.nosuchext", tree, LCOV_FC) == LCOV_SPANS


def test_resolve_nothing_available_is_none(tree: Path) -> None:
    assert resolve_spans("pkg/mod.nosuchext", tree, None) is None
    assert resolve_spans("pkg/mod.nosuchext", None, fc()) is None


# spans_by_name


def test_spans_by_name_keeps_first() -> None:
    first, second = Span("C.x", 2, 4), Span("C.x", 6, 8)
    assert spans_by_name([first, Span("f", 1, 1), second]) == {"C.x": first, "f": Span("f", 1, 1)}
