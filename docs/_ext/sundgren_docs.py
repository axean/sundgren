"""Sphinx extension for the sundgren docs: a collapsible block, the site stylesheet, and the
history panel demo's files from the VS Code extension."""

from __future__ import annotations

import shutil
from pathlib import Path
from typing import Any

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.config import Config
from sphinx.util.docutils import SphinxDirective
from sphinx.writers.html5 import HTML5Translator


class DetailsNode(nodes.General, nodes.Element):
    pass


def _visit_details(self: HTML5Translator, node: DetailsNode) -> None:
    self.body.append('<details class="sg-details">')
    self.body.append(f"<summary>{self.encode(node['summary'])}</summary>")


def _depart_details(self: HTML5Translator, node: DetailsNode) -> None:
    self.body.append("</details>")


class Details(SphinxDirective):
    """A closed <details> block; the argument is the summary line."""

    required_arguments = 1
    final_argument_whitespace = True
    has_content = True

    def run(self) -> list[nodes.Node]:
        node = DetailsNode(summary=self.arguments[0])
        self.state.nested_parse(self.content, self.content_offset, node)
        return [node]


EXTENSION = Path(__file__).resolve().parents[2] / "editors" / "vscode"
# The extension's history page files the demo (_static/demo/index.html) loads. webview.js is
# built by `npm run build` in editors/vscode; without it the demo shows how to build it.
DEMO_FILES = [EXTENSION / "media" / "history.css", EXTENSION / "dist" / "webview.js"]


def _copy_demo_files(app: Sphinx, exception: Exception | None) -> None:
    if exception is not None or app.builder.format != "html":
        return
    demo = Path(app.outdir) / "_static" / "demo"
    demo.mkdir(parents=True, exist_ok=True)
    for file in DEMO_FILES:
        if file.is_file():
            shutil.copyfile(file, demo / file.name)


def _add_environments_menu(app: Sphinx, config: Config) -> None:
    """The Development menu's script; it reads environments.json from the live site."""
    app.add_js_file("environments.js", **{"data-site-root": config.html_baseurl})


def setup(app: Sphinx) -> dict[str, Any]:
    app.add_node(DetailsNode, html=(_visit_details, _depart_details))
    app.add_directive("details", Details)
    app.add_css_file("sundgren.css")
    app.connect("build-finished", _copy_demo_files)
    app.connect("config-inited", _add_environments_menu)
    return {"version": "1", "parallel_read_safe": True, "parallel_write_safe": True}
