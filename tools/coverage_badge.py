"""Shields.io endpoint JSON for the coverage badge, from a `cargo llvm-cov report --json`.

Usage: coverage_badge.py LLVM_COV_JSON OUT_JSON
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

COLORS = ((90, "brightgreen"), (80, "green"), (70, "yellowgreen"), (60, "yellow"), (50, "orange"))


def badge(percent: float) -> dict[str, object]:
    whole = int(percent)  # truncate, so the badge never overstates
    color = next((c for limit, c in COLORS if whole >= limit), "red")
    return {"schemaVersion": 1, "label": "coverage", "message": f"{whole}%", "color": color}


def main(argv: list[str]) -> int:
    report, out = (Path(a) for a in argv)
    percent = json.loads(report.read_text())["data"][0]["totals"]["lines"]["percent"]
    out.write_text(json.dumps(badge(percent)) + "\n")
    print(f"coverage {percent:.1f}%")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
