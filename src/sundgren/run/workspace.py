"""Git worktrees for the base and patched sides."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from types import TracebackType


class PatchApplyError(Exception):
    """git apply failed; str(exc) is git's message."""


class GitError(RuntimeError):
    """A git command other than `git apply` failed; str(exc) is git's message."""


class BadRefError(GitError):
    """The base ref doesn't name a commit."""


# Pin the shape of the applied diff against user config (prefixes, colour, external tools).
_DIFF_CONFIG = (
    *("-c", "diff.noprefix=false"),
    *("-c", "diff.mnemonicPrefix=false"),
    *("-c", "diff.relative=false"),
    *("-c", "core.quotePath=true"),
)
_DIFF_ARGS = (
    "diff",
    "--cached",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "-M",
    "--unified=3",
    "--inter-hunk-context=0",
    "--diff-algorithm=myers",
    "--indent-heuristic",
    "--submodule=short",
    "--src-prefix=a/",
    "--dst-prefix=b/",
)


class Workspace:
    """Context manager: base and patched worktrees, removed on exit unless keep."""

    workdir: Path  # holds base/, patched/ and out/
    base: Path
    patched: Path
    base_sha: str
    applied_patch: str  # `git diff --cached` of patched/: what git apply actually changed
    offset_paths: tuple[str, ...]  # files with a hunk applied away from its header's line

    def __init__(
        self, repo: Path, base_ref: str, patch: Path, *, workdir: Path | None, keep: bool
    ) -> None:
        self.repo = repo
        self.base_ref = base_ref
        self.patch = patch
        self.keep = keep
        self._given_workdir = workdir
        self._created_workdir = False
        self._worktrees: list[Path] = []

    def __enter__(self) -> Workspace:
        try:
            self._setup()
        except BaseException:
            self._cleanup()
            raise
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None:
        if not self.keep:
            self._cleanup()

    def _setup(self) -> None:
        try:
            self.base_sha = _git(
                self.repo,
                "rev-parse",
                "--verify",
                f"{self.base_ref}^{{commit}}",
            )
        except GitError as exc:
            raise BadRefError(f"bad base ref {self.base_ref!r}: {exc}") from None
        if self._given_workdir is None:
            self.workdir = Path(tempfile.mkdtemp(prefix="sundgren-"))
            self._created_workdir = True
        else:
            self.workdir = self._given_workdir.absolute()
            self.workdir.mkdir(parents=True, exist_ok=True)
        self.base = self._add_worktree("base")
        self.patched = self._add_worktree("patched")
        self.offset_paths = self._apply()
        diff = subprocess.run(
            ["git", "-C", str(self.patched), *_DIFF_CONFIG, *_DIFF_ARGS],
            capture_output=True,
            check=False,
        )
        if diff.returncode != 0:
            raise GitError(diff.stderr.decode("utf-8", "replace").strip() or "git diff failed")
        self.applied_patch = diff.stdout.decode("utf-8", "surrogateescape")

    def _apply(self) -> tuple[str, ...]:
        """git apply --index; return the paths git reports as applied at an offset."""
        proc = subprocess.run(
            [
                *("git", "-C", str(self.patched), "-c", "core.quotePath=false"),
                *("apply", "--index", "--verbose", "--whitespace=nowarn"),
                str(self.patch.absolute()),
            ],
            capture_output=True,
            check=False,
            env={**os.environ, "LC_ALL": "C"},  # the offset report below is parsed
        )
        lines = proc.stderr.decode("utf-8", "replace").split("\n")
        if proc.returncode != 0:
            noise = ("Checking patch ", "Hunk #", "Applied patch ")
            message = "\n".join(ln for ln in lines if ln and not ln.startswith(noise))
            raise PatchApplyError(message or f"git apply exited {proc.returncode}")
        offset: set[str] = set()
        current = None
        for line in lines:
            if line.startswith("Checking patch ") and line.endswith("..."):
                current = line[len("Checking patch ") : -len("...")]
            elif line.startswith("Hunk #") and " succeeded at " in line and current:
                offset.add(current)
        return tuple(sorted(offset))

    def _add_worktree(self, name: str) -> Path:
        path = self.workdir / name
        _git(self.repo, "worktree", "add", "--detach", str(path), self.base_sha)
        self._worktrees.append(path)
        return path

    def _cleanup(self) -> None:
        for path in reversed(self._worktrees):
            proc = subprocess.run(
                ["git", "-C", str(self.repo), "worktree", "remove", "--force", str(path)],
                capture_output=True,
                check=False,
            )
            if proc.returncode != 0:
                shutil.rmtree(path, ignore_errors=True)
        self._worktrees.clear()
        subprocess.run(
            ["git", "-C", str(self.repo), "worktree", "prune"], capture_output=True, check=False
        )
        if self._created_workdir:
            shutil.rmtree(self.workdir, ignore_errors=True)
            self._created_workdir = False


def repo_root(path: Path) -> Path:
    return Path(_git(path, "rev-parse", "--show-toplevel"))


def is_dirty(repo: Path) -> bool:
    """Tracked changes only; untracked files don't affect the base ref."""
    return bool(_git(repo, "status", "--porcelain", "--untracked-files=no"))


def _git(repo: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        raise GitError(proc.stderr.strip() or f"git {args[0]} exited {proc.returncode}")
    return proc.stdout.strip()
