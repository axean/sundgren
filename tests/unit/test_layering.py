"""core must not import run or lang; run must not import lang."""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

SRC = Path(__file__).resolve().parents[2] / "src" / "sundgren"

FORBIDDEN = {
    "core": ("sundgren.run", "sundgren.lang", "sundgren.cli"),
    "run": ("sundgren.lang", "sundgren.cli"),
}


def _imports(path: Path) -> list[str]:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    pkg_parts = ("sundgren", *path.relative_to(SRC).parent.parts)
    names: list[str] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names.extend(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom):
            if node.level:
                base = pkg_parts[: len(pkg_parts) - node.level + 1]
                module = ".".join(base + ((node.module,) if node.module else ()))
            else:
                module = node.module or ""
            names.append(module)
            names.extend(f"{module}.{alias.name}" for alias in node.names)
    return names


def _violations(layer: str) -> list[str]:
    bad = []
    for path in sorted((SRC / layer).rglob("*.py")):
        for name in _imports(path):
            if any(name == f or name.startswith(f + ".") for f in FORBIDDEN[layer]):
                bad.append(f"{path.relative_to(SRC)} imports {name}")
    return bad


@pytest.mark.parametrize("layer", sorted(FORBIDDEN))
def test_layer_boundaries(layer: str) -> None:
    assert _violations(layer) == []


def test_relative_import_resolution(tmp_path: Path) -> None:
    # Guard against the checker silently missing relative imports.
    fake = SRC / "core" / "x.py"
    tree_text = "from ..run import presets\n"
    tree = ast.parse(tree_text)
    node = tree.body[0]
    assert isinstance(node, ast.ImportFrom)
    pkg_parts = ("sundgren", *fake.relative_to(SRC).parent.parts)
    base = pkg_parts[: len(pkg_parts) - node.level + 1]
    assert ".".join(base + (node.module or "",)) == "sundgren.run"
