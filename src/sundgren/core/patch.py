"""Git-style unified diff parser. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from sundgren._native import core_patch as _impl

Hunk = _impl.Hunk
FilePatch = _impl.FilePatch
Patch = _impl.Patch
parse_patch = _impl.parse_patch

__all__ = ["FilePatch", "Hunk", "Patch", "parse_patch"]
