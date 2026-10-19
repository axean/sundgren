"""Stubs for sundgren._native.run_template."""

from collections.abc import Mapping, Sequence

PLACEHOLDERS: tuple[str, ...]

def expand_value(template: str, values: Mapping[str, str]) -> str: ...
def expand_command(
    command: str, values: Mapping[str, str], tests: Sequence[str], *, shell: bool
) -> list[str] | str: ...
def default_python() -> str: ...
