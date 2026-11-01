// Where things go in the History map's SVG. One grid cell is one character cell of the Unicode
// rendering; lines run from cell centres to the connected sides. Rows with a commit, a connector
// or a lane label are table rows; the others are short spacers. No DOM.

import { DOWN, LEFT, RIGHT, UP, type Cell, type Graph } from "../graph.js";

export const CELL_WIDTH = 8;
export const ROW_HEIGHT = 24;
export const SPACER_HEIGHT = 8;
export const NODE_RADIUS = 4;
/** Room left of the grid. */
export const GUTTER = 6;
/** Half the break in a lane where a line passes over it. */
const VIA_GAP = 3.5;

/** Left edge of grid column `col`. */
export function cellX(col: number): number {
  return GUTTER + col * CELL_WIDTH;
}

/** Numbers in path data: no float noise. */
function n(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** Vertical placement of the grid rows. */
export interface Placement {
  /** Top edge of grid row `row`. */
  readonly y: (row: number) => number;
  /** Height of grid row `row`; 0 when hidden. */
  readonly h: (row: number) => number;
}

/** Placement of every row, with folded runs hidden. */
export interface Layout extends Placement {
  readonly height: number;
  /** Visible rows that are table rows, top to bottom. */
  readonly rows: readonly number[];
  /** The lanes labelled on each table row, left to right. */
  readonly labels: ReadonlyMap<number, readonly number[]>;
}

/** Rows hidden by the folds in `folded` (indices into `graph.folds`). */
export function hiddenRows(graph: Graph, folded: ReadonlySet<number>): Set<number> {
  const hidden = new Set<number>();
  for (const [index, fold] of graph.folds.entries()) {
    if (folded.has(index)) {
      for (let row = fold.rows[0] + 1; row <= fold.rows[1]; row++) {
        hidden.add(row);
      }
    }
  }
  return hidden;
}

/** The nearest row at or above `row` that isn't hidden. */
function shownAt(row: number, hidden: ReadonlySet<number>): number {
  let shown = row;
  while (shown > 0 && hidden.has(shown)) {
    shown -= 1;
  }
  return shown;
}

export function layout(graph: Graph, folded: ReadonlySet<number>): Layout {
  const hidden = hiddenRows(graph, folded);
  const labels = new Map<number, number[]>();
  const byColumn = graph.lanes
    .map((lane, index) => ({ lane, index }))
    .toSorted((a, b) => a.lane.column - b.lane.column);
  for (const { lane, index } of byColumn) {
    const row = shownAt(lane.label_row, hidden);
    labels.set(row, [...(labels.get(row) ?? []), index]);
  }
  const tall = new Set(labels.keys());
  for (const cell of graph.grid.cells) {
    if (cell.node !== undefined && !hidden.has(cell.row)) {
      tall.add(cell.row);
    }
  }
  const tops: number[] = [];
  const heights: number[] = [];
  let top = 0;
  for (let row = 0; row < graph.grid.height; row++) {
    let height = tall.has(row) ? ROW_HEIGHT * Math.max(1, labels.get(row)?.length ?? 0) : 0;
    if (!tall.has(row) && !hidden.has(row)) {
      height = SPACER_HEIGHT;
    }
    tops.push(top);
    heights.push(height);
    top += height;
  }
  return {
    y: (row) => tops.at(row) ?? top,
    h: (row) => heights.at(row) ?? 0,
    height: top,
    rows: [...tall].toSorted((a, b) => a - b),
    labels,
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

/** The centre of the cell at `row`, `col`. */
export function centre(row: number, col: number, placement: Placement): Point {
  return { x: cellX(col) + CELL_WIDTH / 2, y: placement.y(row) + placement.h(row) / 2 };
}

/** The point `distance` from the cell's centre towards `side`; its edge by default. */
function toward(cell: Cell, placement: Placement, side: number, distance?: number): Point {
  const c = centre(cell.row, cell.col, placement);
  const [down, right] = step(side);
  const d = distance ?? (down === 0 ? CELL_WIDTH : placement.h(cell.row)) / 2;
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
  const c = point(centre(cell.row, cell.col, placement));
  return SIDES.filter((side) => (cell.sides & side) !== 0)
    .map((side) => `M${c}L${point(toward(cell, placement, side))}`)
    .join("");
}

type CellAt = (row: number, col: number) => Cell | undefined;

function cellIndex(cells: readonly Cell[]): CellAt {
  const at = new Map<string, Cell>();
  for (const cell of cells) {
    at.set(`${String(cell.row)},${String(cell.col)}`, cell);
  }
  return (row, col) => at.get(`${String(row)},${String(col)}`);
}

function isAcross(cell: Cell | undefined): boolean {
  return cell?.node === undefined && cell?.sides === (LEFT | RIGHT);
}

/**
The end of the lane broken by the horizontal at `row`, `col`, towards `side` (UP or DOWN): a cell
that goes on away from the via but not towards it. Vias next to each other leave a blank cell
between them.
*/
function brokenEnd(at: CellAt, row: number, col: number, side: number): Cell | undefined {
  const [down] = step(side);
  let next = row + down;
  for (;;) {
    const cell = at(next, col);
    if (cell !== undefined && (cell.sides & side) !== 0 && (cell.sides & opposite(side)) === 0) {
      return cell;
    }
    const isBlank = cell === undefined || cell.sides === 0;
    if (!isBlank || !isAcross(at(next + down, col))) {
      return undefined;
    }
    next += 2 * down;
  }
}

/** A lane passing under a horizontal line: `╵─╷`. */
export interface Via {
  readonly row: number;
  readonly col: number;
  /** The lane broken around the line. */
  readonly lane: number;
}

export function vias(cells: readonly Cell[]): Via[] {
  const at = cellIndex(cells);
  const found: Via[] = [];
  for (const cell of cells) {
    if (!isAcross(cell)) {
      continue;
    }
    const above = brokenEnd(at, cell.row, cell.col, UP);
    const below = brokenEnd(at, cell.row, cell.col, DOWN);
    if (above !== undefined && above.lane === below?.lane && above.lane !== cell.lane) {
      found.push({ row: cell.row, col: cell.col, lane: above.lane });
    }
  }
  return found;
}

/** The lane under a via, from the centre above to the centre below, with a gap at the line. */
function bridge(via: Via, placement: Placement): string {
  const top = centre(via.row - 1, via.col, placement);
  const middle = centre(via.row, via.col, placement);
  const bottom = centre(via.row + 1, via.col, placement);
  const gap = Math.min(VIA_GAP, placement.h(via.row) / 2);
  return (
    `M${point(top)}L${point({ x: middle.x, y: middle.y - gap })}` +
    `M${point({ x: middle.x, y: middle.y + gap })}L${point(bottom)}`
  );
}

/**
Path data per lane, from the grid's cells. Runs of cells with two sides become one path, with
rounded corners where they turn (so dashes run on); other cells draw a spoke per side. A lane
under a via is drawn on through it, broken around the line.
*/
export function lanePaths(cells: readonly Cell[], placement: Placement): Map<number, string> {
  const at = cellIndex(cells);
  /** The next cell of the run across `side`, if the run goes on there. */
  const next = (cell: Cell, side: number): Cell | undefined => {
    const [down, right] = step(side);
    const neighbour = at(cell.row + down, cell.col + right);
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
        const r = Math.min(CELL_WIDTH, placement.h(cell.row)) / 2;
        d += `L${point(toward(cell, placement, from, r))}`;
        d += `Q${point(centre(cell.row, cell.col, placement))} ${point(toward(cell, placement, exit, r))}`;
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
    if (!isRun(cell) && cell.sides !== 0) {
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
  for (const via of vias(cells)) {
    add(via.lane, bridge(via, placement));
  }
  return parts;
}
