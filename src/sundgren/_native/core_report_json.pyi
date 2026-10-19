"""Stubs for sundgren._native.core_report_json."""

from collections.abc import Iterable, Mapping
from typing import Any

from .core_diff import Delta
from .core_model import ReportWarning

SCHEMA_ID: str
LINE_KEYS: tuple[str, ...]
BRANCH_KEYS: tuple[str, ...]
FUNCTION_KEYS: tuple[str, ...]

def build_report(
    delta: Delta,
    meta: Mapping[str, Any],
    *,
    full: bool = False,
    counts: bool = False,
    extra_warnings: Iterable[ReportWarning] = (),
) -> dict[str, Any]: ...
def dumps(report: Mapping[str, Any]) -> str: ...
