"""e2e: `sundgren relate` on an upstream repository and a copy of it."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any

import jsonschema
import pytest

from e2e.test_graph import GIT_ENV, ROOT, SUNDGREN

SCHEMA = json.loads((ROOT / "schema" / "relate.schema.json").read_text("utf-8"))
GRAPH_SCHEMA = json.loads((ROOT / "schema" / "graph.schema.json").read_text("utf-8"))


def git(repo: Path, *args: str, day: int | None = None) -> str:
    env = dict(GIT_ENV)
    if day is not None:
        env["GIT_AUTHOR_DATE"] = env["GIT_COMMITTER_DATE"] = f"2026-01-{day:02}T12:00:00Z"
    proc = subprocess.run(
        ["git", "-C", str(repo), *args], check=True, env=env, capture_output=True, text=True
    )
    return proc.stdout


def commit(repo: Path, message: str, day: int) -> None:
    git(repo, "commit", "-q", "--allow-empty", "-m", message, day=day)


def sync(repo: Path, message: str, day: int) -> None:
    git(repo, "fetch", "-q", "origin")
    git(repo, "merge", "-q", "--no-ff", "-m", message, "origin/main", day=day)


@pytest.fixture
def pair(tmp_path: Path) -> tuple[Path, Path]:
    """upstream: u1 .. u7. copy: cloned at u2, own work, synced twice (u3, u4; u5), a side branch
    with u6, and then upstream adds u7."""
    upstream, copy = tmp_path / "upstream", tmp_path / "copy"
    upstream.mkdir()
    git(upstream, "init", "-q", "-b", "main")
    commit(upstream, "u1", 1)
    commit(upstream, "u2", 2)
    git(tmp_path, "clone", "-q", "upstream", "copy")
    commit(upstream, "u3", 3)
    commit(upstream, "u4", 4)
    commit(copy, "a1", 5)
    sync(copy, "m1", 6)
    commit(upstream, "u5", 7)
    commit(copy, "a2", 8)
    sync(copy, "m2", 9)
    commit(upstream, "u6", 10)
    git(copy, "switch", "-q", "-c", "side")
    sync(copy, "s1", 11)
    git(copy, "switch", "-q", "main")
    commit(upstream, "u7", 12)
    return copy, upstream


def relate(cache: Path, *args: str) -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "XDG_CACHE_HOME": str(cache)}
    return subprocess.run(
        [SUNDGREN, "relate", *args], capture_output=True, text=True, timeout=60, env=env
    )


def relate_json(cache: Path, *args: str) -> dict[str, Any]:
    proc = relate(cache, *args, "--format", "json")
    assert proc.returncode == 0, proc.stderr
    doc: dict[str, Any] = json.loads(proc.stdout)
    jsonschema.validate(doc, SCHEMA)
    return doc


def test_a_copy_and_its_upstream(pair: tuple[Path, Path], tmp_path: Path) -> None:
    copy, upstream = pair
    refs = (git(copy, "for-each-ref"), git(upstream, "for-each-ref"))
    doc = relate_json(tmp_path / "cache", str(upstream), "--repo", str(copy))
    assert (git(copy, "for-each-ref"), git(upstream, "for-each-ref")) == refs
    assert doc["related"] is True
    assert doc["shared"]["commits"] == 6
    this, other = doc["this"], doc["other"]
    assert (this["name"], other["name"]) == ("copy", "upstream")
    assert this["copy_point"]["commit"]["subject"] == "u2"
    assert this["copy_point"]["first_own"]["subject"] == "a1"
    # From u2 to u3, when upstream moved on.
    window = this["copy_point"]["window"]
    assert window["until"] - window["from"] == 86400
    assert doc["last_sync"]["subject"] == "u5"
    newest = doc["newest_shared"]
    assert newest["commit"]["subject"] == "u6"
    assert newest["on_this_base"] is False
    assert newest["this_refs"] == ["side", "origin/main"]
    merges = [(m["commit"]["subject"], m["count"]) for m in this["sync_merges"]]
    assert merges == [("m1", 2), ("m2", 1)]
    assert other["sync_merges"] == []
    assert doc["copy"] == "this"
    assert doc["direction"] == {"commits": 3, "this_first": 0, "other_first": 3, "same_time": 0}
    kinds = [m["kind"] for m in doc["marks"]]
    assert kinds.count("sync-merge") == 2 and "last-sync" in kinds


def test_as_text(pair: tuple[Path, Path], tmp_path: Path) -> None:
    copy, upstream = pair
    proc = relate(tmp_path / "cache", str(upstream), "--repo", str(copy), "--name", "up")
    assert proc.returncode == 0, proc.stderr
    assert "copy's main left up's main at " in proc.stdout
    assert "  the copy: made between 2026-01-02 12:00 and 2026-01-03 12:00\n" in proc.stdout
    assert "merges into copy's main from up's main:\n" in proc.stdout


def test_unrelated(tmp_path: Path) -> None:
    a, b = tmp_path / "a", tmp_path / "b"
    for repo in (a, b):
        repo.mkdir()
        git(repo, "init", "-q", "-b", "main")
        commit(repo, repo.name, 1)
    proc = relate(tmp_path / "cache", str(b), "--repo", str(a))
    assert proc.returncode == 0, proc.stderr
    assert proc.stdout.endswith("unrelated: no shared commits\n")
    doc = relate_json(tmp_path / "cache", str(b), "--repo", str(a))
    assert doc["related"] is False and doc["marks"] == []


def test_a_taken_name_is_a_usage_error(pair: tuple[Path, Path], tmp_path: Path) -> None:
    copy, upstream = pair
    proc = relate(tmp_path / "cache", str(upstream), "--repo", str(copy), "--name", "origin")
    assert proc.returncode == 2
    assert "--name" in proc.stderr


def test_graph_with_the_other_repository(pair: tuple[Path, Path], tmp_path: Path) -> None:
    copy, upstream = pair
    env = {**os.environ, "XDG_CACHE_HOME": str(tmp_path / "cache")}
    args = [SUNDGREN, "graph", "--repo", str(copy), "--other", str(upstream)]
    proc = subprocess.run([*args, "--format", "json"], capture_output=True, text=True, env=env)
    assert proc.returncode == 0, proc.stderr
    doc = json.loads(proc.stdout)
    jsonschema.validate(doc, GRAPH_SCHEMA)
    assert doc["other"]["base"]["name"] == "upstream/main"
    assert any(lane["name"] == "upstream/main" and lane["pinned"] for lane in doc["lanes"])
    marks = {c["subject"]: [m["kind"] for m in c.get("marks", [])] for c in doc["commits"]}
    assert marks["u2"] == ["copy-point"]
    assert marks["m1"] == ["sync-merge"]
    proc = subprocess.run([*args, "--format", "text"], capture_output=True, text=True, env=env)
    assert "[copy point of main]" in proc.stdout
    assert "[sync +2]" in proc.stdout
