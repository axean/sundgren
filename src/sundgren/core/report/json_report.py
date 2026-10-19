"""Delta -> sundgren.delta/1 JSON. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

import json
from importlib.resources import files
from typing import Any

from sundgren._native import core_report_json as _impl

SCHEMA_ID = _impl.SCHEMA_ID
LINE_KEYS = _impl.LINE_KEYS
BRANCH_KEYS = _impl.BRANCH_KEYS
FUNCTION_KEYS = _impl.FUNCTION_KEYS

build_report = _impl.build_report
dumps = _impl.dumps


def load_schema() -> dict[str, Any]:
    text = files("sundgren.core.report").joinpath("delta.schema.json").read_text(encoding="utf-8")
    schema: dict[str, Any] = json.loads(text)
    return schema


__all__ = [
    "BRANCH_KEYS",
    "FUNCTION_KEYS",
    "LINE_KEYS",
    "SCHEMA_ID",
    "build_report",
    "dumps",
    "load_schema",
]
