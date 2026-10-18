"""The recorded Rust example still shows what the page says it shows."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

EXAMPLE = Path(__file__).resolve().parents[2] / "docs" / "examples" / "ignore_divide_rs"
LIB = "src/lib.rs"
LIB_SRC = (EXAMPLE / "project" / LIB).read_text().splitlines()


def line_of(needle: str) -> int:
    return next(no for no, line in enumerate(LIB_SRC, 1) if needle in line)


def report() -> dict[str, Any]:
    data: dict[str, Any] = json.loads((EXAMPLE / "report.json").read_text())
    return data


def test_divide_goes_dark() -> None:
    rep = report()
    assert rep["meta"]["preset"] == "cargo-tarpaulin"
    [lib] = rep["files"]
    assert lib["path"] == LIB
    status = {f["name"]: f["status"] for f in lib["functions"]}
    assert status == {"divide": "went_dark", "classify": "unchanged"}  # plain names, not mangled
    lines = {ln["line"]: ln["category"] for ln in lib["lines"]}
    body = [line_of("if b == 0"), line_of('return Err("b is zero")'), line_of("Ok(a / b)")]
    assert [lines[no] for no in body] == ["lost"] * len(body)
    # tarpaulin keeps the signature line hit although divide never runs.
    assert lines[line_of("pub fn divide")] == "kept_hit"


def test_no_branches_tests_or_test_code() -> None:
    rep = report()
    assert rep["summary"]["branches"] == {"deleted": 0, "gained": 0, "lost": 0, "new": 0}
    assert rep["files"][0]["branches"] == []
    assert rep["tests"] == []  # the preset writes no JUnit
    # tarpaulin leaves the #[cfg(test)] module out, including the line the patch changes.
    assert max(ln["line"] for ln in rep["files"][0]["lines"]) < line_of("#[cfg(test)]")
