"""Sphinx directives for the sundgren docs: the report graphic and a collapsible block."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path
from typing import Any

from docutils import nodes
from sphinx.application import Sphinx
from sphinx.util.docutils import SphinxDirective
from sphinx.writers.html5 import HTML5Translator

import sundgren_render


class SundgrenGraphic(SphinxDirective):
    """Render docs/examples/<name>/report.json over its project and patch."""

    required_arguments = 1
    has_content = False

    def run(self) -> list[nodes.Node]:
        _, example = self.env.relfn2path(self.arguments[0], self.env.docname)
        root = Path(example)
        manifest = self._read(root / "example.toml", tomllib.loads)
        report = self._read(root / "report.json", json.loads)
        patch_text = self._read(root / manifest["patch"], str)
        sources = {
            path: self._read(root / "project" / path, str)
            for path in sundgren_render.changed_paths(report)
        }
        try:
            html = sundgren_render.render(report, sources, patch_text)
        except (KeyError, ValueError) as exc:
            raise self.error(f"{root.name}: cannot render report: {exc}") from None
        return [nodes.raw("", html, format="html")]

    def _read(self, path: Path, parse: Any) -> Any:
        self.env.note_dependency(str(path))
        try:
            return parse(path.read_text(encoding="utf-8"))
        except OSError as exc:
            raise self.error(f"cannot read {path}: {exc}") from None


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
    app.add_directive("sundgren-graphic", SundgrenGraphic)
    app.add_node(details, html=(_visit_details, _depart_details))
    app.add_directive("details", Details)
    app.add_css_file("sundgren.css")
    return {"version": "1", "parallel_read_safe": True, "parallel_write_safe": True}
