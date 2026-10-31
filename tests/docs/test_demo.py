"""The docs demos' data (docs/_static/demo/graph.js, containment-data.js) is current sundgren
output."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema

import demo_containment
from demo_graph import demo_graph, read

ROOT = Path(__file__).resolve().parents[2]


def test_demo_graph_is_current() -> None:
    committed = read()
    schema = json.loads((ROOT / "schema" / "graph.schema.json").read_text("utf-8"))
    jsonschema.validate(committed, schema)
    assert committed == demo_graph(), "regenerate it: uv run tools/demo_graph.py"


def test_demo_containment_is_current() -> None:
    committed = demo_containment.read()
    assert [entry["branch"] for entry in committed["results"]] == demo_containment.BRANCHES
    relations = {entry["branch"]: entry["result"]["relation"] for entry in committed["results"]}
    assert relations == {
        "fitting-extract": "a-in-b",
        "plots-reviewed": "a-in-b",
        "dev-rebased": "same",
        "export-and-more": "overlap",
        "logging": "unrelated",
        **dict.fromkeys(demo_containment.UNRELATED, "unrelated"),
        "readme-typo": "empty",
    }
    assert committed == demo_containment.demo_containment(), (
        "regenerate it: uv run tools/demo_containment.py"
    )
