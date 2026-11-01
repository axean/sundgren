// Where things go in the History map's SVG. One grid cell is one character cell of the Unicode
// rendering; lines run from cell centres to the connected sides. No DOM.

import { DOWN, LEFT, RIGHT, UP, type Cell, type Graph } from "../graph.js";

export const CELL_WIDTH = 9;
export const CELL_HEIGHT = 22;
export const NODE_RADIUS = 4.5;
/** Room left of the grid for the counts of the left connectors. */
export const GUTTER = 36;
/** Grid cells per commit column, and cells besides them (the connector columns). */
const CELLS_PER_COLUMN = 3;
const EXTRA_CELLS = 3;
/** Marker labels: most rows, longest name, and the width of one character. */
const MAX_LABEL_ROWS = 3;
const MAX_LABEL_LENGTH = 24;
export const LABEL_CHARACTER_WIDTH = 6.6;

/** The most commit columns that fit in `pixels` of grid (the gutter not included). */
export function columnsFor(pixels: number): number {
  const cells = Math.floor(pixels / CELL_WIDTH);
  return Math.max(1, Math.floor((cells - EXTRA_CELLS) / CELLS_PER_COLUMN));
}

/** Left edge of grid column `col`. */
export function cellX(col: number): number {
  return GUTTER + col * CELL_WIDTH;
}

