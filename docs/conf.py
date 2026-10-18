"""Sphinx config for the sundgren site (one page, built from docs/examples)."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "_ext"))

project = "sundgren"
author = "Axel E. Andersson"
copyright = "2026, Axel E. Andersson"

extensions = ["myst_parser", "sundgren_docs"]
myst_enable_extensions = ["alert"]
exclude_patterns = ["_build", "examples", "Thumbs.db", ".DS_Store"]
root_doc = "index"

html_theme = "shibuya"
html_title = "sundgren"
html_baseurl = "https://axean.github.io/sundgren/"
html_static_path = ["_static"]
html_copy_source = False
html_show_sourcelink = False
html_theme_options = {
    "accent_color": "teal",
    "page_layout": "compact",
    "github_url": "https://github.com/axean/sundgren",
    "show_ai_links": False,
}
html_context = {"source_type": "github", "source_user": "axean", "source_repo": "sundgren"}
