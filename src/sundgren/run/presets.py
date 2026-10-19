"""Presets: command templates plus where their outputs land. Pure data."""

from __future__ import annotations

from sundgren._native import run_presets as _impl

Preset = _impl.Preset
PresetError = _impl.PresetError

# Filled by lang packs; the Rust functions read it at call time.
_BUILTINS: dict[str, Preset] = {}

register_builtin_presets = _impl.register_builtin_presets
builtin_presets = _impl.builtin_presets
user_presets = _impl.user_presets
all_presets = _impl.all_presets
