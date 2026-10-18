"""Python pack: pytest preset, test-file selector, ast symbolizer."""

from importlib.resources import files

from sundgren.core.symbols import register_symbolizer
from sundgren.lang.python.selector import PytestFilesSelector
from sundgren.lang.python.symbolizer import PythonSymbolizer
from sundgren.run.presets import register_builtin_presets
from sundgren.run.selectors import register_selector

register_builtin_presets(files(__name__) / "presets.toml")
register_selector("python.pytest_files", PytestFilesSelector())
register_symbolizer((".py",), PythonSymbolizer())
