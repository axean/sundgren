from __future__ import annotations

import subprocess
import tomllib

from support import ROOT
from support.shed import SUNDGREN


def test_version() -> None:
    version = tomllib.loads((ROOT / "pyproject.toml").read_text("utf-8"))["project"]["version"]
    out = subprocess.run([SUNDGREN, "--version"], capture_output=True, text=True, check=True)
    assert out.stdout.strip() == f"sundgren {version}"
