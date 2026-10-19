"""Function and directory-tree rollups. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from typing import Literal

from sundgren._native import core_rollup as _impl

FunctionStatus = Literal[
    "went_dark",
    "lit_up",
    "partial_loss",
    "partial_gain",
    "mixed",
    "unchanged",
    "added",
    "removed",
]

FunctionDelta = _impl.FunctionDelta
Stats = _impl.Stats
TreeNode = _impl.TreeNode
function_deltas = _impl.function_deltas
build_tree = _impl.build_tree

__all__ = [
    "FunctionDelta",
    "FunctionStatus",
    "Stats",
    "TreeNode",
    "build_tree",
    "function_deltas",
]
