"""Wrong-tree guard: coverage measured the main checkout instead of the worktree."""

from __future__ import annotations

from sundgren._native import run_guards as _impl

wrong_tree_paths = _impl.wrong_tree_paths
