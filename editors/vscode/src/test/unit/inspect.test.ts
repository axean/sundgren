import * as assert from "node:assert/strict";

import type { LineFound } from "../../containment.js";
import { groupLines, textAt } from "../../inspect.js";

function line(path: string, number: number, item?: string): LineFound {
  return {
    path,
    line: number,
    text: `${path} ${String(number)}`,
    found: "exact",
    ...(item !== undefined && { item }),
  };
}

describe("groupLines", () => {
  it("groups by file, then by item in order of first line", () => {
    const lines = [
      line("a.py", 1),
      line("a.py", 3, "A"),
      line("a.py", 4, "A.f"),
      line("a.py", 6, "A"),
      line("b.rs", 2, "g"),
    ];
    const grouped = groupLines(lines).map((file) => [
      file.path,
      file.items.map((group) => [group.item, group.lines.map((l) => l.line)]),
    ]);
    assert.deepEqual(grouped, [
      [
        "a.py",
        [
          [undefined, [1]],
          ["A", [3, 6]],
          ["A.f", [4]],
        ],
      ],
      ["b.rs", [["g", [2]]]],
    ]);
    assert.deepEqual(groupLines([]), []);
  });
});

describe("textAt", () => {
  it("finds a line's text by path and number", () => {
    const texts = textAt([line("a.py", 1), line("b.rs", 7)]);
    assert.equal(texts.get("b.rs:7"), "b.rs 7");
    assert.equal(texts.get("a.py:2"), undefined);
  });
});
