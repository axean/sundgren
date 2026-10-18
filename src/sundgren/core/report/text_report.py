"""sundgren.delta/1 report -> terminal summary."""

from __future__ import annotations

from collections import Counter
from collections.abc import Mapping, Sequence
from typing import Any

from sundgren.core.report.json_report import BRANCH_KEYS, FUNCTION_KEYS, LINE_KEYS

TOP_FILES = 20

_LINE_KEYS = ("lost", "gained", "new_hit", "new_miss", "deleted_hit", "deleted_miss", "unmeasured")
_BRANCH_KEYS = ("lost", "gained", "new", "deleted", "flaky", "unmeasured")


def render_text(report: Mapping[str, Any], hints: Sequence[str] = ()) -> str:
    """Test changes, summary counts, top files with their functions, warnings, hints."""
    out: list[str] = []
    changed_tests = [t for t in report["tests"] if t["base"] != t["patched"]]
    if changed_tests:
        out.append("Tests with a changed outcome")
        out += [f"  {t['id']}: {t['base']} -> {t['patched']}" for t in changed_tests]
        out.append("")

    s = report["summary"]
    cov = s["coverage"]
    out += [
        "Summary",
        f"  lines      {_counts(s['lines'], (*LINE_KEYS, 'flaky', 'unmeasured'))}",
        f"  branches   {_counts(s['branches'], BRANCH_KEYS)}",
        f"  functions  {_counts(s['functions'], FUNCTION_KEYS)}",
        f"  coverage   {_pct(cov['base_pct'])} -> {_pct(cov['patched_pct'])}",
    ]
    if s["external_files"]:
        out.append(f"  external files (not in tree): {s['external_files']}")
    out.append("")

    files = [f for f in report["files"] if _changed(f)]
    if files:
        files.sort(key=lambda f: (-(_line_count(f, "lost") + _line_count(f, "gained")), f["path"]))
        out.append(f"Files (top {TOP_FILES} by lost + gained lines)")
        for f in files[:TOP_FILES]:
            out.append(f"  {f['path']}: {_file_counts(f)}")
            for fn in f["functions"]:
                if fn["status"] != "unchanged":
                    n = fn["lines"]
                    out.append(
                        f"    {fn['name']} ({fn['start']}-{fn['end']}): {fn['status']}, "
                        f"lost {n['lost']}, gained {n['gained']}"
                    )
        if len(files) > TOP_FILES:
            out.append(f"  ... and {len(files) - TOP_FILES} more")
    else:
        out.append("No coverage changes.")

    if report["warnings"]:
        out += ["", "Warnings"]
        out += [f"  [{w['code']}] {w['message']}" for w in report["warnings"]]
    if hints and (report["warnings"] or any(f["inferred_side"] for f in report["files"])):
        out += ["", "Hints"]
        out += [f"  - {h}" for h in hints]
    return "\n".join(out) + "\n"


def _counts(counts: Mapping[str, int], keys: Sequence[str]) -> str:
    return ", ".join(f"{k.replace('_', ' ')} {counts[k]}" for k in keys)


def _pct(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.1f}%"


def _changed(f: Mapping[str, Any]) -> bool:
    return (
        any(not e["category"].startswith("kept_") for e in f["lines"])
        or any(not b["category"].startswith("branch_kept_") for b in f["branches"])
        or any(fn["status"] != "unchanged" for fn in f["functions"])
        or bool(f["flaky"])
    )


def _line_count(f: Mapping[str, Any], category: str) -> int:
    return sum(1 for e in f["lines"] if e["category"] == category)


def _file_counts(f: Mapping[str, Any]) -> str:
    lines = Counter(e["category"] for e in f["lines"])
    branches = Counter(b["category"].removeprefix("branch_") for b in f["branches"])
    parts = [f"{k.replace('_', ' ')} {lines[k]}" for k in _LINE_KEYS if lines[k]]
    if f["flaky"]:
        parts.append(f"flaky {len(f['flaky'])}")
    branch_parts = [f"{k} {branches[k]}" for k in _BRANCH_KEYS if branches[k]]
    if branch_parts:
        parts.append("branches " + ", ".join(branch_parts))
    if f["inferred_side"]:
        parts.append(f"{f['inferred_side']} side inferred")
    return "; ".join(parts) or "functions only"
