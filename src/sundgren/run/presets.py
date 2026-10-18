"""Presets: command templates plus where their outputs land. Pure data."""

from __future__ import annotations

import tomllib
from collections.abc import Mapping
from dataclasses import dataclass
from importlib.resources.abc import Traversable
from pathlib import Path
from typing import Any, Literal


@dataclass(frozen=True, slots=True)
class Preset:
    name: str
    description: str
    commands: tuple[str, ...]  # run in order, cwd = worktree
    env: Mapping[str, str]  # values are templates
    lcov: str  # template for the lcov output path
    junit: str | None  # template for the JUnit output path
    selector: str | None  # selector id; None = run all unless --tests
    tests_style: Literal["args", "filter"]
    shell: bool = False
    hints: tuple[str, ...] = ()
    source: str = "built-in"  # where it was defined, shown by `sundgren presets`


class PresetError(ValueError):
    """A presets file or table is malformed."""


_BUILTINS: dict[str, Preset] = {}

_KEYS = frozenset(
    {"description", "commands", "env", "lcov", "junit", "selector", "tests_style", "shell", "hints"}
)


def register_builtin_presets(source: Traversable) -> None:
    """Register presets from a presets.toml shipped by a lang pack."""
    parsed = _parse_presets(_load_toml(source.read_bytes(), str(source)), str(source), "built-in")
    for name, preset in parsed.items():
        existing = _BUILTINS.get(name)
        if existing is not None and existing != preset:
            raise PresetError(f"{source}: preset {name!r}: already registered by another pack")
    _BUILTINS.update(parsed)


def builtin_presets() -> dict[str, Preset]:
    return dict(sorted(_BUILTINS.items()))


def user_presets(repo: Path) -> dict[str, Preset]:
    """Presets from sundgren.toml or [tool.sundgren.presets.*] in pyproject.toml."""
    out: dict[str, Preset] = {}
    pyproject = repo / "pyproject.toml"
    if pyproject.is_file():
        doc = _load_toml(pyproject.read_bytes(), str(pyproject))
        tool = doc.get("tool", {})
        sundgren = tool.get("sundgren", {}) if isinstance(tool, dict) else {}
        if not isinstance(sundgren, dict):
            raise PresetError(f"{pyproject}: [tool.sundgren] must be a table")
        out.update(_parse_presets(sundgren, str(pyproject), "pyproject.toml"))
    # sundgren.toml wins over pyproject.toml for the same name.
    dedicated = repo / "sundgren.toml"
    if dedicated.is_file():
        doc = _load_toml(dedicated.read_bytes(), str(dedicated))
        out.update(_parse_presets(doc, str(dedicated), "sundgren.toml"))
    return dict(sorted(out.items()))


def all_presets(repo: Path) -> dict[str, Preset]:
    """Built-ins overridden by user presets of the same name."""
    return dict(sorted({**_BUILTINS, **user_presets(repo)}.items()))


def _load_toml(data: bytes, file: str) -> dict[str, Any]:
    try:
        return tomllib.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as exc:
        raise PresetError(f"{file}: {exc}") from None


def _parse_presets(doc: Mapping[str, Any], file: str, source: str) -> dict[str, Preset]:
    """Presets from the `presets` table of `doc`; absent table = none."""
    table = doc.get("presets", {})
    if not isinstance(table, dict):
        raise PresetError(f"{file}: 'presets' must be a table")
    return {name: _parse_preset(name, raw, file, source) for name, raw in table.items()}


def _parse_preset(name: str, raw: object, file: str, source: str) -> Preset:
    def fail(problem: str) -> PresetError:
        return PresetError(f"{file}: preset {name!r}: {problem}")

    if not isinstance(raw, dict):
        raise fail("must be a table")
    unknown = sorted(set(raw) - _KEYS)
    if unknown:
        raise fail(f"unknown key(s) {', '.join(unknown)}")
    for key in ("commands", "lcov"):
        if key not in raw:
            raise fail(f"missing required key {key!r}")

    def opt_str(key: str) -> str | None:
        value = raw.get(key)
        if value is not None and not isinstance(value, str):
            raise fail(f"{key!r} must be a string")
        return value

    def str_list(key: str) -> tuple[str, ...]:
        value = raw.get(key, [])
        if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
            raise fail(f"{key!r} must be a list of strings")
        return tuple(value)

    commands = str_list("commands")
    if not commands:
        raise fail("'commands' must not be empty")
    env = raw.get("env", {})
    if not isinstance(env, dict) or not all(isinstance(v, str) for v in env.values()):
        raise fail("'env' must be a table of strings")
    tests_style = raw.get("tests_style", "args")
    if tests_style not in ("args", "filter"):
        raise fail("'tests_style' must be 'args' or 'filter'")
    shell = raw.get("shell", False)
    if not isinstance(shell, bool):
        raise fail("'shell' must be a boolean")
    lcov = opt_str("lcov")
    if not lcov:
        raise fail("'lcov' must be a non-empty string")
    return Preset(
        name=name,
        description=opt_str("description") or "",
        commands=commands,
        env=dict(env),
        lcov=lcov,
        junit=opt_str("junit"),
        selector=opt_str("selector"),
        tests_style=tests_style,
        shell=shell,
        hints=str_list("hints"),
        source=source,
    )
