"""Sphinx extension for the sundgren docs: a collapsible block and the site stylesheet."""

from __future__ import annotations

from typing import Any

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.util.docutils import SphinxDirective
from sphinx.writers.html5 import HTML5Translator


class details(nodes.General, nodes.Element):  # noqa: N801 (docutils node naming)
    pass


def _visit_details(self: HTML5Translator, node: details) -> None:
    self.body.append('<details class="sg-details">')
    self.body.append(f"<summary>{self.encode(node['summary'])}</summary>")


def _depart_details(self: HTML5Translator, node: details) -> None:
    self.body.append("</details>")


class Details(SphinxDirective):
    """A closed <details> block; the argument is the summary line."""

    required_arguments = 1
    final_argument_whitespace = True
    has_content = True

    def run(self) -> list[nodes.Node]:
        node = details(summary=self.arguments[0])
        self.state.nested_parse(self.content, self.content_offset, node)
        return [node]


def setup(app: Sphinx) -> dict[str, Any]:
    app.add_node(details, html=(_visit_details, _depart_details))
    app.add_directive("details", Details)
    app.add_css_file("sundgren.css")
    return {"version": "1", "parallel_read_safe": True, "parallel_write_safe": True}