/** Numbers in path data: no float noise. */
function n(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** A `▼` above the grid at a reference commit, with the refs there that aren't lanes. */
export interface Marker {
  readonly col: number;
  readonly names: readonly string[];
  /** The label as shown (cut short if long), and its row (0 nearest the grid). */
  readonly label: string;
  readonly labelRow: number;
}

/**
The markers above the grid, as in `sundgren graph --format text`: refs on reference commits whose
names aren't drawn lanes (tags, a remote twin with nothing of its own). `▼` and label share a
row; a label that would overlap the one before it goes a row higher.
*/
export function markers(graph: Graph): Marker[] {
  const reference = graph.lanes.findIndex((lane) => lane.kind === "reference");
  const drawn = new Set(graph.lanes.map((lane) => lane.name));
  const found = graph.commits
    .filter((commit) => commit.lanes.includes(reference))
    .map((commit) => ({
      col: commit.column,
      names: commit.refs.filter((name) => !drawn.has(name)),
    }))
    .filter((marker) => marker.names.length > 0)
    .toSorted((a, b) => a.col - b.col);
  // The right end of the last label in each row.
  const ends = new Map<number, number>();
  return found.map((marker) => {
    const joined = marker.names.join(", ");
    const label =
      joined.length > MAX_LABEL_LENGTH ? `${joined.slice(0, MAX_LABEL_LENGTH - 1)}…` : joined;
    const start = cellX(marker.col);
    const end = start + (label.length + 1) * LABEL_CHARACTER_WIDTH + CELL_WIDTH;
    let labelRow = 0;
    while (labelRow < MAX_LABEL_ROWS - 1 && (ends.get(labelRow) ?? 0) > start) {
      labelRow += 1;
    }
    ends.set(labelRow, Math.max(ends.get(labelRow) ?? 0, end));
    return { ...marker, label, labelRow };
  });
}

/** Vertical placement: the marker rows, then the grid rows. */
export interface Rows {
  /** Top edge of grid row `row`. */
  readonly y: (row: number) => number;
  /** Top edge of marker row `labelRow`. */
  readonly labelY: (labelRow: number) => number;
  readonly height: number;
  /** Marker rows above the grid; 0 without markers. */
  readonly inserted: number;
}

export function rows(graph: Graph, found: readonly Marker[]): Rows {
  const inserted = Math.max(0, ...found.map((marker) => marker.labelRow + 1));
  return {
    y: (row: number) => (row + inserted) * CELL_HEIGHT,
    labelY: (labelRow: number) => (inserted - 1 - labelRow) * CELL_HEIGHT,
    height: (graph.grid.height + inserted) * CELL_HEIGHT,
    inserted,
  };
}

/** The side across from `side`. */
function opposite(side: number): number {
  return ((side << 2) | (side >> 2)) & 15;
}

const SIDES = [UP, RIGHT, DOWN, LEFT] as const;

function sideCount(sides: number): number {
  return SIDES.filter((side) => (sides & side) !== 0).length;
}

/** Row and column steps towards `side`. */
function step(side: number): readonly [number, number] {
  switch (side) {
    case UP: {
      return [-1, 0];
    }
    case DOWN: {
      return [1, 0];
    }
    case LEFT: {
      return [0, -1];
    }
    default: {
      return [0, 1];
    }
  }
}

interface Point {
  readonly x: number;
  readonly y: number;
}

type Placement = Pick<Rows, "y">;

function centre(cell: Cell, placement: Placement): Point {
  return {
    x: cellX(cell.col) + CELL_WIDTH / 2,
    y: placement.y(cell.row) + CELL_HEIGHT / 2,
  };
}

/** The point `distance` from the cell's centre towards `side`; its edge by default. */
function toward(cell: Cell, placement: Placement, side: number, distance?: number): Point {
  const c = centre(cell, placement);
  const [down, right] = step(side);
  const d = distance ?? (down === 0 ? CELL_WIDTH : CELL_HEIGHT) / 2;
  return { x: c.x + right * d, y: c.y + down * d };
}

function point(p: Point): string {
  return `${n(p.x)} ${n(p.y)}`;
}

function isRun(cell: Cell): boolean {
  return sideCount(cell.sides) === 2;
}

/** A cell that isn't part of a run: a spoke from its centre per side. */
function spokes(cell: Cell, placement: Placement): string {
  return SIDES.filter((side) => (cell.sides & side) !== 0)
    .map((side) => `M${point(centre(cell, placement))}L${point(toward(cell, placement, side))}`)
    .join("");
}

/**
Path data per lane, from the grid's cells. Runs of cells with two sides become one path, with
rounded corners where they turn (so dashes run on); other cells draw a spoke per side.
*/
export function lanePaths(cells: readonly Cell[], placement: Placement): Map<number, string> {
  const at = new Map<string, Cell>();
  for (const cell of cells) {
    at.set(`${String(cell.row)},${String(cell.col)}`, cell);
  }
  /** The next cell of the run across `side`, if the run goes on there. */
  const next = (cell: Cell, side: number): Cell | undefined => {
    const [down, right] = step(side);
    const neighbour = at.get(`${String(cell.row + down)},${String(cell.col + right)}`);
    return neighbour?.lane === cell.lane &&
      (neighbour.sides & opposite(side)) !== 0 &&
      isRun(cell) &&
      isRun(neighbour)
      ? neighbour
      : undefined;
  };
  const parts = new Map<number, string>();
  const add = (lane: number, part: string): void => {
    parts.set(lane, (parts.get(lane) ?? "") + part);
  };
  const seen = new Set<Cell>();
  /** One path along a run, from `start`'s `entry` side to where the run ends. */
  const walk = (start: Cell, entry: number): void => {
    let cell: Cell | undefined = start;
    let from = entry;
    let d = `M${point(toward(start, placement, entry))}`;
    while (cell !== undefined) {
      seen.add(cell);
      const exit = cell.sides & ~from;
      if (exit !== opposite(from)) {
        const r = CELL_WIDTH / 2;
        d += `L${point(toward(cell, placement, from, r))}`;
        d += `Q${point(centre(cell, placement))} ${point(toward(cell, placement, exit, r))}`;
      }
      const following = next(cell, exit);
      if (following === undefined || seen.has(following)) {
        d += `L${point(toward(cell, placement, exit))}`;
      } else {
        from = opposite(exit);
      }
      cell = following !== undefined && seen.has(following) ? undefined : following;
    }
    add(start.lane, d);
  };
  for (const cell of cells) {
    if (!isRun(cell)) {
      add(cell.lane, spokes(cell, placement));
    }
  }
  // Runs start at a side that doesn't go on; what is left after that are closed loops.
  for (const cell of cells) {
    const open = SIDES.find((side) => (cell.sides & side) !== 0 && next(cell, side) === undefined);
    if (open !== undefined && isRun(cell) && !seen.has(cell)) {
      walk(cell, open);
    }
  }
  for (const cell of cells) {
    if (isRun(cell) && !seen.has(cell)) {
      walk(cell, SIDES.find((side) => (cell.sides & side) !== 0) ?? UP);
    }
  }
  return parts;
}
