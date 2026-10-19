"""Git worktrees for the base and patched sides."""

from __future__ import annotations

from sundgren._native import run_workspace as _impl

PatchApplyError = _impl.PatchApplyError
GitError = _impl.GitError
BadRefError = _impl.BadRefError
Workspace = _impl.Workspace
repo_root = _impl.repo_root
is_dirty = _impl.is_dirty
