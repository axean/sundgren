"""sha256 helpers."""

from __future__ import annotations

import hashlib
from pathlib import Path


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def sha256_file_or_none(path: Path | None) -> str | None:
    """sha256 of a file, or None when the path is unknown or missing."""
    if path is None or not path.is_file():
        return None
    return sha256_file(path)
