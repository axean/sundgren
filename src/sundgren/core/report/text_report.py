"""sundgren.delta/1 report -> terminal summary. Implemented in Rust (`sundgren._native`)."""

from __future__ import annotations

from sundgren._native import core_report_text as _impl

TOP_FILES = _impl.TOP_FILES
render_text = _impl.render_text

__all__ = ["TOP_FILES", "render_text"]
