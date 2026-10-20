"""Sphinx config for the sundgren site: home, examples, how it works, reference, changelog."""

import os
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "_ext"))

from sundgren_banner import preview_banner, release_label  # noqa: E402

_pyproject = tomllib.loads((Path(__file__).parents[1] / "pyproject.toml").read_text("utf-8"))
# docs.yml sets these: a release build (the live site), or the pull request of a preview.
_released = os.environ.get("SUNDGREN_DOCS_RELEASED") == "true"
_pr = int(os.environ.get("SUNDGREN_DOCS_PR") or 0)

project = "sundgren"
author = "Axel E. Andersson"
copyright = "2026, Axel E. Andersson"

extensions = ["myst_parser", "sundgren_docs"]
myst_enable_extensions = ["alert"]
myst_heading_anchors = 3
exclude_patterns = ["_build", "examples", "Thumbs.db", ".DS_Store"]
root_doc = "index"

html_theme = "shibuya"
html_title = "sundgren"
html_baseurl = "https://axean.github.io/sundgren/"
html_static_path = ["_static"]
templates_path = ["_templates"]
html_copy_source = False
html_show_sourcelink = False
html_favicon = "_static/brand/favicon.ico"
html_theme_options = {
    "accent_color": "orange",
    "light_logo": "_static/brand/sundgren-icon-128.png",
    "dark_logo": "_static/brand/sundgren-icon-128.png",
    "page_layout": "compact",
    "github_url": "https://github.com/axean/sundgren",
    "nav_links": [
        {"title": "Examples", "url": "examples"},
        {"title": "How it works", "url": "how-it-works"},
        {"title": "Reference", "url": "reference"},
        {"title": "Changelog", "url": "changelog"},
    ],
    "show_ai_links": False,
    "announcement": preview_banner(_pr),
}
html_context = {
    "source_type": "github",
    "source_user": "axean",
    "source_repo": "sundgren",
    "sundgren_release_label": release_label(_pyproject["project"]["version"], _released, _pr),
}
