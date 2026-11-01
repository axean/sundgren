import * as assert from "node:assert/strict";

import { DOWN, LEFT, RIGHT, UP, type Cell } from "../../graph.js";
import {
  CELL_WIDTH,
  cellX,
  GUTTER,
  hiddenRows,
  lanePaths,
  layout,
  ROW_HEIGHT,
  SPACER_HEIGHT,
  vias,
} from "../../webview/geometry.js";
import { GRAPH } from "../graph-fixture.js";

const H = 20;
const flat = { y: (row: number): number => row * H, h: (): number => H };

function cell(row: number, col: number, sides: number, lane = 0): Cell {
  return { row, col, sides, lane };
}

describe("geometry", () => {
  it("places columns after the gutter", () => {
    assert.equal(cellX(0), GUTTER);
    assert.equal(cellX(2), GUTTER + 2 * CELL_WIDTH);
  });

  it("draws a straight run as one segment", () => {
    const paths = lanePaths(
      [cell(0, 0, DOWN), cell(1, 0, UP | DOWN), cell(2, 0, UP | DOWN), cell(3, 0, UP)],
      flat,
    );
    const x = String(GUTTER + CELL_WIDTH / 2);
    assert.equal(
      paths.get(0),
      // The ends are spokes from their centres; the middle is one run, edge to edge.
      `M${x} 10L${x} 20M${x} 70L${x} 60M${x} 20L${x} 60`,
    );
  });

  it("rounds a corner", () => {
    const paths = lanePaths([cell(0, 1, DOWN), cell(1, 1, UP | LEFT), cell(1, 0, RIGHT)], flat);
    // Down into the corner cell, a curve through its centre, out to its left edge.
    assert.match(paths.get(0) ?? "", /M18 20L18 26Q18 30 14 30L14 30/u);
  });

  it("finds vias, next to each other too, and draws the lane on under them", () => {
    const cells = [
      cell(0, 1, DOWN, 1),
      cell(1, 1, UP, 1),
      cell(2, 0, RIGHT, 0),
      cell(2, 1, LEFT | RIGHT, 0),
      cell(2, 2, LEFT, 0),
      cell(3, 1, 0, 1),
      cell(4, 0, RIGHT, 0),
      cell(4, 1, LEFT | RIGHT, 0),
      cell(4, 2, LEFT, 0),
      cell(5, 1, DOWN, 1),
      cell(6, 1, UP, 1),
    ];
    assert.deepEqual(vias(cells), [
      { row: 2, col: 1, lane: 1 },
      { row: 4, col: 1, lane: 1 },
    ]);
    const paths = lanePaths(cells, flat);
    // Lane 1 from row 1's centre down to just above the line in row 2.
    assert.match(paths.get(1) ?? "", /M18 30L18 46\.5M18 53\.5L18 70/u);
    // A horizontal that ends on a lane is no via.
    assert.deepEqual(vias([cell(0, 0, DOWN, 1), cell(1, 0, UP | RIGHT, 1), cell(1, 1, LEFT)]), []);
  });

  it("finds the fixture's two vias", () => {
    assert.deepEqual(vias(GRAPH.grid.cells), [
      { row: 11, col: 6, lane: 3 },
      { row: 13, col: 6, lane: 3 },
    ]);
  });

  it("draws every lane of the fixture", () => {
    const paths = lanePaths(GRAPH.grid.cells, flat);
    assert.deepEqual(new Set(paths.keys()), new Set(GRAPH.lanes.map((_, index) => index)));
  });

  it("makes table rows of commits, connectors and labels, and spacers of the rest", () => {
    const placed = layout(GRAPH, new Set());
    assert.deepEqual(placed.rows, [0, 1, 2, 4, 5, 6, 7, 9, 10, 12, 14, 15, 16, 17, 18, 19]);
    assert.equal(placed.h(0), ROW_HEIGHT);
    assert.equal(placed.h(3), SPACER_HEIGHT);
    assert.equal(placed.y(4), 3 * ROW_HEIGHT + SPACER_HEIGHT);
    assert.equal(placed.height, 16 * ROW_HEIGHT + 4 * SPACER_HEIGHT);
    assert.deepEqual(placed.labels.get(4), [0]);
  });

  it("hides a folded run but its first row, and stacks labels on a shared row", () => {
    assert.deepEqual([...hiddenRows(GRAPH, new Set([0]))], [17, 18]);
    const folded = layout(GRAPH, new Set([0]));
    assert.equal(folded.h(17), 0);
    assert.equal(folded.y(19), folded.y(16) + ROW_HEIGHT);
    assert.deepEqual(folded.rows.slice(-2), [16, 19]);

    const shared = {
      ...GRAPH,
      lanes: GRAPH.lanes.map((lane) => (lane.name === "fix-x" ? { ...lane, label_row: 1 } : lane)),
    };
    const stacked = layout(shared, new Set());
    assert.deepEqual(stacked.labels.get(1), [1, 3]);
    assert.equal(stacked.h(1), 2 * ROW_HEIGHT);
  });
});
