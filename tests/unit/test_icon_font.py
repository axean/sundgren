from __future__ import annotations

from pathlib import Path

import icon_font

SHIPPED = Path(__file__).parents[2] / "editors" / "vscode" / "sundgren-icons.woff"


def test_shipped_font_is_current(tmp_path: Path) -> None:
    out = tmp_path / "icons.woff"
    icon_font.build(out)
    # Out of date: run `uv run tools/icon_font.py editors/vscode/sundgren-icons.woff`.
    assert out.read_bytes() == SHIPPED.read_bytes()
