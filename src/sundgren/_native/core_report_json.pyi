"""Stubs for sundgren._native.core_report_json."""

from collections.abc import Mapping
from typing import Any

SCHEMA_ID: str
LINE_KEYS: tuple[str, ...]
BRANCH_KEYS: tuple[str, ...]
FUNCTION_KEYS: tuple[str, ...]

def dumps(report: Mapping[str, Any]) -> str: ...
