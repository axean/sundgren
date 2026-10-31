"""The docs demos' data: what the docs build writes with the current sundgren (docs/_ext)."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import jsonschema

import demo_containment
import demo_graph

ROOT = Path(__file__).resolve().parents[2]


def read(path: Path, prefix: str) -> Any:
    return json.loads(path.read_text("utf-8").removeprefix(prefix).removesuffix(";\n"))


def test_demo_graph(tmp_path: Path) -> None:
    out = tmp_path / "graph.js"
    demo_graph.write(out)
    schema = json.loads((ROOT / "schema" / "graph.schema.json").read_text("utf-8"))
    jsonschema.validate(read(out, demo_graph.PREFIX), schema)


def test_demo_containment(tmp_path: Path) -> None:
    out = tmp_path / "containment-data.js"
    demo_containment.write(out)
    data = read(out, demo_containment.PREFIX)
    branches = [demo_containment.TARGET, *demo_containment.BRANCHES]
    # Every branch as target, comments ignored and counted, every other branch compared with it.
    assert list(data["targets"]) == branches
    for target, modes in data["targets"].items():
        assert list(modes) == ["ignored", "counted"]
        for results in modes.values():
            assert [entry["branch"] for entry in results] == [b for b in branches if b != target]
    # With every line, for the inspect page.
    first = data["targets"]["dev"]["ignored"][0]["result"]
    assert first["comments"] == "ignored" and "lines_found" in first["a_in_b"]

    shown = data["targets"][data["target"]]["ignored"]
    relations = {entry["branch"]: entry["result"]["relation"] for entry in shown}
    assert relations == {
        "frame-crc": "a-in-b",
        "ringbuf-overwrite": "a-in-b",
        "restructure-filters-and-crc": "a-in-b",
        "docs-and-license-headers": "unrelated",
        "dev-rebased": "same",
        "csv-export-and-plots": "overlap",
        **dict.fromkeys(demo_containment.UNRELATED, "unrelated"),
        "fix-readme-typo": "empty",
    }
    counted = data["targets"]["dev"]["counted"]
    headers = next(e for e in counted if e["branch"] == "docs-and-license-headers")
    assert headers["result"]["a_in_b"]["containment"] > 0
