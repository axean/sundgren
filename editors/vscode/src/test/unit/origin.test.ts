import * as assert from "node:assert/strict";

import {
  WORK_TREE,
  inspection,
  parseOrigin,
  verdict,
  type Origin,
  type OriginDocument,
} from "../../origin.js";

const BEST = { path: "lib/big.py", exact: 8, fuzzy: 1, share: 0.9, distinctive: 0.7 };
const SECOND = { path: "lib/other.py", exact: 3, fuzzy: 0, share: 0.3, distinctive: 0.1 };

function document(origins: readonly Origin[]): OriginDocument {
  return {
    schema: "sundgren.origin/0",
    repository: "/here",
    other: "/upstream",
    threshold: 0.9,
    comments: "ignored",
    references: [{ rev: "v1", oid: "a1", files: 3 }],
    files: [{ path: "copy.py", lines: 10, comment_lines: 1, origins }],
  };
}

const LINES = [
  { path: "copy.py", line: 1, text: "# Header", found: "comment" },
  {
    path: "copy.py",
    line: 2,
    text: "def f():",
    found: "exact",
    at: { path: "lib/big.py", line: 4 },
  },
] as const;

describe("parseOrigin", () => {
  it("accepts sundgren origin's documents", () => {
    const full = document([
      { rev: "v1", oid: "a1", best: BEST, second: SECOND, gap: 0.6, lines_found: LINES },
      { rev: "main", oid: "b2" },
    ]);
    assert.deepEqual(parseOrigin(structuredClone(full)), full);
  });

  it("rejects anything else", () => {
    const good = document([{ rev: "v1", oid: "a1", best: BEST }]);
    for (const value of [
      undefined,
      {},
      { ...good, schema: "sundgren.origin/1" },
      { ...good, comments: "some" },
      { ...good, references: [{ rev: "v1" }] },
      document([{ rev: "v1", oid: "a1", best: { ...BEST, share: 1.5 } }]),
      document([{ rev: "v1", oid: "a1", gap: -0.1 }]),
      { ...good, files: [{ path: "a", lines: 1, comment_lines: 0 }] },
    ]) {
      assert.equal(parseOrigin(value), undefined, JSON.stringify(value));
    }
  });
});

describe("verdict", () => {
  it("calls a lead of 20 % clear", () => {
    assert.equal(verdict({ rev: "v1", oid: "a", best: BEST, gap: 0.2 }), "clear");
    assert.equal(verdict({ rev: "v1", oid: "a", best: BEST, gap: 0.05 }), "close");
    assert.equal(verdict({ rev: "v1", oid: "a" }), "none");
  });
});

describe("inspection", () => {
  it("reads as the file at the work tree looked up in the best file", () => {
    const origin = { rev: "v1", oid: "a1", best: BEST, gap: 0.9, lines_found: LINES };
    const made = document([origin]);
    const file = made.files[0];
    assert.ok(file !== undefined);
    const result = inspection(made, file, origin);
    assert.equal(result?.a.oid, WORK_TREE);
    assert.equal(result.b.oid, "a1");
    assert.equal(result.a_in_b.containment, 0.9);
    assert.deepEqual(result.a_in_b.files, [{ path: "copy.py", lines: 10, exact: 8, fuzzy: 1 }]);
    assert.equal(result.a_in_b.lines_found, LINES);
  });

  it("has none without a best file or its lines", () => {
    const made = document([]);
    const file = { path: "copy.py", lines: 10, comment_lines: 0, origins: [] };
    assert.equal(inspection(made, file, { rev: "v1", oid: "a1" }), undefined);
    assert.equal(inspection(made, file, { rev: "v1", oid: "a1", best: BEST }), undefined);
  });
});
