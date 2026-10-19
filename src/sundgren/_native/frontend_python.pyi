"""Stubs for sundgren._native.frontend_python."""

from collections.abc import Sequence
from os import PathLike

from .core_symbols import Span

def function_spans(
    python: str | PathLike[str], files: Sequence[tuple[str | PathLike[str], str]]
) -> list[list[Span] | None]: ...
