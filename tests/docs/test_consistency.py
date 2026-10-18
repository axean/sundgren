"""The docs examples agree with the CI pins and the test fixtures."""

from __future__ import annotations

import re
import subprocess
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
EXAMPLES = ROOT / "docs" / "examples"
WORKFLOWS = ROOT / ".github" / "workflows"


def rust_pins() -> tuple[str, str]:
    """(rustc, cargo-tarpaulin) versions from the Rust example's `requires`."""
    manifest = tomllib.loads((EXAMPLES / "ignore_divide_rs" / "example.toml").read_text())
    expect = {req["run"]: req["expect"] for req in manifest["requires"]}
    rustc = re.fullmatch(r"rustc (\S+) ", expect["rustc --version"])
    tarpaulin = re.fullmatch(r"tarpaulin (\S+)\n", expect["cargo tarpaulin --version"])
    assert rustc and tarpaulin
    return rustc[1], tarpaulin[1]


def job(workflow: str, name: str) -> str:
    """Text of one job: from `  name:` to the next top-level job key."""
    text = (WORKFLOWS / workflow).read_text()
    found = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  \S|\Z)", text, re.M | re.S)
    assert found, f"no job {name} in {workflow}"
    return found[1]


@pytest.mark.parametrize(
    ("workflow", "name"), [("ci.yml", "e2e-rust"), ("ci.yml", "coverage"), ("docs.yml", "rust")]
)
def test_rust_pins_match_workflows(workflow: str, name: str) -> None:
    rustc, tarpaulin = rust_pins()
    steps = job(workflow, name)
    assert re.findall(r"dtolnay/rust-toolchain@(\S+)", steps) == [rustc]
    assert re.findall(r"cargo-tarpaulin@([\w.]+)", steps) == [tarpaulin]


def tracked(path: Path) -> dict[str, bytes]:
    """Git-tracked files under `path`, by path relative to it."""
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", str(path)],
        cwd=ROOT,
        capture_output=True,
        check=True,
    ).stdout
    rel = path.relative_to(ROOT).as_posix() + "/"
    names = [n.decode() for n in out.split(b"\0") if n]
    return {n.removeprefix(rel): (ROOT / n).read_bytes() for n in names}


@pytest.mark.parametrize(
    ("example", "fixture"), [("ignore_divide_rs", "shed_rs"), ("skip_divide", "shed_py")]
)
def test_example_project_matches_fixture(example: str, fixture: str) -> None:
    project = tracked(EXAMPLES / example / "project")
    assert project, f"{example}/project has no tracked files"
    assert project == tracked(ROOT / "tests" / "fixtures" / fixture)
