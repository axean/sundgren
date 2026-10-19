"""Command templates: placeholder expansion, tokenwise {tests}."""

from __future__ import annotations

from sundgren._native import run_template as _impl

PLACEHOLDERS = _impl.PLACEHOLDERS
expand_value = _impl.expand_value
expand_command = _impl.expand_command
default_python = _impl.default_python
