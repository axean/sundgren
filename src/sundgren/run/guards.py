"""Wrong-tree guard: coverage measured the main checkout instead of the worktree."""

from __future__ import annotations

import os
from collections.abc import Iterable
from pathlib import Path


def wrong_tree_paths(raw_paths: Iterable[str], worktree: Path, main_repo: Path) -> list[str]:
    """Raw SF paths outside `worktree` but inside `main_repo`, sorted."""
    tree = os.path.realpath(worktree)
    repo = os.path.realpath(main_repo)
    bad = set()
    for raw in raw_paths:
        if not os.path.isabs(raw):  # relative SF paths are relative to the worktree
            continue
        real = os.path.realpath(raw)
        # The worktree may itself sit inside main_repo (--workdir in the repo).
        if not _within(real, tree) and _within(real, repo):
            bad.add(raw)
    return sorted(bad)


def _within(path: str, root: str) -> bool:
    return os.path.commonpath([path, root]) == root
