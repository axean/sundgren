// A hand-made sundgren.graph/3 document for the unit tests and for looking at the page without
// the binary. Time runs down, newest on top; one lane per grid column 0, 2, 4, ...
//
//   ○          f2   fix-x (in sync with origin/fix-x)
//   │   ●      d3   origin/dev (remote only)
//   ○   │      f1
// ╭─╯   │
// ●     │      m5   main (in sync with origin/main)
// │ ○   │      y2   feature-y
// │ │   ○      d2
// │ ○   │      y1
// ├─╯   │
// ●     │      m4
// ●     ╵      m3   v1.0, merges old-fix
// ├───────╮
// │       ○    o1   old-fix (merged)
// ├───────╯
// ●     ╷      m2
// │     ○      d1
// ●     │      m1   ┐
// ●     │      m0   ├ a fold
// ●     │      m-1  ┘
// ⊙     ⊙          both go on below the page

import { DOWN, LEFT, RIGHT, UP, type Cell, type Commit, type Graph, type Lane } from "../graph.js";

/** Cells drawn by segments; the first lane to touch a cell keeps it. */
export class GridBuilder {
  private readonly cells = new Map<string, { -readonly [K in keyof Cell]: Cell[K] }>();

  /** The same cells, in the order made. */
  private readonly made: Cell[] = [];

  private at(row: number, col: number): { sides: number; lane: number } | undefined {
    return this.cells.get(`${String(row)},${String(col)}`);
  }

  private touch(row: number, col: number, lane: number, sides: number): void {
    const cell = this.at(row, col);
    if (cell === undefined) {
      const made = { row, col, sides, lane };
      this.cells.set(`${String(row)},${String(col)}`, made);
      this.made.push(made);
    } else {
      cell.sides |= sides;
    }
  }

  /** A horizontal line in `row` from `from` to `to`, both centres. */
  across(lane: number, row: number, from: number, to: number): this {
    for (let col = from; col <= to; col++) {
      this.touch(row, col, lane, (col < to ? RIGHT : 0) | (col > from ? LEFT : 0));
    }
    return this;
  }

  /** A vertical line in `col` from `from` down to `to`, both centres. */
  down(lane: number, col: number, from: number, to: number): this {
    for (let row = from; row <= to; row++) {
      this.touch(row, col, lane, (row < to ? DOWN : 0) | (row > from ? UP : 0));
    }
    return this;
  }

  /** A vertical broken around a horizontal of `lane` in `row`: `╵─╷`. */
  via(lane: number, row: number, col: number): this {
    const above = this.at(row - 1, col);
    const below = this.at(row + 1, col);
    const middle = this.at(row, col);
    if (above !== undefined) {
      above.sides &= ~DOWN;
    }
    if (below !== undefined) {
      below.sides &= ~UP;
    }
    if (middle !== undefined) {
      middle.sides = LEFT | RIGHT;
      middle.lane = lane;
    }
    return this;
  }

  node(row: number, col: number, node: "commit" | "connector", commit?: number): this {
    const cell = this.cells.get(`${String(row)},${String(col)}`);
    if (cell !== undefined) {
      cell.node = node;
      if (commit !== undefined) {
        cell.commit = commit;
      }
    }
    return this;
  }

  build(): Cell[] {
    return this.made.toSorted((a, b) => a.row - b.row || a.col - b.col);
  }
}

/** A made-up 40-character commit id. */
export function oid(name: string): string {
  return Buffer.from(name).toString("hex").padEnd(40, "0").slice(0, 40);
}

