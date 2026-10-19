"""JUnit XML -> {test_id: outcome}. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from sundgren._native import core_junit as _impl

parse_junit = _impl.parse_junit
