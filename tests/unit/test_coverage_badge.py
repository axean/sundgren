from __future__ import annotations

import json
from pathlib import Path

import pytest

import coverage_badge


@pytest.mark.parametrize(
    ("percent", "message", "color"),
    [
        (99.99, "99%", "brightgreen"),
        (90.0, "90%", "brightgreen"),
        (89.6, "89%", "green"),
        (80.0, "80%", "green"),
        (79.9, "79%", "yellowgreen"),
        (65.0, "65%", "yellow"),
        (55.0, "55%", "orange"),
        (12.3, "12%", "red"),
    ],
)
def test_badge(percent: float, message: str, color: str) -> None:
    assert coverage_badge.badge(percent) == {
        "schemaVersion": 1,
        "label": "coverage",
        "message": message,
        "color": color,
    }


def test_main_reads_coverage_json(tmp_path: Path) -> None:
    report = tmp_path / "coverage.json"
    report.write_text(json.dumps({"totals": {"percent_covered": 91.27}}))
    out = tmp_path / "badge.json"
    assert coverage_badge.main([str(report), str(out)]) == 0
    assert json.loads(out.read_text())["message"] == "91%"
