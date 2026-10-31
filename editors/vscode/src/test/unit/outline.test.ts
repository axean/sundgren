import * as assert from "node:assert/strict";

import { flatten, parseOutline, segmentLines, type Outline, type Token } from "../../outline.js";

function outline(): Outline {
  return {
    schema: "sundgren.outline/0",
    files: [
      {
        path: "/r/m.py",
        language: "python",
        items: [
          {
            kind: "class",
            name: "A",
            start: 1,
            end: 3,
            children: [{ kind: "function", name: "f", start: 2, end: 3 }],
          },
        ],
        tokens: [{ kind: "ident", line: 1, end_line: 1, text: "class" }],
      },
      { path: "/r/notes.txt", items: [] },
    ],
  };
}

describe("parseOutline", () => {
  it("accepts a sundgren.outline/0 document", () => {
    const value = structuredClone(outline());
    assert.deepEqual(parseOutline(value), value);
  });

  it("rejects anything else", () => {
    const bad: unknown[] = [
      undefined,
      [],
      { ...outline(), schema: "sundgren.outline/1" },
      { schema: "sundgren.outline/0", files: [{ path: "x", items: [], language: "cobol" }] },
      { schema: "sundgren.outline/0", files: [{ path: "x", items: [{ kind: "f", name: "g" }] }] },
      {
        schema: "sundgren.outline/0",
        files: [
          { path: "x", items: [], tokens: [{ kind: "word", line: 1, end_line: 1, text: "" }] },
        ],
      },
      {
        schema: "sundgren.outline/0",
        files: [
          {
            path: "x",
            items: [{ kind: "f", name: "g", start: 1, end: 2, children: [{ start: 0 }] }],
          },
        ],
      },
    ];
    for (const value of bad) {
      assert.equal(parseOutline(value), undefined);
    }
  });
});

describe("flatten", () => {
  it("lists parents before children, with their depth", () => {
    const placed = flatten(outline().files[0]?.items ?? []);
    assert.deepEqual(
      placed.map((p) => [p.item.name, p.depth]),
      [
        ["A", 0],
        ["f", 1],
      ],
    );
  });
});

function token(kind: Token["kind"], text: string, line = 1): Token {
  return { kind, line, end_line: line, text };
}

describe("segmentLines", () => {
  it("splits the source into lines of tokens and the text between them", () => {
    const source = 'x = "a\nb"  # c\ny\n';
    const tokens = [
      token("ident", "x"),
      token("punct", "="),
      token("string", '"a\nb"'),
      token("comment", "# c", 2),
      token("ident", "y", 3),
    ];
    assert.deepEqual(segmentLines(source, tokens), [
      [
        { text: "x", kind: "ident" },
        { text: " ", kind: undefined },
        { text: "=", kind: "punct" },
        { text: " ", kind: undefined },
        { text: '"a', kind: "string" },
      ],
      [
        { text: 'b"', kind: "string" },
        { text: "  ", kind: undefined },
        { text: "# c", kind: "comment" },
      ],
      [{ text: "y", kind: "ident" }],
    ]);
  });

  it("skips tokens it can't find and keeps the rest of the text", () => {
    assert.deepEqual(segmentLines("a\n\nb", [token("ident", "zzz"), token("ident", "")]), [
      [{ text: "a", kind: undefined }],
      [],
      [{ text: "b", kind: undefined }],
    ]);
    assert.deepEqual(segmentLines("", []), [[]]);
  });
});
