"""Build the shed fixture projects into throwaway git repos and run the CLI on them."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import Any

from support import FIXTURES

SUNDGREN = os.environ.get("SUNDGREN_BIN") or str(Path(sys.executable).with_name("sundgren"))

SHED_PY = FIXTURES / "shed_py"
SHED_RS = FIXTURES / "shed_rs"
PATCHES = FIXTURES / "patches"
SHED_PY_RUNS = FIXTURES / "shed_py_runs"
SCHEMA = (
    Path(__file__).resolve().parents[2]
    / "src"
    / "sundgren"
    / "core"
    / "report"
    / "delta.schema.json"
)

_IGNORE = shutil.ignore_patterns("__pycache__", "*.pyc", ".pytest_cache", ".coverage", "target")

# Fixed identity and dates keep commit shas stable; global/system config is ignored.
_GIT_ENV = {
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "shed",
    "GIT_AUTHOR_EMAIL": "shed@example.invalid",
    "GIT_COMMITTER_NAME": "shed",
    "GIT_COMMITTER_EMAIL": "shed@example.invalid",
    "GIT_AUTHOR_DATE": "2026-01-01T00:00:00+0000",
    "GIT_COMMITTER_DATE": "2026-01-01T00:00:00+0000",
}


# Variables that would redirect git, pytest, coverage or imports in the child processes.
_LEAKY_PREFIXES = ("GIT_", "PYTEST_", "COVERAGE_", "COV_CORE_")
_LEAKY_NAMES = frozenset({"PYTHONPATH"})


def clean_env(**overrides: str) -> dict[str, str]:
    """os.environ without git/pytest/coverage/PYTHONPATH variables, plus `overrides`."""
    env = {
        k: v
        for k, v in os.environ.items()
        if not k.startswith(_LEAKY_PREFIXES) and k not in _LEAKY_NAMES
    }
    return {**env, **overrides}


def git(repo: Path, *args: str) -> str:
    """Run git in `repo` isolated from the user's config; return stdout."""
    out = subprocess.run(
        ["git", *args],
        cwd=repo,
        env=clean_env(**_GIT_ENV),
        capture_output=True,
        text=True,
        check=True,
    )
    return out.stdout


def _make_repo(src: Path, dest: Path) -> Path:
    shutil.copytree(src, dest, ignore=_IGNORE)
    git(dest, "-c", "init.defaultBranch=main", "init", "-q")
    git(dest, "config", "user.name", "shed")
    git(dest, "config", "user.email", "shed@example.invalid")
    git(dest, "config", "commit.gpgsign", "false")
    git(dest, "add", "-A")
    git(dest, "commit", "-q", "-m", "shed")
    return dest


def make_shed_py(tmp_path: Path) -> Path:
    """Committed copy of fixtures/shed_py at tmp_path/shed_py."""
    return _make_repo(SHED_PY, tmp_path / "shed_py")


def make_shed_rs(tmp_path: Path) -> Path:
    """Committed copy of fixtures/shed_rs at tmp_path/shed_rs."""
    return _make_repo(SHED_RS, tmp_path / "shed_rs")


def commit_files(repo: Path, files: Mapping[str, str], message: str = "update") -> None:
    """Write `files` (repo-relative path -> text) and commit them."""
    for rel, text in files.items():
        path = repo / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", message)


def run_sundgren(
    *args: str | Path, gitconfig: Path | None = None, timeout: float = 600
) -> subprocess.CompletedProcess[str]:
    """`sundgren ARGS` in a clean env, then assert it left nothing behind."""
    strs = [str(a) for a in args]
    repo = Path(strs[strs.index("--repo") + 1])
    tmp = repo.parent / "tmp"  # private TMPDIR, so the leftover check sees only this run
    tmp.mkdir(exist_ok=True)
    env = clean_env(
        GIT_CONFIG_GLOBAL=str(gitconfig) if gitconfig else os.devnull,
        GIT_CONFIG_NOSYSTEM="1",
        TMPDIR=str(tmp),
    )
    proc = subprocess.run(
        [SUNDGREN, *strs],
        capture_output=True,
        text=True,
        timeout=timeout,
        env=env,
    )
    assert_no_leftovers(repo, tmp)
    return proc


def assert_no_leftovers(repo: Path, tmp: Path) -> None:
    """Only the main worktree is registered and no sundgren-* dir is left in `tmp`."""
    # Plain asserts here aren't rewritten by pytest, so they carry their own messages.
    porcelain = git(repo, "worktree", "list", "--porcelain").splitlines()
    worktrees = [w.removeprefix("worktree ") for w in porcelain if w.startswith("worktree ")]
    assert worktrees == [str(repo.resolve())], f"worktrees left behind: {worktrees[1:]}"
    left = sorted(p.name for p in tmp.glob("sundgren-*"))
    assert not left, f"temp dirs left behind in {tmp}: {left}"


def validate_report(report: Any) -> None:
    """Validate against delta.schema.json (draft 2020-12)."""
    from jsonschema import Draft202012Validator

    schema = json.loads(SCHEMA.read_text())
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(report)


def run_json(*args: str | Path, gitconfig: Path | None = None) -> tuple[dict[str, Any], str]:
    """Run with `--format json`, assert exit 0, validate; return (report, raw stdout)."""
    proc = run_sundgren(*args, "--format", "json", gitconfig=gitconfig)
    assert proc.returncode == 0, f"exit {proc.returncode}\nstderr:\n{proc.stderr}"
    report = json.loads(proc.stdout)
    validate_report(report)
    return report, proc.stdout
