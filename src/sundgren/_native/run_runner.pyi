"""Stubs for sundgren._native.run_runner."""

from collections.abc import Mapping
from dataclasses import Field
from pathlib import Path
from typing import Any, ClassVar

from sundgren.run.presets import Preset
from sundgren.run.selectors import Selection

LOG_TAIL_LINES: int

class RunArtifacts:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("lcov", "junit", "returncode", "log")
    def __init__(self, lcov: Path, junit: Path | None, returncode: int, log: Path) -> None: ...
    @property
    def lcov(self) -> Path: ...
    @property
    def junit(self) -> Path | None: ...
    @property
    def returncode(self) -> int: ...
    @property
    def log(self) -> Path: ...
    def __hash__(self) -> int: ...

class RunFailure(Exception):
    """lcov output missing after the commands ran."""

    log_tail: str
    def __init__(self, message: str, log_tail: str) -> None: ...

class TemplateRunner:
    @property
    def preset(self) -> Preset: ...
    @property
    def python(self) -> str: ...
    @property
    def env_overrides(self) -> dict[str, str]: ...
    def __init__(
        self, preset: Preset, *, python: str, env_overrides: Mapping[str, str] | None = None
    ) -> None: ...
    def display_commands(self, selection: Selection) -> list[str]: ...
    def run(
        self, worktree: Path, selection: Selection, out_dir: Path, repeat: int = 1
    ) -> list[RunArtifacts]: ...

def adhoc_preset(
    commands: tuple[str, ...], lcov: str, junit: str | None, *, shell: bool
) -> Preset: ...
