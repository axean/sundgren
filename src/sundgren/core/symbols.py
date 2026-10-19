"""Function spans: Symbolizer protocol, registry by extension, lcov FN fallback.

`Span`, `LcovFnSymbolizer` and `resolve_spans` are implemented in Rust (`sundgren._native`).
The registry stays here; `resolve_spans` reads it through `get_symbolizer` at call time.
"""

from __future__ import annotations

from pathlib import PurePosixPath
from typing import Protocol

from sundgren._native import core_symbols as _impl

Span = _impl.Span
LcovFnSymbolizer = _impl.LcovFnSymbolizer
resolve_spans = _impl.resolve_spans
spans_by_name = _impl.spans_by_name


class Symbolizer(Protocol):
    def functions(self, source: str) -> list[Span]: ...


_REGISTRY: dict[str, Symbolizer] = {}


def register_symbolizer(extensions: tuple[str, ...], symbolizer: Symbolizer) -> None:
    for ext in extensions:
        _REGISTRY[ext] = symbolizer


def get_symbolizer(path: str) -> Symbolizer | None:
    return _REGISTRY.get(PurePosixPath(path).suffix)


__all__ = [
    "LcovFnSymbolizer",
    "Span",
    "Symbolizer",
    "get_symbolizer",
    "register_symbolizer",
    "resolve_spans",
    "spans_by_name",
]
