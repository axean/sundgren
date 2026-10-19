"""(Snapshots, Snapshots, LineMaps, outcomes) -> Delta. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from typing import Literal

from sundgren._native import core_diff as _impl

Side = Literal["base", "patched"]
LineCategory = Literal[
    "kept_hit",
    "kept_miss",
    "lost",
    "gained",
    "unmeasured",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
]
BranchCategory = Literal[
    "branch_kept_hit",
    "branch_kept_miss",
    "branch_lost",
    "branch_gained",
    "branch_new",
    "branch_deleted",
    "branch_flaky",
    "branch_unmeasured",
]

LineDelta = _impl.LineDelta
BranchDelta = _impl.BranchDelta
FlakyLine = _impl.FlakyLine
FileDelta = _impl.FileDelta
OutcomePair = _impl.OutcomePair
Delta = _impl.Delta
diff = _impl.diff

__all__ = [
    "BranchCategory",
    "BranchDelta",
    "Delta",
    "FileDelta",
    "FlakyLine",
    "LineCategory",
    "LineDelta",
    "OutcomePair",
    "Side",
    "diff",
]
