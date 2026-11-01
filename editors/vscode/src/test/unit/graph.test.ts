import * as assert from "node:assert/strict";

import { parseGraph } from "../../graph.js";
import { GRAPH, text } from "../graph-fixture.js";

/** `GRAPH` with `change` applied to a copy. */
function changed(change: (graph: Record<string, unknown>) => void): unknown {
  const copy = structuredClone(GRAPH) as unknown as Record<string, unknown>;
  change(copy);
  return copy;
}

function first(graph: Record<string, unknown>, key: "lanes" | "commits"): Record<string, unknown> {
  const items = (key === "lanes" ? graph["lanes"] : graph["commits"]) as Record<string, unknown>[];
  const item = items[0];
  assert.ok(item !== undefined);
  return item;
}

function grid(graph: Record<string, unknown>): Record<string, unknown> {
  return graph["grid"] as Record<string, unknown>;
}

function firstCell(graph: Record<string, unknown>): Record<string, unknown> {
  const cell = (grid(graph)["cells"] as Record<string, unknown>[])[0];
  assert.ok(cell !== undefined);
  return cell;
}

describe("parseGraph", () => {
  it("accepts a sundgren.graph/2 document", () => {
    assert.deepEqual(parseGraph(structuredClone(GRAPH)), GRAPH);
  });

  it("reads the schema's nulls as missing fields", () => {
    // As sundgren prints it: null where there is nothing.
    const nulls = JSON.parse(
      `{"target": null, "frame": {"offset": 0, "older": null, "newer": null, "columns": 1}}`,
    ) as Record<string, unknown>;
    const lane = JSON.parse(
      `{"family": null, "twin": null, "only": null, "score": null, "fork": null, "before": null}`,
    ) as Record<string, unknown>;
    const wire = changed((graph) => {
      Object.assign(graph, nulls);
      Object.assign(first(graph, "lanes"), lane);
    });
    const parsed = parseGraph(wire);
    assert.ok(parsed !== undefined);
    assert.equal("target" in parsed, false);
    assert.deepEqual(parsed.frame, { offset: 0, columns: 1 });
    const reference = GRAPH.lanes[0];
    assert.ok(reference !== undefined);
    const { before, ...rest } = reference;
    assert.equal(before, 40);
    assert.deepEqual(parsed.lanes[0], rest);
  });

  it("the fixture draws like the sketch", () => {
    assert.equal(
      text(GRAPH),
      [
        "⊙─●──●──●──●──●──●",
        "     │     │  │  ├──○──○",
        "     │     │  │  ╰──○",
        "     │     │  ╰──○──○",
        "⊙───╴│╶─○─╴│╶─○────────○",
        "     ╰──○──╯",
      ].join("\n"),
    );
  });

  it("rejects anything else", () => {
    const broken: unknown[] = [
      undefined,
      [],
      changed((graph) => (graph["schema"] = "sundgren.graph/1")),
      changed((graph) => (graph["base"] = { name: "main", oid: "HEAD" })),
      changed((graph) => (graph["frame"] = { offset: 0, columns: 0 })),
      changed((graph) => (graph["candidates"] = -1)),
      changed((graph) => (first(graph, "lanes")["kind"] = "tag")),
      changed((graph) => (first(graph, "lanes")["only"] = "both")),
      changed((graph) => (first(graph, "lanes")["family"] = 0.5)),
      changed((graph) => (first(graph, "commits")["refs"] = [{ name: "main" }])),
      changed((graph) => (first(graph, "commits")["author"] = { name: "Ada" })),
      changed((graph) => (firstCell(graph)["sides"] = 16)),
      changed((graph) => (firstCell(graph)["node"] = "tag")),
    ];
    for (const value of broken) {
      assert.equal(parseGraph(value), undefined, JSON.stringify(value));
    }
  });

  it("rejects indexes that point at nothing", () => {
    const broken: unknown[] = [
      changed((graph) => (first(graph, "lanes")["twin"] = 99)),
      changed((graph) => (first(graph, "lanes")["row"] = 99)),
      changed((graph) => (first(graph, "commits")["lanes"] = [99])),
      changed((graph) => (firstCell(graph)["lane"] = 99)),
      changed((graph) => (firstCell(graph)["col"] = 99)),
      changed((graph) => (firstCell(graph)["commit"] = 99)),
    ];
    for (const value of broken) {
      assert.equal(parseGraph(value), undefined, JSON.stringify(value));
    }
  });
});
