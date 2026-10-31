import * as assert from "node:assert/strict";

import { parseGraph, type Graph } from "../../graph.js";

const GRAPH: Graph = {
  schema: "sundgren.graph/1",
  repository: "/r",
  rev: "main",
  oid: "b".repeat(40),
  truncated: false,
  columns: 1,
  commits: [
    {
      oid: "b".repeat(40),
      parents: ["a".repeat(40)],
      author: { name: "Ada", email: "ada@example.com" },
      authored: 1,
      committed: 2,
      subject: "b",
      refs: [{ name: "main", kind: "branch" }],
      column: 0,
      color: 0,
      lines: [],
    },
  ],
  tail: [{ from: 0, to: 0, color: 0 }],
};

describe("parseGraph", () => {
  it("accepts a sundgren.graph/1 document", () => {
    assert.deepEqual(parseGraph(structuredClone(GRAPH)), GRAPH);
  });

  it("rejects anything else", () => {
    const broken: unknown[] = [
      undefined,
      [],
      { ...GRAPH, schema: "sundgren.graph/2" },
      { ...GRAPH, columns: -1 },
      { ...GRAPH, tail: [{ from: 0, to: 0 }] },
      { ...GRAPH, commits: [{ ...GRAPH.commits[0], refs: [{ name: "x", kind: "note" }] }] },
      { ...GRAPH, commits: [{ ...GRAPH.commits[0], author: "Ada" }] },
      { ...GRAPH, commits: [{ ...GRAPH.commits[0], lines: [{ from: 0.5, to: 0, color: 0 }] }] },
    ];
    for (const value of broken) {
      assert.equal(parseGraph(value), undefined, JSON.stringify(value));
    }
  });
});
