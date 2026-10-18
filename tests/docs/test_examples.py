"""Run every docs example's commands and compare the output with its recordings."""

from __future__ import annotations

import json
import os
import subprocess
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


def git(cwd: Path, *args: str) -> str:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env |= {"GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}
    out = subprocess.run(["git", *args], cwd=cwd, env=env, capture_output=True, text=True)
    assert out.returncode == 0, out.stderr
    return out.stdout


def fake_example(path: Path) -> Example:
    (path / "project" / "pkg").mkdir(parents=True)
    (path / "project" / "pkg" / "a.py").write_text("x = 1\n")
    (path / "project" / ".gitignore").write_text("a.py\n")  # must not drop the tracked a.py
    (path / "p.patch").write_text("")
    return Example(path, "python", "p.patch", ("out.txt",), ("sundgren run --patch p.patch",), ())


def test_make_repo_commits_only_tracked_files(tmp_path: Path) -> None:
    example = fake_example(tmp_path / "ex")
    git(tmp_path, "init", "-q")
    git(tmp_path, "add", "-f", "ex/project")
    docs_examples.make_repo(example, tmp_path / "clean")
    (example.path / "project" / "stray.txt").write_text("untracked\n")
    docs_examples.make_repo(example, tmp_path / "dirty")

    for repo in (tmp_path / "clean", tmp_path / "dirty"):
        assert git(repo, "ls-files").split() == [".gitignore", "pkg/a.py"]
        assert git(repo, "status", "--porcelain").split() == ["??", "p.patch"]
    head = [git(tmp_path / r, "rev-parse", "HEAD") for r in ("clean", "dirty")]
    assert head[0] == head[1]


def test_project_files_needs_tracked_files(tmp_path: Path) -> None:
    example = fake_example(tmp_path / "ex")
    if subprocess.run(["git", "rev-parse"], cwd=tmp_path, capture_output=True).returncode == 0:
        pytest.skip(f"{tmp_path} is inside a git repo")
    with pytest.raises(docs_examples.ExampleError, match="must be in a git checkout of sundgren"):
        docs_examples.project_files(example)
    git(tmp_path, "init", "-q")
    with pytest.raises(docs_examples.ExampleError, match="git tracks no files"):
        docs_examples.project_files(example)
