// A hand-made sundgren.graph/2 document, after the sketch in notes/history-map.md, for the unit
// tests and for looking at the page without the binary.
//
//      v1.0  origin/main
//       ▼     ▼
// main ⊙─●──●──●──●──●──●
//         │     │  │  │  ╰──○──○        fix-x
//         │     │  │  │  ╰──○           origin/fix-x
//         │     │  │  ╰──○──○           feature-y
//      ⊙──┼──○──┼──○────────○        origin/dev (vias at the two verticals)
//         ╰──○──╯                       old-fix (merged)

import { DOWN, LEFT, RIGHT, UP, type Cell, type Commit, type Graph, type Lane } from "../graph.js";

/** Cells drawn by segments; the first lane to touch a cell keeps it. */
export class GridBuilder {
  private readonly cells = new Map<string, { -readonly [K in keyof Cell]: Cell[K] }>();

  /** The same cells, in the order made. */
  private readonly made: Cell[] = [];

  private touch(row: number, col: number, lane: number, sides: number): void {
    const key = `${String(row)},${String(col)}`;
    const cell = this.cells.get(key);
    if (cell === undefined) {
      const made = { row, col, sides, lane };
      this.cells.set(key, made);
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

  /** A lane in `row` broken around a vertical of `lane` in `col`: `╴│╶`. */
  via(lane: number, row: number, col: number): this {
    const at = (c: number): { sides: number; lane: number } | undefined =>
      this.cells.get(`${String(row)},${String(c)}`);
    const left = at(col - 1);
    const right = at(col + 1);
    const middle = at(col);
    if (left !== undefined) {
      left.sides &= ~RIGHT;
    }
    if (right !== undefined) {
      right.sides &= ~LEFT;
    }
    if (middle !== undefined) {
      middle.sides = UP | DOWN;
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

/** Grid column of commit column `index`. */
export function nodeColumn(index: number): number {
  return 3 * index + 2;
}

const COLUMNS = 8;
const WIDTH = 3 * COLUMNS + 3;

function lane(fields: Partial<Lane> & Pick<Lane, "name" | "row" | "tip">): Lane {
  return {
    branch: fields.name.replace(/^origin\//u, ""),
    kind: "local",
    protected: false,
    pinned: false,
    merged: false,
    ahead: 0,
    behind: 0,
    after: false,
    ...fields,
  };
}

function commit(name: string, column: number, lanes: number[], parents: string[]): Commit {
  return {
    oid: oid(name),
    parents: parents.map((parent) => oid(parent)),
    author: "Ada",
    committed: 1_767_225_600 + column * 3600,
    subject: `commit ${name}`,
    refs: [],
    column: nodeColumn(column),
    lanes,
  };
}

const MAIN = 0;
const FIX = 1;
const ORIGIN_FIX = 2;
const FEATURE = 3;
const DEV = 4;
const OLD = 5;

const COMMITS: Commit[] = [
  commit("m0", 0, [MAIN], ["m-1"]),
  { ...commit("m1", 1, [MAIN], ["m0"]), refs: ["v1.0"] },
  commit("o1", 2, [OLD], ["m1"]),
  commit("m2", 2, [MAIN], ["m1"]),
  { ...commit("m3", 3, [MAIN], ["m2", "o1"]), refs: ["origin/main"] },
  commit("d1", 1, [DEV], ["d0"]),
  commit("m4", 4, [MAIN], ["m3"]),
  commit("d2", 4, [DEV], ["d1"]),
  { ...commit("m5", 5, [MAIN], ["m4"]), refs: ["main"] },
  commit("y1", 5, [FEATURE], ["m4"]),
  commit("y2", 6, [FEATURE], ["y1"]),
  commit("f1", 6, [FIX, ORIGIN_FIX], ["m5"]),
  { ...commit("f2", 7, [FIX], ["f1"]), refs: ["fix-x"] },
  { ...commit("d3", 7, [DEV], ["d2"]), refs: ["origin/dev"] },
];

const index = (name: string): number => COMMITS.findIndex((c) => c.oid === oid(name));

const n = nodeColumn;
const CELLS = new GridBuilder()
  // main, with a connector: it goes on before the frame.
  .across(MAIN, 0, 0, n(5))
  .node(0, 0, "connector")
  // fix-x and origin/fix-x leave main's tip.
  .down(FIX, n(5), 0, 2)
  .across(FIX, 1, n(5), n(7))
  .across(ORIGIN_FIX, 2, n(5), n(6))
  // feature-y leaves m4.
  .down(FEATURE, n(4), 0, 3)
  .across(FEATURE, 3, n(4), n(6))
  // old-fix leaves m1 and is merged at m3.
  .down(OLD, n(1), 0, 5)
  .across(OLD, 5, n(1), n(3))
  .down(OLD, n(3), 0, 5)
  // origin/dev comes in from before the frame and passes under old-fix twice.
  .across(DEV, 4, 0, n(7))
  .node(4, 0, "connector")
  .via(OLD, 4, n(1))
  .via(OLD, 4, n(3))
  .node(0, n(0), "commit", index("m0"))
  .node(0, n(1), "commit", index("m1"))
  .node(0, n(2), "commit", index("m2"))
  .node(0, n(3), "commit", index("m3"))
  .node(0, n(4), "commit", index("m4"))
  .node(0, n(5), "commit", index("m5"))
  .node(1, n(6), "commit", index("f1"))
  .node(1, n(7), "commit", index("f2"))
  .node(2, n(6), "commit", index("f1"))
  .node(3, n(5), "commit", index("y1"))
  .node(3, n(6), "commit", index("y2"))
  .node(4, n(2), "commit", index("d1"))
  .node(4, n(4), "commit", index("d2"))
  .node(4, n(7), "commit", index("d3"))
  .node(5, n(2), "commit", index("o1"))
  .build();

export const GRAPH: Graph = {
  schema: "sundgren.graph/2",
  repository: "/r",
  base: { name: "main", oid: oid("m5") },
  target: { name: "fix-x", oid: oid("f2") },
  threshold: 0.9,
  frame: { offset: 0, older: 4, columns: COLUMNS },
  candidates: 9,
  lanes: [
    lane({
      name: "main",
      kind: "reference",
      row: 0,
      protected: true,
      pinned: true,
      tip: oid("m5"),
      before: 40,
    }),
    lane({
      name: "fix-x",
      row: 1,
      family: 0,
      twin: ORIGIN_FIX,
      pinned: true,
      ahead: 2,
      fork: oid("m5"),
      tip: oid("f2"),
    }),
    lane({
      name: "origin/fix-x",
      kind: "remote",
      row: 2,
      family: 0,
      twin: FIX,
      ahead: 1,
      score: 1,
      fork: oid("m5"),
      tip: oid("f1"),
    }),
    lane({
      name: "feature-y",
      row: 3,
      family: 1,
      only: "local",
      ahead: 2,
      behind: 1,
      score: 0.62,
      fork: oid("m4"),
      tip: oid("y2"),
    }),
    lane({
      name: "origin/dev",
      kind: "remote",
      row: 4,
      family: 2,
      only: "remote",
      ahead: 15,
      behind: 7,
      score: 0.1,
      fork: oid("m-9"),
      tip: oid("d3"),
      before: 12,
    }),
    lane({
      name: "old-fix",
      row: 5,
      family: 3,
      only: "local",
      merged: true,
      behind: 3,
      fork: oid("m1"),
      tip: oid("o1"),
    }),
  ],
  commits: COMMITS,
  grid: { width: WIDTH, height: 6, cells: CELLS },
};

const GLYPHS = " ╵╶╰╷│╭├╴╯─┴╮┤┬┼";

/** `graph`'s grid as Unicode, like `sundgren graph --format text` without labels. */
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
