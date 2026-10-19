"""Patch -> per-file line maps between the base and patched trees. Implemented in Rust."""

from __future__ import annotations

from sundgren._native import core_linemap as _impl

LineMap = _impl.LineMap
identity = _impl.identity
build_linemaps = _impl.build_linemaps
linemap_for = _impl.linemap_for

__all__ = ["LineMap", "build_linemaps", "identity", "linemap_for"]
