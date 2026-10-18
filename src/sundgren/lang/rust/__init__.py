"""Rust pack: cargo-llvm-cov and cargo-tarpaulin presets."""

from importlib.resources import files

from sundgren.run.presets import register_builtin_presets

register_builtin_presets(files(__name__) / "presets.toml")
