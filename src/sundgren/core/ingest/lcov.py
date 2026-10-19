"""lcov tracefile parser and dumper. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from sundgren._native import core_lcov as _impl

parse = _impl.parse
dump = _impl.dump
LcovIngestor = _impl.LcovIngestor
