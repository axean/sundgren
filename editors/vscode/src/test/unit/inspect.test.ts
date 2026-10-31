import * as assert from "node:assert/strict";

import type { LineFound } from "../../containment.js";
import {
  changed,
  characters,
  foldState,
  groupLines,
  segments,
  tally,
  textAt,
} from "../../inspect.js";

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

function found(number: number, how: LineFound["found"]): LineFound {
  return { ...line("a.py", number), found: how };
}

describe("segments", () => {
  const lines = [
    found(1, "comment"),
    found(2, "exact"),
    found(3, "fuzzy"),
    found(4, "exact"),
    found(5, "missing"),
    found(6, "missing"),
    found(7, "exact"),
  ];

  it("folds runs of same and comment lines, keeping the different ones", () => {
    assert.deepEqual(
      segments(lines, true).map((s) =>
        s.kind === "line" ? s.line.line : s.lines.map((l) => l.line),
      ),
      [[1, 2], 3, [4], 5, 6, [7]],
    );
  });

  it("shows every line when not folding", () => {
    assert.equal(segments(lines, false).length, 7);
    assert.deepEqual(segments([], true), []);
  });

  it("counts how lines were found", () => {
    assert.deepEqual(tally(lines), { exact: 3, fuzzy: 1, missing: 2, comment: 1 });
  });
});

/** `text` with what `changed` marks in brackets. */
function shown(text: string, other: string): string {
  const marks = changed(text, other);
  return characters(text)
    .map((character, index) => (marks.at(index) === true ? `[${character}]` : character))
    .join("");
}

describe("changed", () => {
  it("marks only what the other line doesn't have", () => {
    assert.equal(shown("(a + 1) ? 0 : 1", "a + 1 ? 0 : 1"), "[(]a + 1[)] ? 0 : 1");
    assert.equal(shown("abc", "axc"), "a[b]c");
    assert.equal(shown("same", "same"), "same");
    assert.equal(shown("tail", ""), "[t][a][i][l]");
    assert.deepEqual(changed("", "x"), []);
  });

  it("keeps characters whole", () => {
    assert.equal(shown("é = 1", "e = 1"), "[é] = 1");
  });

  it("falls back to the middle for very long lines", () => {
    const long = "x".repeat(600);
    assert.equal(changed(`${long}a${long}`, `${long}b${long}`).filter(Boolean).length, 1);
  });
});

describe("foldState", () => {
  it("opens and closes folds whichever way they start", () => {
    assert.deepEqual(foldState(new Set(), "item:a", true), {
      isOpen: true,
      toggleKey: "closed:item:a",
    });
    assert.equal(foldState(new Set(["closed:item:a"]), "item:a", true).isOpen, false);
    assert.deepEqual(foldState(new Set(), "item:a", false), { isOpen: false, toggleKey: "item:a" });
    assert.equal(foldState(new Set(["item:a"]), "item:a", false).isOpen, true);
  });
});
