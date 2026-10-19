"""Stubs for sundgren._native.run_presets."""

from collections.abc import Mapping, Sequence
from dataclasses import Field
from importlib.resources.abc import Traversable
from pathlib import Path
from typing import Any, ClassVar, Literal

class PresetError(ValueError):
    """A presets file or table is malformed."""

class Preset:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = (
        "name",
        "description",
        "commands",
        "env",
        "lcov",
        "junit",
        "selector",
        "tests_style",
        "shell",
        "hints",
        "source",
    )
    @property
    def name(self) -> str: ...
    @property
    def description(self) -> str: ...
    @property
    def commands(self) -> tuple[str, ...]: ...
    @property
    def env(self) -> Mapping[str, str]: ...
    @property
    def lcov(self) -> str: ...
    @property
    def junit(self) -> str | None: ...
    @property
    def selector(self) -> str | None: ...
    @property
    def tests_style(self) -> Literal["args", "filter"]: ...
    @property
    def shell(self) -> bool: ...
    @property
    def hints(self) -> tuple[str, ...]: ...
    @property
    def source(self) -> str: ...
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
