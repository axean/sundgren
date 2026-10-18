"""Function spans: Symbolizer protocol, registry by extension, lcov FN fallback."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Protocol

from sundgren.core.model import FileCoverage, LineNo


@dataclass(frozen=True, slots=True)
class Span:
    name: str  # qualified, e.g. "Class.method"
    start: LineNo
    end: LineNo
    body_start: LineNo | None = None  # first body line; lines before it (signature) don't count

    @property
    def body_lines(self) -> range:
        return range(self.body_start if self.body_start is not None else self.start, self.end + 1)


class Symbolizer(Protocol):
    def functions(self, source: str) -> list[Span]: ...


_REGISTRY: dict[str, Symbolizer] = {}


def register_symbolizer(extensions: tuple[str, ...], symbolizer: Symbolizer) -> None:
    for ext in extensions:
        _REGISTRY[ext] = symbolizer


def get_symbolizer(path: str) -> Symbolizer | None:
    return _REGISTRY.get(PurePosixPath(path).suffix)


class LcovFnSymbolizer:
    """Spans from lcov FN records. Missing end lines run to the next function or last line.

    The FN line is the signature, which tools report hit on import or entry, so the body starts
    one line later. A one-line function has no separate body.
    """

    def spans(self, fc: FileCoverage) -> list[Span]:
        funcs = sorted(fc.functions, key=lambda f: (f.start, f.name))
        last_line = max(fc.lines, default=0)
        spans = []
        for i, f in enumerate(funcs):
            end = f.end
            if end is None:
                later = [g.start for g in funcs[i + 1 :] if g.start > f.start]
                end = later[0] - 1 if later else max(last_line, f.start)
            end = max(end, f.start)
            spans.append(Span(f.name, f.start, end, f.start + 1 if end > f.start else None))
        return spans


def resolve_spans(path: str, root: Path | None, fc: FileCoverage | None) -> list[Span] | None:
    """Spans for one side of a file: registered symbolizer, then lcov FN, else None."""
    symbolizer = get_symbolizer(path)
    if symbolizer is not None and root is not None:
        source_path = root / path
        try:
            source = source_path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            source = None
        if source is not None:
            try:
                return sorted(symbolizer.functions(source), key=_span_key)
            except SyntaxError:
                pass
    if fc is not None and fc.functions:
        return sorted(LcovFnSymbolizer().spans(fc), key=_span_key)
    return None


def _span_key(span: Span) -> tuple[int, int, str]:
    return (span.start, span.end, span.name)


def spans_by_name(spans: Sequence[Span]) -> dict[str, Span]:
    """First span per qualified name (duplicates, e.g. redefinitions, keep the earliest)."""
    out: dict[str, Span] = {}
    for s in spans:
        out.setdefault(s.name, s)
    return out
