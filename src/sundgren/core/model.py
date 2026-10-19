"""Coverage data model shared by every layer. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from typing import Literal

from sundgren._native import core_model as _impl

LineNo = int  # 1-based

Outcome = Literal["passed", "failed", "skipped", "xfailed", "xpassed", "error", "not_run"]

LineStatus = _impl.LineStatus
BranchKey = _impl.BranchKey
FunctionRecord = _impl.FunctionRecord
FileCoverage = _impl.FileCoverage
Snapshot = _impl.Snapshot
ReportWarning = _impl.ReportWarning
merge_file_coverage = _impl.merge_file_coverage
snapshot_to_dict = _impl.snapshot_to_dict

__all__ = [
    "BranchKey",
    "FileCoverage",
    "FunctionRecord",
    "LineNo",
    "LineStatus",
    "Outcome",
    "ReportWarning",
    "Snapshot",
    "merge_file_coverage",
    "snapshot_to_dict",
]
