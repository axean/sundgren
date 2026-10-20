"""Spec §12.2: `sundgren run` with the Rust presets on shed_rs, `#[ignore]` on the divide test."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any

import pytest

from support.shed import PATCHES, SHED_RS, make_shed_rs, run_sundgren, validate_report

pytestmark = pytest.mark.slow

LIB = "src/lib.rs"
LIB_SRC = (SHED_RS / LIB).read_text().splitlines()


def line_of(needle: str) -> int:
    return next(no for no, line in enumerate(LIB_SRC, 1) if needle in line)


DIVIDE_BODY = {line_of("if b == 0"), line_of('return Err("b is zero")'), line_of("Ok(a / b)")}


# tarpaulin needs ptrace and personality(ADDR_NO_RANDOMIZE); e.g. Docker's default seccomp denies
# them. Best guess at the wording, since no such box was available to check.
TRACE_DENIED = re.compile(r"ptrace|ASLR|EPERM|Operation not permitted", re.IGNORECASE)


def run_report(preset: str, *args: str | Path) -> dict[str, Any]:
    """Like run_json, but skip when tarpaulin can't trace in this environment."""
    proc = run_sundgren(*args, "--format", "json")
    if preset == "cargo-tarpaulin" and proc.returncode != 0:
        denied = [line for line in proc.stderr.splitlines() if TRACE_DENIED.search(line)]
        if denied:
            pytest.skip(f"tarpaulin cannot trace here: {denied[0].strip()}")
    assert proc.returncode == 0, f"exit {proc.returncode}\nstderr:\n{proc.stderr}"
    report: dict[str, Any] = json.loads(proc.stdout)
    validate_report(report)
    return report


def cargo_has(subcommand: str) -> bool:
    if shutil.which("cargo") is None:
        return False
    try:
        proc = subprocess.run(["cargo", subcommand, "--version"], capture_output=True, timeout=120)
    except (OSError, subprocess.TimeoutExpired):
        return False
    return proc.returncode == 0


@pytest.mark.parametrize(
    ("preset", "subcommand"),
    [("cargo-llvm-cov", "llvm-cov"), ("cargo-tarpaulin", "tarpaulin")],
)
def test_ignore_divide(tmp_path: Path, preset: str, subcommand: str) -> None:
    if not cargo_has(subcommand):
        pytest.skip(f"cargo {subcommand} not installed")
    repo = make_shed_rs(tmp_path)
    patch = PATCHES / "ignore_divide_rs.patch"
    report = run_report(preset, "run", "--repo", repo, "--patch", patch, "--preset", preset)

    assert report["meta"]["preset"] == preset
    assert report["meta"]["patch"]["files"] == [LIB]
    assert report["meta"]["selection"]["run_all"] is True
    assert report["tests"] == []  # no JUnit from either preset
    summary = report["summary"]
    assert summary["external_files"] == 0  # absolute SF paths resolved against the worktrees
    assert summary["lines"]["gained"] == 0
    assert summary["lines"]["lost"] >= len(DIVIDE_BODY)

    [lib] = report["files"]
    assert lib["path"] == LIB
    assert lib["source_changed"] is True
    lost = {ln["line"]: ln for ln in lib["lines"] if ln["category"] == "lost"}
    assert lost.keys() >= DIVIDE_BODY
    for no in DIVIDE_BODY:
        assert lost[no]["base_line"] == no
        base_hits, patched_hits = lost[no]["hits"]
        assert base_hits > 0 and patched_hits == 0

    # Spans come from lcov FN records (no Rust symbolizer); llvm-cov's symbols are demangled.
    [divide] = [f for f in lib["functions"] if f["start"] == line_of("pub fn divide")]
    assert divide["status"] == "went_dark"
    assert divide["name"] == ("shed::divide" if preset == "cargo-llvm-cov" else "divide")
    assert divide["lines"]["lost"] >= len(DIVIDE_BODY)
    if preset == "cargo-tarpaulin":
        assert report["warnings"] == []
        # tarpaulin keeps the signature line hit; FN has no end, so the span runs to classify.
        assert [(f["name"], f["status"], f["start"], f["end"]) for f in lib["functions"]] == [
            ("divide", "went_dark", line_of("pub fn divide"), line_of("pub fn classify") - 1)
        ]
        assert divide["lines"]["lost"] == len(DIVIDE_BODY)
