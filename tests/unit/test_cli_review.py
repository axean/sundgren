"""sundgren review on the duplicate-functions fixture."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from sundgren.cli import EXIT_OK, main

FIXTURE = Path(__file__).resolve().parents[2] / "crates/sundgren-analyze/tests/fixtures/dupes_py"


def test_review_json(capsys: pytest.CaptureFixture[str]) -> None:
    lcov = str(FIXTURE / "coverage.lcov")
    assert main(["review", "--root", str(FIXTURE), "--lcov", lcov, "--format", "json"]) == EXIT_OK
    report = json.loads(capsys.readouterr().out)
    assert report["schema"] == "sundgren.review/1"
    groups = [[n["id"] for n in s["nodes"]] for s in report["suggestions"]]
    assert groups == [
        ["dupes/geometry.py::clamp", "dupes/util.py::clamp", "dupes/util.py::limit"],
        ["dupes/util.py::shout", "dupes/util.py::yell"],
    ]
    assert [s["confidence"] for s in report["suggestions"]] == ["medium", "low"]
    assert all(s["patch"].startswith("--- a/dupes/util.py") for s in report["suggestions"])


def test_review_text_without_coverage(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["review", "--root", str(FIXTURE)]) == EXIT_OK
    out = capsys.readouterr().out
    assert out.startswith("sundgren review: ") and "(no coverage given)" in out
    assert "+yell = shout" in out


def test_review_bad_root(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    assert main(["review", "--root", str(tmp_path / "missing")]) != EXIT_OK
    assert "not a directory" in capsys.readouterr().err
