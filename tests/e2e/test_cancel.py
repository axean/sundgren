"""SIGINT/SIGTERM at each phase: sundgren stops its commands, removes its worktrees, exits fast."""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
from collections.abc import Iterator
from pathlib import Path

import pytest

from support.shed import SUNDGREN, assert_no_leftovers, clean_env, commit_files, git

# The exit budget after the signal; the target is ~2 s.
BUDGET = 2.0
LCOV = "printf 'SF:f.txt\\nDA:1,1\\nend_of_record\\n' > {lcov}"


def make_repo(tmp_path: Path) -> tuple[Path, Path]:
    """A one-file repo and a patch for it."""
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "-c", "init.defaultBranch=main", "init", "-q")
    commit_files(repo, {"f.txt": "one\n"})
    patch = tmp_path / "p.patch"
    (repo / "f.txt").write_text("two\n")
    patch.write_text(git(repo, "diff"))
    git(repo, "checkout", "-q", "f.txt")
    return repo, patch


@pytest.fixture
def pids(tmp_path: Path) -> Iterator[Path]:
    """Where the test commands record their pids; whatever is left is killed afterwards."""
    path = tmp_path / "pids"
    yield path
    for pid in map(int, path.read_text().split() if path.exists() else []):
        if alive(pid):
            os.kill(pid, signal.SIGKILL)


def sleeper(pids: Path) -> str:
    """Shell that records its pid and a background child's, then sleeps."""
    return f"sleep 60 & echo $! >> {pids}; echo $$ >> {pids}; wait"


