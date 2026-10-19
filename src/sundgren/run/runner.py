"""TemplateRunner: run a preset's commands in a worktree, N repeats."""

from __future__ import annotations

from sundgren._native import run_runner as _impl

LOG_TAIL_LINES = _impl.LOG_TAIL_LINES
RunArtifacts = _impl.RunArtifacts
RunFailure = _impl.RunFailure
TemplateRunner = _impl.TemplateRunner
adhoc_preset = _impl.adhoc_preset
