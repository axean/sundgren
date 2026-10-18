from __future__ import annotations

import subprocess
import sys

from sundgren import __version__


def test_version() -> None:
    out = subprocess.run(
        [sys.executable, "-m", "sundgren", "--version"], capture_output=True, text=True, check=True
    )
    assert out.stdout.strip() == f"sundgren {__version__}"
