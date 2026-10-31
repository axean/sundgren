"""The docs demo's graph (docs/_static/demo/graph.js) is current sundgren output."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema

from demo_graph import demo_graph, read

ROOT = Path(__file__).resolve().parents[2]


def test_demo_graph_is_current() -> None:
    committed = read()
    schema = json.loads((ROOT / "schema" / "graph.schema.json").read_text("utf-8"))
    jsonschema.validate(committed, schema)
    assert committed == demo_graph(), "regenerate it: uv run tools/demo_graph.py"
