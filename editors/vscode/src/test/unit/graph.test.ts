import * as assert from "node:assert/strict";

import { parseGraph } from "../../graph.js";
import { GRAPH, text } from "../graph-fixture.js";

type Fields = Record<string, unknown>;

/** `GRAPH` with `change` applied to a copy. */
function changed(change: (graph: Fields) => void): unknown {
  const copy = structuredClone(GRAPH) as unknown as Fields;
  change(copy);
  return copy;
}

function first(graph: Fields, key: "lanes" | "commits" | "folds"): Fields {
  const item = (Reflect.get(graph, key) as Fields[])[0];
  assert.ok(item !== undefined);
  return item;
}

function firstCell(graph: Fields): Fields {
  const cell = ((graph["grid"] as Fields)["cells"] as Fields[])[0];
  assert.ok(cell !== undefined);
  return cell;
}

describe("parseGraph", () => {
  it("accepts a sundgren.graph/3 document", () => {
    assert.deepEqual(parseGraph(structuredClone(GRAPH)), GRAPH);
  });

  it("reads the schema's nulls as missing fields", () => {
    // As sundgren prints it: null where there is nothing.
    const lane = JSON.parse(
      `{"family": null, "twin": null, "remote": null, "only": null, "score": null, "fork": null, "before": null}`,
    ) as Fields;
    const wire = changed((graph) => {
      graph["show"] = JSON.parse(`{"mode": "local", "like": null}`) as Fields;
      Object.assign(first(graph, "lanes"), lane);
    });
    const parsed = parseGraph(wire);
    assert.ok(parsed !== undefined);
    assert.deepEqual(parsed.show, { mode: "local" });
    const reference = GRAPH.lanes[0];
    assert.ok(reference !== undefined);
    const { before, remote, ...rest } = reference;
    assert.equal(before, 40);
    assert.equal(remote, "origin/main");
    assert.deepEqual(parsed.lanes[0], rest);
  });

  it("reads the like branch", () => {
    const like = { name: "fix-x", oid: "a".repeat(40) };
    const parsed = parseGraph(changed((graph) => (graph["show"] = { mode: "like", like })));
    assert.deepEqual(parsed?.show, { mode: "like", like });
  });

  it("the fixture draws like the sketch", () => {
    assert.equal(
      text(GRAPH),
      [
        "  ○",
        "  │   ○",
        "  ○   │",
        "╭─╯   │",
        "●     │",
        "│   ○ │",
        "│   │ ○",
        "│   ○ │",
        "├───╯ │",
        "●     │",
        "●     ╵",
        "├───────╮",
        "│       ○",
        "├───────╯",
        "●     ╷",
        "│     ○",
        "●     │",
        "●     │",
        "●     │",
        "⊙     ⊙",
      ].join("\n"),
    );
  });

  it("rejects anything else", () => {
    const broken: unknown[] = [
      undefined,
      [],
      changed((graph) => (graph["schema"] = "sundgren.graph/2")),
      changed((graph) => (graph["base"] = { name: "main", oid: "HEAD" })),
      changed((graph) => (graph["show"] = { mode: "target", like: undefined })),
      changed((graph) => (graph["page"] = { commits: 0, more: false })),
      changed((graph) => (graph["page"] = { commits: 50 })),
      changed((graph) => (graph["branches"] = [1])),
      changed((graph) => (first(graph, "commits")["body"] = undefined)),
      changed((graph) => (first(graph, "lanes")["kind"] = "tag")),
      changed((graph) => (first(graph, "lanes")["only"] = "both")),
      changed((graph) => (first(graph, "lanes")["family"] = 0.5)),
      changed((graph) => (first(graph, "lanes")["remote"] = 1)),
      changed((graph) => (first(graph, "commits")["refs"] = [{ name: "main" }])),
      changed((graph) => (first(graph, "commits")["author"] = { name: "Ada" })),
      changed((graph) => (first(graph, "folds")["rows"] = [18, 16])),
      changed((graph) => (first(graph, "folds")["rows"] = [16])),
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
      changed((graph) => (first(graph, "lanes")["label_row"] = 99)),
      changed((graph) => (first(graph, "lanes")["column"] = 99)),
      changed((graph) => (first(graph, "commits")["lanes"] = [99])),
      changed((graph) => (first(graph, "commits")["row"] = 99)),
      changed((graph) => (first(graph, "folds")["lane"] = 99)),
      changed((graph) => (first(graph, "folds")["commits"] = [99])),
      changed((graph) => (first(graph, "folds")["rows"] = [16, 99])),
      changed((graph) => (firstCell(graph)["lane"] = 99)),
      changed((graph) => (firstCell(graph)["col"] = 99)),
      changed((graph) => (firstCell(graph)["commit"] = 99)),
    ];
    for (const value of broken) {
      assert.equal(parseGraph(value), undefined, JSON.stringify(value));
    }
  });
});