function lane(fields: Partial<Lane> & Pick<Lane, "name" | "column" | "label_row" | "tip">): Lane {
  return {
    branch: fields.name.replace(/^origin\//u, ""),
    kind: "local",
    protected: false,
    pinned: false,
    merged: false,
    ahead: 0,
    behind: 0,
    ...fields,
  };
}

const HEIGHT = 20;

function commit(name: string, row: number, lanes: number[], parents: string[]): Commit {
  return {
    oid: oid(name),
    parents: parents.map((parent) => oid(parent)),
    author: row % 3 === 0 ? "Ada" : "Grace Hopper",
    email: row % 3 === 0 ? "ada@example.com" : "grace@example.com",
    committed: 1_767_225_600 + (HEIGHT - row) * 3600,
    subject: `commit ${name}`,
    body: row === 0 ? "Why it was done.\n\nMore on it." : "",
    refs: [],
    row,
    lanes,
  };
}

const MAIN = 0;
const FIX = 1;
const FEATURE = 2;
const DEV = 3;
const OLD = 4;

/** Oldest first. */
const COMMITS: Commit[] = [
  commit("m-1", 18, [MAIN], ["m-2"]),
  commit("m0", 17, [MAIN], ["m-1"]),
  commit("m1", 16, [MAIN], ["m0"]),
  commit("d1", 15, [DEV], ["d0"]),
  commit("m2", 14, [MAIN], ["m1"]),
  commit("o1", 12, [OLD], ["m2"]),
  { ...commit("m3", 10, [MAIN], ["m2", "o1"]), refs: ["v1.0"] },
  commit("m4", 9, [MAIN], ["m3"]),
  commit("y1", 7, [FEATURE], ["m4"]),
  commit("d2", 6, [DEV], ["d1"]),
  { ...commit("y2", 5, [FEATURE], ["y1"]), refs: ["feature-y"] },
  { ...commit("m5", 4, [MAIN], ["m4"]), refs: ["main", "origin/main"] },
  commit("f1", 2, [FIX], ["m5"]),
  { ...commit("d3", 1, [DEV], ["d2"]), refs: ["origin/dev"] },
  { ...commit("f2", 0, [FIX], ["f1"]), refs: ["fix-x", "origin/fix-x"] },
];

/** The index of commit `name`. */
export function index(name: string): number {
  return COMMITS.findIndex((c) => c.oid === oid(name));
}

const CELLS = new GridBuilder()
  // fix-x leaves main's tip.
  .down(FIX, 2, 0, 3)
  .across(FIX, 3, 0, 2)
  .down(MAIN, 0, 3, 19)
  // origin/dev comes from below the page.
  .down(DEV, 6, 1, 19)
  // feature-y leaves m4.
  .down(FEATURE, 4, 5, 8)
  .across(FEATURE, 8, 0, 4)
  // old-fix leaves m2 and is merged at m3, passing over origin/dev twice.
  .across(OLD, 11, 0, 8)
  .down(OLD, 8, 11, 13)
  .across(OLD, 13, 0, 8)
  .via(OLD, 11, 6)
  .via(OLD, 13, 6)
  .node(0, 2, "commit", index("f2"))
  .node(1, 6, "commit", index("d3"))
  .node(2, 2, "commit", index("f1"))
  .node(4, 0, "commit", index("m5"))
  .node(5, 4, "commit", index("y2"))
  .node(6, 6, "commit", index("d2"))
  .node(7, 4, "commit", index("y1"))
  .node(9, 0, "commit", index("m4"))
  .node(10, 0, "commit", index("m3"))
  .node(12, 8, "commit", index("o1"))
  .node(14, 0, "commit", index("m2"))
  .node(15, 6, "commit", index("d1"))
  .node(16, 0, "commit", index("m1"))
  .node(17, 0, "commit", index("m0"))
  .node(18, 0, "commit", index("m-1"))
  .node(19, 0, "connector")
  .node(19, 6, "connector")
  .build();

export const GRAPH: Graph = {
  schema: "sundgren.graph/3",
  repository: "/r",
  base: { name: "main", oid: oid("m5") },
  show: { mode: "local" },
  threshold: 0.9,
  page: { commits: 50, more: true, newer: false },
  branches: [
    { name: "fix-x", local: true, authors: ["ada@example.com"] },
    { name: "feature-y", local: true, authors: ["grace@example.com"] },
    { name: "dev", local: false, authors: ["ada@example.com", "grace@example.com"] },
    { name: "old-fix", local: true, authors: [] },
    { name: "far-off", local: false, authors: ["grace@example.com"] },
  ],
  lanes: [
    lane({
      name: "main",
      kind: "reference",
      column: 0,
      label_row: 4,
      remote: "origin/main",
      protected: true,
      pinned: true,
      tip: oid("m5"),
      before: 40,
    }),
    lane({
      name: "fix-x",
      column: 2,
      label_row: 0,
      family: 0,
      remote: "origin/fix-x",
      ahead: 2,
      fork: oid("m5"),
      tip: oid("f2"),
    }),
    lane({
      name: "feature-y",
      column: 4,
      label_row: 5,
      family: 1,
      only: "local",
      ahead: 2,
      behind: 1,
      fork: oid("m4"),
      tip: oid("y2"),
    }),
    lane({
      name: "origin/dev",
      kind: "remote",
      column: 6,
      label_row: 1,
      family: 2,
      only: "remote",
      ahead: 15,
      behind: 7,
      fork: oid("m-9"),
      tip: oid("d3"),
      before: 12,
    }),
    lane({
      name: "old-fix",
      column: 8,
      label_row: 12,
      family: 3,
      only: "local",
      merged: true,
      behind: 3,
      fork: oid("m2"),
      tip: oid("o1"),
    }),
  ],
  commits: COMMITS,
  folds: [{ lane: MAIN, rows: [16, 18], commits: [index("m1"), index("m0"), index("m-1")] }],
  grid: { width: 9, height: HEIGHT, cells: CELLS },
};

const GLYPHS = " ╵╶╰╷│╭├╴╯─┴╮┤┬┼";

/** `graph`'s grid as Unicode, unfolded, like `sundgren graph --format text` without labels. */
export function text(graph: Graph): string {
  const rows = Array.from({ length: graph.grid.height }, () =>
    Array.from({ length: graph.grid.width }, () => " "),
  );
  for (const cell of graph.grid.cells) {
    const isReference = graph.lanes[cell.lane]?.kind === "reference";
    let glyph = GLYPHS[cell.sides] ?? "?";
    if (cell.node === "connector") {
      glyph = "⊙";
    } else if (cell.node === "commit") {
      glyph = isReference ? "●" : "○";
    }
    const row = rows[cell.row];
    if (row !== undefined) {
      row[cell.col] = glyph;
    }
  }
  return rows.map((row) => row.join("").trimEnd()).join("\n");
}