def start(repo: Path, patch: Path, *args: str) -> subprocess.Popen[str]:
    tmp = repo.parent / "tmp"
    tmp.mkdir(exist_ok=True)
    env = clean_env(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1", TMPDIR=str(tmp))
    return subprocess.Popen(
        [SUNDGREN, "run", "--repo", str(repo), "--patch", str(patch), *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )


def wait_for_pids(pids: Path, proc: subprocess.Popen[str], count: int = 2) -> list[int]:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if pids.exists() and len(lines := pids.read_text().split()) >= count:
            return [int(p) for p in lines]
        if proc.poll() is not None:
            _, err = proc.communicate()
            pytest.fail(f"sundgren exited {proc.returncode} early:\n{err}")
        time.sleep(0.02)
    pytest.fail("the test command never started")


def alive(pid: int) -> bool:
    """Running and not a zombie."""
    try:
        state = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
    except FileNotFoundError:
        return False
    return state != "Z"


def cancel(proc: subprocess.Popen[str], sig: signal.Signals) -> tuple[int, float, str]:
    """Send `sig` to sundgren only; return (exit code, seconds to exit, stderr)."""
    sent = time.monotonic()
    proc.send_signal(sig)
    try:
        _, err = proc.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        pytest.fail(f"sundgren still running 10 s after {sig.name}")
    return proc.returncode, time.monotonic() - sent, err


def assert_stopped(repo: Path, pids: list[int]) -> None:
    deadline = time.monotonic() + 1
    while any(map(alive, pids)) and time.monotonic() < deadline:
        time.sleep(0.02)
    assert not [p for p in pids if alive(p)], "test processes still running"
    assert_no_leftovers(repo, repo.parent / "tmp")


@pytest.mark.parametrize(("sig", "code"), [(signal.SIGTERM, 143), (signal.SIGINT, 130)])
def test_cancel_base_run(tmp_path: Path, pids: Path, sig: signal.Signals, code: int) -> None:
    repo, patch = make_repo(tmp_path)
    proc = start(repo, patch, "--shell", "--cmd", sleeper(pids), "--lcov", "{out}/l.info")
    running = wait_for_pids(pids, proc)
    got, took, err = cancel(proc, sig)
    assert got == code, err
    assert err == "sundgren: cancelled\n"
    assert took < BUDGET
    assert_stopped(repo, running)


def test_cancel_patched_run(tmp_path: Path, pids: Path) -> None:
    repo, patch = make_repo(tmp_path)
    script = f"case {{worktree}} in */patched) {sleeper(pids)};; *) {LCOV};; esac"
    proc = start(repo, patch, "--shell", "--cmd", script, "--lcov", "{out}/l.info")
    running = wait_for_pids(pids, proc)
    got, took, err = cancel(proc, signal.SIGTERM)
    assert got == 143, err
    assert took < BUDGET
    assert_stopped(repo, running)


def test_cancel_ignored_sigterm_is_killed(tmp_path: Path, pids: Path) -> None:
    repo, patch = make_repo(tmp_path)
    script = f"trap '' TERM; {sleeper(pids)}"
    proc = start(repo, patch, "--shell", "--cmd", script, "--lcov", "{out}/l.info")
    running = wait_for_pids(pids, proc)
    got, took, err = cancel(proc, signal.SIGTERM)
    assert got == 143, err
    assert took < BUDGET  # 1 s grace, then SIGKILL
    assert_stopped(repo, running)


def test_cancel_worktree_setup(tmp_path: Path, pids: Path) -> None:
    # git runs post-checkout during `git worktree add`.
    repo, patch = make_repo(tmp_path)
    hook = repo / ".git" / "hooks" / "post-checkout"
    hook.write_text(f"#!/bin/sh\n{sleeper(pids)}\n")
    hook.chmod(0o755)
    proc = start(repo, patch, "--cmd", "true", "--lcov", "l.info")
    running = wait_for_pids(pids, proc)
    got, took, err = cancel(proc, signal.SIGTERM)
    assert got == 143, err
    assert took < BUDGET
    assert_stopped(repo, running)


def test_cancel_removes_kept_worktrees(tmp_path: Path, pids: Path) -> None:
    repo, patch = make_repo(tmp_path)
    proc = start(
        repo,
        patch,
        "--keep-worktrees",
        "--shell",
        "--cmd",
        sleeper(pids),
        "--lcov",
        "{out}/l.info",
    )
    running = wait_for_pids(pids, proc)
    got, _, err = cancel(proc, signal.SIGTERM)
    assert got == 143, err
    assert_stopped(repo, running)


@pytest.mark.skipif(sys.platform != "linux", reason="PR_SET_PDEATHSIG is Linux only")
def test_sigkill_takes_the_direct_child_along(tmp_path: Path, pids: Path) -> None:
    repo, patch = make_repo(tmp_path)
    proc = start(repo, patch, "--shell", "--cmd", sleeper(pids), "--lcov", "{out}/l.info")
    _, shell = wait_for_pids(pids, proc)
    proc.kill()
    proc.communicate()
    deadline = time.monotonic() + 1
    while alive(shell) and time.monotonic() < deadline:
        time.sleep(0.02)
    # Its background sleep is a grandchild and escapes; the fixture kills it.
    assert not alive(shell)


def test_cancel_check_worktree(tmp_path: Path, pids: Path) -> None:
    repo, patch = make_repo(tmp_path)
    hook = repo / ".git" / "hooks" / "post-checkout"
    hook.write_text(f"#!/bin/sh\n{sleeper(pids)}\n")
    hook.chmod(0o755)
    proc = start(repo, patch, "--check", "--cmd", "true", "--lcov", "l.info")
    running = wait_for_pids(pids, proc)
    got, took, err = cancel(proc, signal.SIGTERM)
    assert got == 143, err
    assert took < BUDGET
    assert_stopped(repo, running)


def test_cancel_check_preset_checks(tmp_path: Path, pids: Path) -> None:
    repo, patch = make_repo(tmp_path)
    preset = (
        f'[presets.p]\ncommands = ["true"]\nlcov = "l"\nchecks = ["sh -c \'{sleeper(pids)}\'"]\n'
    )
    commit_files(repo, {"sundgren.toml": preset})
    proc = start(repo, patch, "--check", "--preset", "p")
    running = wait_for_pids(pids, proc)
    got, took, err = cancel(proc, signal.SIGTERM)
    assert got == 143, err
    assert took < BUDGET
    assert_stopped(repo, running)
