"""e2e: run the built `sundgren` binary on small git repositories."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

import jsonschema
import pytest

ROOT = Path(__file__).resolve().parents[2]
SUNDGREN = os.environ.get("SUNDGREN_BIN") or str(Path(sys.executable).with_name("sundgren"))
SCHEMA = json.loads((ROOT / "schema" / "graph.schema.json").read_text("utf-8"))

# Fixed identity and dates keep commit shas stable; global/system config is ignored.
GIT_ENV = {
    **os.environ,
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "Ada",
    "GIT_AUTHOR_EMAIL": "ada@example.com",
    "GIT_COMMITTER_NAME": "Ada",
    "GIT_COMMITTER_EMAIL": "ada@example.com",
    "GIT_AUTHOR_DATE": "2026-01-01T00:00:00Z",
    "GIT_COMMITTER_DATE": "2026-01-01T00:00:00Z",
}


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, env=GIT_ENV, capture_output=True)


def sundgren(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run([SUNDGREN, *args], capture_output=True, text=True, timeout=60)


def graph(*args: str) -> dict[str, Any]:
    proc = sundgren("graph", *args)
    assert proc.returncode == 0, proc.stderr
    doc: dict[str, Any] = json.loads(proc.stdout)
    jsonschema.validate(doc, SCHEMA)
    return doc


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    """main: a - b - m, where m merges feature (a - f); open: work (from b), pushed one behind."""
    git(tmp_path, "init", "-q", "-b", "main")
    git(tmp_path, "commit", "-q", "--allow-empty", "-m", "a")
    git(tmp_path, "switch", "-q", "-c", "feature")
    git(tmp_path, "commit", "-q", "--allow-empty", "-m", "f")
    git(tmp_path, "switch", "-q", "main")
    git(tmp_path, "commit", "-q", "--allow-empty", "-m", "b")
    git(tmp_path, "merge", "-q", "--no-ff", "-m", "m", "feature")
    git(tmp_path, "tag", "v1")
    git(tmp_path, "switch", "-q", "-c", "work", "HEAD~1")
    git(tmp_path, "commit", "-q", "--allow-empty", "-m", "w1")
    git(tmp_path, "update-ref", "refs/remotes/origin/work", "HEAD")
    git(tmp_path, "commit", "-q", "--allow-empty", "-m", "w2")
    git(tmp_path, "switch", "-q", "main")
    return tmp_path


def test_version() -> None:
    version = tomllib.loads((ROOT / "pyproject.toml").read_text("utf-8"))["project"]["version"]
    assert sundgren("--version").stdout.strip() == f"sundgren {version}"


def test_map_of_a_merge(repo: Path) -> None:
    doc = graph("--repo", str(repo), "--target", "work")
    assert doc["repository"] == str(repo.resolve())
    assert doc["base"]["name"] == "main"
    assert doc["target"]["name"] == "work"
    lanes = {lane["name"]: lane for lane in doc["lanes"]}
    assert lanes["main"]["kind"] == "reference"
    assert lanes["main"]["protected"] is True
    assert lanes["feature"]["merged"] is True
    assert (lanes["work"]["ahead"], lanes["work"]["behind"]) == (2, 2)
    assert lanes["origin/work"]["kind"] == "remote"
    assert lanes["work"]["twin"] == doc["lanes"].index(lanes["origin/work"])
    assert [c["subject"] for c in doc["commits"]] == ["a", "f", "b", "w1", "m", "w2"]
    refs = {c["subject"]: c["refs"] for c in doc["commits"]}
    assert sorted(refs["m"]) == ["main", "v1"]


def test_map_as_text(repo: Path) -> None:
    proc = sundgren("graph", "--repo", str(repo), "--target", "work", "--format", "text")
    assert proc.returncode == 0, proc.stderr
    lines = proc.stdout.splitlines()
    assert lines[0] == "base main · target work · 1 of 1 branches in the frame"
    assert any("●" in line and "main" in line for line in lines)


def test_bad_base_is_a_usage_error(repo: Path) -> None:
    proc = sundgren("graph", "--repo", str(repo), "--base", "nope")
    assert proc.returncode == 2
    assert proc.stderr == "sundgren: nope is not a commit\n"


def test_not_a_repository(tmp_path: Path) -> None:
    proc = sundgren("graph", "--repo", str(tmp_path))
    assert proc.returncode == 1
    assert proc.stderr.startswith("sundgren: git: ")
