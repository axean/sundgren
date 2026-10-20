"""Build markers (docs/_ext/sundgren_banner.py): a release label, or a preview banner."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

import docs_examples

sys.path.insert(0, str(docs_examples.ROOT / "docs" / "_ext"))
from sundgren_banner import preview_banner, release_label  # noqa: E402


def test_local_build_has_neither() -> None:
    assert preview_banner(0) == ""
    assert release_label("0.2.1", released=False, pr=0) == ""


def test_release_label_links_github_and_pypi() -> None:
    html = release_label("0.2.1", released=True, pr=0)
    assert html.startswith('<span class="sg-release">v0.2.1 ')
    assert 'href="https://github.com/axean/sundgren/releases/tag/v0.2.1"' in html
    assert 'href="https://pypi.org/project/sundgren/0.2.1/"' in html


def test_preview_banner_links_the_pull_request() -> None:
    html = preview_banner(9)
    assert html.startswith('<span class="sg-preview"><strong>PREVIEW:</strong>')
    assert 'href="https://github.com/axean/sundgren/pull/9">pull request #9</a>' in html
    # A preview never shows a release label, even if it was built as a release.
    assert release_label("0.2.1", released=True, pr=9) == ""


def build(tmp_path: Path, **env: str) -> Path:
    for module in ("sphinx", "myst_parser", "shibuya"):
        pytest.importorskip(module)
    out = tmp_path / "html"
    cmd = [sys.executable, "-m", "sphinx", "-W", "--keep-going", "-q", "-b", "html"]
    cmd += ["-d", str(tmp_path / "doctrees"), str(docs_examples.ROOT / "docs"), str(out)]
    proc = subprocess.run(cmd, capture_output=True, text=True, env={**os.environ, **env})
    assert proc.returncode == 0, proc.stderr
    return out


@pytest.mark.skipif(sys.version_info < (3, 12), reason="Sphinx 9 needs Python 3.12")
def test_preview_build_has_banner(tmp_path: Path) -> None:
    out = build(tmp_path, SUNDGREN_DOCS_PR="9")
    for page in ("index.html", "examples.html", "changelog.html"):
        html = (out / page).read_text()
        assert 'class="announcement"' in html and "pull request #9" in html, page
        assert 'class="sg-release"' not in html, page


@pytest.mark.skipif(sys.version_info < (3, 12), reason="Sphinx 9 needs Python 3.12")
def test_release_build_has_label_before_search(tmp_path: Path) -> None:
    out = build(tmp_path, SUNDGREN_DOCS_RELEASED="true")
    html = (out / "index.html").read_text()
    assert 'class="announcement"' not in html
    label, search = html.find('class="sg-release"'), html.find('class="searchbox')
    assert 0 < label < search, (label, search)
