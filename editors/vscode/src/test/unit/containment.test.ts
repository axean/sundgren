import * as assert from "node:assert/strict";

import {
  compareResults,
  median,
  parseContainment,
  type Containment,
  type Direction,
} from "../../containment.js";

/** A direction; without `containment` when it had no lines. */
function direction(containment?: number): Direction {
  const files = [{ path: "a.py", lines: 10, exact: 8, fuzzy: 1 }];
  const counts = { lines: 10, exact: 8, fuzzy: 1, in_order: 7, files };
  return containment === undefined
    ? counts
    : { ...counts, containment, exact_containment: 0.8, ordered_containment: 0.7 };
}

function result(containment?: number): Containment {
  const side = {
    rev: "x",
    oid: "1",
    fork: "0",
    commits: 2,
    files: 1,
    lines: 10,
    comment_lines: 2,
    files_read: 1,
  };
  return {
    schema: "sundgren.containment/0",
    repository: "/r",
    base: "main",
    base_oid: "0",
    threshold: 0.9,
    comments: "ignored",
    a: side,
    b: side,
    a_in_b: direction(containment),
    b_in_a: direction(0.1),
    relation: "a-in-b",
  };
}

describe("parseContainment", () => {
  it("accepts a sundgren.containment/0 document", () => {
    const value = structuredClone(result(0.9));
    assert.deepEqual(parseContainment(value), value);
    const empty = structuredClone(result());
    assert.deepEqual(parseContainment(empty), empty);
    const good = result(0.9);
    const read = {
      ...good,
      a_in_b: {
        ...good.a_in_b,
        items: [
          { path: "a.py", lines: 2, exact: 2, fuzzy: 0 },
          {
            path: "a.py",
            item: "Shape.area",
            kind: "function",
            lines: 8,
            exact: 6,
            fuzzy: 1,
            counterpart: { path: "b.py", item: "Geometry.area", lines: 7 },
          },
        ],
      },
    };
    assert.deepEqual(parseContainment(structuredClone(read)), read);
  });

  it("rejects anything else", () => {
    const good = result(0.9);
    const broken: unknown[] = [
      undefined,
      { ...good, schema: "sundgren.containment/1" },
      { ...good, relation: "cousins" },
      { ...good, a_in_b: { ...good.a_in_b, containment: 1.5 } },
      { ...good, a_in_b: { ...good.a_in_b, files: [{ path: "a.py" }] } },
      { ...good, b: { ...good.b, lines: -1 } },
      { ...good, b: { ...good.b, files_read: undefined } },
      { ...good, comments: "sometimes" },
      { ...good, a_in_b: { ...good.a_in_b, lines_found: [{ path: "a.py", line: 1 }] } },
      { ...good, a_in_b: { ...good.a_in_b, items: [{ path: "a.py", lines: 1 }] } },
      {
        ...good,
        a_in_b: {
          ...good.a_in_b,
          items: [
            { path: "a.py", lines: 1, exact: 1, fuzzy: 0, counterpart: { path: "b.py", item: 3 } },
          ],
        },
      },
    ];
    for (const value of broken) {
      assert.equal(parseContainment(value), undefined, JSON.stringify(value));
    }
  });
});

describe("median", () => {
  it("is the middle value, or the mean of the two middle ones", () => {
    assert.equal(median([]), undefined);
    assert.equal(median([0.3]), 0.3);
    assert.equal(median([0.9, 0.1, 0.2]), 0.2);
    assert.equal(median([0.4, 0.1, 0.2, 0.3]), 0.25);
  });
});

describe("compareResults", () => {
  it("puts the most contained first, then empty, then missing results", () => {
    const results = [result(0.2), undefined, result(), result(0.9)];
    const sorted = results.toSorted(compareResults);
    const label = (r: Containment | undefined): string =>
      r === undefined ? "none" : String(r.a_in_b.containment ?? "empty");
    assert.deepEqual(
      sorted.map((r) => label(r)),
      ["0.9", "0.2", "empty", "none"],
    );
  });
});
