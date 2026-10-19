"""Path normalization: raw SF paths -> repo-relative posix paths. Implemented in Rust."""

from __future__ import annotations

from sundgren._native import core_normalize as _impl

normalize_path = _impl.normalize_path
normalize_snapshot = _impl.normalize_snapshot
