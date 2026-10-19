"""Delta -> sundgren.delta/1 JSON."""

from __future__ import annotations

import json
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import asdict
from importlib.resources import files
from typing import Any

from sundgren._native import core_report_json as _impl
from sundgren.core.diff import Delta, FileDelta, LineDelta
from sundgren.core.model import ReportWarning
from sundgren.core.rollup import TreeNode

SCHEMA_ID = "sundgren.delta/1"

LINE_KEYS = ("lost", "gained", "new_hit", "new_miss", "deleted_hit", "deleted_miss")
BRANCH_KEYS = ("lost", "gained", "new", "deleted")
FUNCTION_KEYS = (
    "went_dark",
    "lit_up",
    "partial_loss",
    "partial_gain",
    "mixed",
    "added",
    "removed",
)


def build_report(
    delta: Delta,
    meta: Mapping[str, Any],
    *,
    full: bool = False,
    counts: bool = False,
    extra_warnings: Sequence[ReportWarning] = (),
) -> dict[str, Any]:
    """The sundgren.delta/1 dict; by default only changes, `full` keeps everything."""
    warnings = sorted(set(delta.warnings) | set(extra_warnings), key=lambda w: (w.code, w.message))
    return {
        "schema": SCHEMA_ID,
        "meta": dict(meta),
        "tests": [
            {"id": t.id, "base": t.base, "patched": t.patched}
            for t in delta.tests
            if full or t.base != t.patched
        ],
        "summary": _summary(delta),
        "tree": _tree(delta.tree),
        "files": [
            _file(f, full, counts)
            for f in delta.files
            if full or f.changed or (counts and any(_count_changed(e) for e in f.lines))
        ],
        "warnings": [{"code": w.code, "message": w.message} for w in warnings],
        "calls": None,
    }


dumps = _impl.dumps


def load_schema() -> dict[str, Any]:
    text = files("sundgren.core.report").joinpath("delta.schema.json").read_text(encoding="utf-8")
    schema: dict[str, Any] = json.loads(text)
    return schema


def _summary(delta: Delta) -> dict[str, Any]:
    lines: Counter[str] = Counter(e.category for f in delta.files for e in f.lines)
    branches: Counter[str] = Counter(b.category for f in delta.files for b in f.branches)
    functions: Counter[str] = Counter(fn.status for f in delta.files for fn in f.functions)
    stats = delta.tree.stats
    return {
        "lines": {
            **{k: lines[k] for k in LINE_KEYS},
            "flaky": sum(len(f.flaky) for f in delta.files),
            "unmeasured": lines["unmeasured"],
        },
        "branches": {k: branches[f"branch_{k}"] for k in BRANCH_KEYS},
        "functions": {k: functions[k] for k in FUNCTION_KEYS},
        "coverage": {
            "base_pct": _pct(stats.hit_base, stats.executable_base),
            "patched_pct": _pct(stats.hit_patched, stats.executable_patched),
        },
        "external_files": delta.external_files,
    }


def _pct(hit: int, executable: int) -> float | None:
    return round(hit * 100 / executable, 1) if executable else None


def _tree(node: TreeNode) -> dict[str, Any]:
    out: dict[str, Any] = {
        "name": node.name,
        "path": node.path,
        "kind": node.kind,
        "stats": asdict(node.stats),
    }
    if node.kind == "dir":
        out["children"] = [_tree(c) for c in node.children]
    return out


def _file(f: FileDelta, full: bool, counts: bool) -> dict[str, Any]:
    return {
        "path": f.path,
        "base_path": f.base_path,
        "sha256": {"base": f.sha256[0], "patched": f.sha256[1]},
        "source_changed": f.source_changed,
        "inferred_side": f.inferred_side,
        "lines": [
            {"line": e.line, "base_line": e.base_line, "category": e.category, "hits": list(e.hits)}
            for e in f.lines
            if full or not e.category.startswith("kept_")
        ],
        "branches": [
            {
                "line": b.line,
                "base_line": b.base_line,
                "block": b.block,
                "branch": b.branch,
                "category": b.category,
                "taken": list(b.taken),
            }
            for b in f.branches
            if full or not b.category.startswith("branch_kept_")
        ],
        "functions": [
            {
                "name": fn.name,
                "start": fn.start,
                "end": fn.end,
                "status": fn.status,
                "lines": {"lost": fn.lost, "gained": fn.gained, "executable": fn.executable},
            }
            for fn in f.functions
            if full or fn.status != "unchanged"
        ],
        "flaky": [
            {"line": e.line, "base_line": e.base_line, "hits": list(e.hits), "sides": list(e.sides)}
            for e in f.flaky
        ],
        "count_changed": [
            {"line": e.line, "base_line": e.base_line, "hits": list(e.hits)}
            for e in f.lines
            if counts and _count_changed(e)
        ],
    }


def _count_changed(e: LineDelta) -> bool:
    return e.category == "kept_hit" and e.hits[0] != e.hits[1]
