"""Strict Sphinx build of the docs site (warnings are errors)."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"

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
    for css in ("brand.css", "site.css"):
        assert f"_static/{css}" in html and (out / "_static" / css).is_file()
    assert (out / "_static" / "fonts" / "InterVariable.woff2").is_file()
    assert (out / "_static" / "history.css").is_file()
    assert '<link rel="canonical" href="https://axean.github.io/sundgren/' in html
    assert not (out / "examples").exists()
    assert sorted(p.name for p in out.glob("*.html")) == [
        "changelog.html",
        "containment.html",
        "development.html",
        "features.html",
        "genindex.html",
        "history.html",
        "index.html",
        "search.html",
    ]
    demo = out / "_static" / "demo"
    assert {p.name for p in demo.iterdir()} >= {
        "index.html",
        "demo.js",
        "graph.js",
        "containment.html",
        "containment-demo.js",
        "containment-data.js",
        "inspect.html",
        "inspect-demo.js",
    }
    assert 'src="_static/demo/index.html"' in (out / "history.html").read_text()
    assert 'data-site-root="https://axean.github.io/sundgren/"' in html
    assert (out / "_static" / "environments.js").is_file()
    assert 'id="sg-environments"' in (out / "development.html").read_text()
    assert 'rel="shortcut icon" href="_static/favicon.ico"' in html
    assert 'class="announcement"' not in html  # no banner on a local build
    assert 'href="changelog.html"' in html
    assert "Changelog" in (out / "changelog.html").read_text()
