"""Stubs for sundgren._native.run_presets."""

from collections.abc import Mapping, Sequence
from importlib.resources.abc import Traversable
from pathlib import Path
from typing import Literal

class PresetError(ValueError):
    """A presets file or table is malformed."""

class Preset:
    name: str
    description: str
    commands: tuple[str, ...]
    env: Mapping[str, str]
    lcov: str
    junit: str | None
    selector: str | None
    tests_style: Literal["args", "filter"]
    shell: bool
    hints: tuple[str, ...]
    source: str
    def __init__(
        self,
        name: str,
        description: str,
        commands: Sequence[str],
        env: Mapping[str, str],
        lcov: str,
        junit: str | None,
        selector: str | None,
        tests_style: Literal["args", "filter"],
        shell: bool = False,
        hints: Sequence[str] = (),
        source: str = "built-in",
    ) -> None: ...

def register_builtin_presets(source: Traversable | Path) -> None: ...
def builtin_presets() -> dict[str, Preset]: ...
def user_presets(repo: Path) -> dict[str, Preset]: ...
def all_presets(repo: Path) -> dict[str, Preset]: ...
