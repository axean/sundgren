"""The announcement banner (docs/_ext/sundgren_banner.py) on live and preview builds."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

import docs_examples

sys.path.insert(0, str(docs_examples.ROOT / "docs" / "_ext"))
from sundgren_banner import banner  # noqa: E402


def test_local_build_has_no_banner() -> None:
    assert banner("0.2.1", released=False, pr=0) == ""


def test_release_banner_links_pypi_and_github() -> None:
    html = banner("0.2.1", released=True, pr=0)
    assert "sundgren <strong>0.2.1</strong>, the latest release" in html
    assert 'href="https://pypi.org/project/sundgren/0.2.1/"' in html
    assert 'href="https://github.com/axean/sundgren/releases/tag/v0.2.1"' in html
    assert "sg-banner-release" in html


def test_preview_banner_links_the_pull_request() -> None:
    html = banner("0.2.1", released=False, pr=9)
    assert html.startswith('<span class="sg-banner sg-banner-preview"><strong>PREVIEW:</strong>')
    assert 'href="https://github.com/axean/sundgren/pull/9">pull request #9</a>' in html
    # A preview is never presented as a release, even if both were set.
    assert banner("0.2.1", released=True, pr=9) == html


@pytest.mark.skipif(sys.version_info < (3, 12), reason="Sphinx 9 needs Python 3.12")
def test_preview_build_shows_banner(tmp_path: Path) -> None:
    for module in ("sphinx", "myst_parser", "shibuya"):
        pytest.importorskip(module)
    out = tmp_path / "html"
    cmd = [sys.executable, "-m", "sphinx", "-W", "--keep-going", "-q", "-b", "html"]
    cmd += ["-d", str(tmp_path / "doctrees"), str(docs_examples.ROOT / "docs"), str(out)]
    env = {**os.environ, "SUNDGREN_DOCS_PR": "9"}
    proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
    assert proc.returncode == 0, proc.stderr
    for page in ("index.html", "examples.html", "changelog.html"):
        html = (out / page).read_text()
        assert 'class="announcement"' in html and "pull request #9" in html, page
