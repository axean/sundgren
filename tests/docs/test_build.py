"""Strict Sphinx build of the docs site (warnings are errors)."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import pytest

import docs_examples

DOCS = docs_examples.ROOT / "docs"

pytestmark = pytest.mark.skipif(sys.version_info < (3, 12), reason="Sphinx 9 needs Python 3.12")


def test_strict_build(tmp_path: Path) -> None:
    for module in ("sphinx", "myst_parser", "shibuya"):
        pytest.importorskip(module)
    out = tmp_path / "html"
    cmd = [sys.executable, "-m", "sphinx", "-W", "--keep-going", "-n", "-E", "-q", "-b", "html"]
    cmd += ["-d", str(tmp_path / "doctrees"), str(DOCS), str(out)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    assert proc.returncode == 0, proc.stderr
    html = (out / "index.html").read_text()
    # Every example is on the examples page.
    page = (DOCS / "examples.md").read_text()
    graphics = re.findall(r"^```\{sundgren-graphic\} (\S+)$", page, re.M)
    assert sorted(graphics) == [f"examples/{p.name}" for p in docs_examples.example_dirs()]
    assert (out / "examples.html").read_text().count('<div class="sg">') == len(graphics)
    assert "_static/sundgren.css" in html and (out / "_static" / "sundgren.css").is_file()
    assert '<link rel="canonical" href="https://axean.github.io/sundgren/' in html
    assert not (out / "examples").exists()
    assert sorted(p.name for p in out.glob("*.html")) == [
        "changelog.html",
        "examples.html",
        "genindex.html",
        "how-it-works.html",
        "index.html",
        "reference.html",
        "search.html",
    ]
    assert 'rel="shortcut icon" href="_static/favicon.ico"' in html
    assert 'class="announcement"' not in html  # no banner on a local build
    assert 'href="changelog.html"' in html
    version = re.search(
        r'^version = "(.+)"', (docs_examples.ROOT / "pyproject.toml").read_text(), re.M
    )
    assert version and f"{version.group(1)}" in (out / "changelog.html").read_text()
