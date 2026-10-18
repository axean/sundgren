"""Run every docs example's commands and compare the output with its recordings."""

from __future__ import annotations

import json
from collections.abc import Callable
from pathlib import Path

import pytest

import docs_examples
from docs_examples import Example

DIRS = docs_examples.example_dirs()


@pytest.mark.slow
@pytest.mark.parametrize("path", DIRS, ids=[p.name for p in DIRS])
def test_example_matches_recording(
    path: Path, produce: Callable[[Example], dict[str, str]]
) -> None:
    example = docs_examples.load(path)
    if text := docs_examples.diff(example, produce(example)):
        pytest.fail(
            f"{example.name}: output differs from the recording\n{text}\n"
            "If the change is intended, re-record with:\n"
            f"  uv run python tools/docs_examples.py --update {example.name}",
            pytrace=False,
        )


def test_required_languages_have_examples() -> None:
    languages = {docs_examples.load(p).language for p in DIRS}
    assert docs_examples.required_languages() <= languages


def test_skip_divide_recording_matches_page() -> None:
    """The facts the page's prose states about skip_divide hold in its recording."""
    report = json.loads((docs_examples.EXAMPLES / "skip_divide" / "report.json").read_text())
    [calc] = [f for f in report["files"] if f["path"] == "shed/calc.py"]
    lines = {ln["line"]: ln["category"] for ln in calc["lines"]}
    assert lines[5] == "kept_hit"  # def divide runs at import
    assert [n for n, c in lines.items() if c == "lost"] == [6, 7, 8]
    assert [(b["line"], b["category"]) for b in calc["branches"] if b["line"] == 6] == [
        (6, "branch_lost"),
        (6, "branch_lost"),
    ]
    assert {f["name"]: f["status"] for f in calc["functions"]}["divide"] == "went_dark"
    changed = [t for t in report["tests"] if t["base"] != t["patched"]]
    assert changed == [
        {"id": "tests.test_calc::test_divide", "base": "passed", "patched": "skipped"}
    ]
