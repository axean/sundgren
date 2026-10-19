"""Stubs for sundgren._native.run_guards."""

from collections.abc import Iterable
from pathlib import Path

def wrong_tree_paths(raw_paths: Iterable[str], worktree: Path, main_repo: Path) -> list[str]: ...
