"""sha256 helpers. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from sundgren._native import core_hashing as _impl

sha256_bytes = _impl.sha256_bytes
sha256_file = _impl.sha256_file
sha256_file_or_none = _impl.sha256_file_or_none
