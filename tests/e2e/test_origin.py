"""e2e: `sundgren origin` on two small git repositories."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

import jsonschema
import pytest

ROOT = Path(__file__).resolve().parents[2]
SUNDGREN = os.environ.get("SUNDGREN_BIN") or str(Path(sys.executable).with_name("sundgren"))
SCHEMA = json.loads((ROOT / "schema" / "origin.schema.json").read_text("utf-8"))

GIT_ENV = {
    **os.environ,
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "Ada",
    "GIT_AUTHOR_EMAIL": "ada@example.com",
    "GIT_COMMITTER_NAME": "Ada",
    "GIT_COMMITTER_EMAIL": "ada@example.com",
}


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, env=GIT_ENV, capture_output=True)


def commit(repo: Path, path: str, text: str) -> None:
    (repo / path).write_text(text)
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", path)


WORDS = ["port", "baud", "parity", "stopbits", "timeout", "buffer", "retries", "checksum"]


def function(name: str, count: int) -> str:
    """`count` lines, each too unlike the others to be a fuzzy match."""
    body = "".join(f"    {name}_{word} = read_{word}()\n" for word in WORDS[:count])
    return f"def {name}():\n{body}\n"


@pytest.fixture
def repos(tmp_path: Path) -> tuple[Path, Path]:
    """upstream: v1 has big.py (alpha, beta); then alpha moves to alpha.py. here: alpha, copied."""
    upstream = tmp_path / "upstream"
    here = tmp_path / "here"
    for repo in (upstream, here):
        repo.mkdir()
        git(repo, "init", "-q", "-b", "main")
    commit(upstream, "big.py", "# Header.\n" + function("alpha", 8) + function("beta", 8))
    git(upstream, "tag", "v1")
    git(upstream, "rm", "-q", "big.py")
    commit(upstream, "alpha.py", function("alpha", 4))
    commit(here, "copy.py", "# Header.\n" + function("alpha", 8))
    return here, upstream


def origin(*args: str) -> dict[str, Any]:
    proc = subprocess.run([SUNDGREN, "origin", *args], capture_output=True, text=True, timeout=60)
    assert proc.returncode == 0, proc.stderr
    doc: dict[str, Any] = json.loads(proc.stdout)
    jsonschema.validate(doc, SCHEMA)
    return doc


def test_origin_at_two_commits(repos: tuple[Path, Path]) -> None:
    here, upstream = repos
    doc = origin(str(here / "copy.py"), "--compare-repo", str(upstream), "--at", "v1,main", "--lines")
    assert doc["repository"] == str(here.resolve())
    assert [r["rev"] for r in doc["references"]] == ["v1", "main"]
    (file,) = doc["files"]
    assert (file["path"], file["lines"], file["comment_lines"]) == ("copy.py", 9, 1)
    at_v1, today = file["origins"]
    assert at_v1["best"]["path"] == "big.py"
    assert at_v1["best"]["share"] == 1.0
    assert at_v1["gap"] == 1.0
    assert at_v1["lines_found"][1]["at"] == {"path": "big.py", "line": 2}
    assert today["best"]["path"] == "alpha.py"
    assert today["best"]["share"] == 5 / 9


def test_origin_as_text(repos: tuple[Path, Path]) -> None:
    here, upstream = repos
    proc = subprocess.run(
        [SUNDGREN, "origin", "copy.py", "--compare-repo", str(upstream), "--at", "v1", "--format", "text"],
        capture_output=True,
        text=True,
        cwd=here,
        timeout=60,
    )
    assert proc.returncode == 0, proc.stderr
    lines = proc.stdout.splitlines()
    assert lines[0] == "copy.py: 9 lines"
    assert "100% from big.py" in lines[1]


def test_origin_of_an_unknown_commit(repos: tuple[Path, Path]) -> None:
    here, upstream = repos
    proc = subprocess.run(
        [SUNDGREN, "origin", str(here / "copy.py"), "--compare-repo", str(upstream), "--at", "nope"],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert proc.returncode == 2
    assert proc.stderr == "sundgren: nope is not a commit\n"
