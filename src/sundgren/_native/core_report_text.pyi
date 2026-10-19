"""Stubs for sundgren._native.core_report_text."""

from collections.abc import Iterable, Mapping
from typing import Any

TOP_FILES: int

def render_text(report: Mapping[str, Any], hints: Iterable[str] = ()) -> str: ...
