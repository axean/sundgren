// Where things go in the graph's SVG. Row r's node is at (x(column), y(r)). No DOM.

import type { Line } from "../graph.js";

export const ROW_HEIGHT = 24;
export const COLUMN_WIDTH = 16;
export const NODE_RADIUS = 4;

export function x(column: number): number {
  return column * COLUMN_WIDTH + COLUMN_WIDTH / 2;
}

export function y(row: number): number {
  return row * ROW_HEIGHT + ROW_HEIGHT / 2;
}

/** Width of the graph area for `columns` columns. */
export function graphWidth(columns: number): number {
  return Math.max(columns, 1) * COLUMN_WIDTH;
}

/** SVG path data from `from` at y1 to `to` at y2: straight down, or an S-curve between columns. */
function path(from: number, to: number, y1: number, y2: number): string {
  const x1 = x(from);
  const x2 = x(to);
  if (from === to) {
    return `M${String(x1)} ${String(y1)}V${String(y2)}`;
  }
  const middle = (y1 + y2) / 2;
  return `M${String(x1)} ${String(y1)}C${String(x1)} ${String(middle)} ${String(x2)} ${String(middle)} ${String(x2)} ${String(y2)}`;
}

/** The path of a line into row `row` from the row above. */
export function linePath(line: Line, row: number): string {
  return path(line.from, line.to, y(row - 1), y(row));
}

/** The path of a tail line: from the last row down past the bottom edge. */
export function tailPath(line: Line, lastRow: number): string {
  return path(line.from, line.to, y(lastRow), y(lastRow) + ROW_HEIGHT);
}
