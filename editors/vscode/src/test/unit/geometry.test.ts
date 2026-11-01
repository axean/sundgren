import * as assert from "node:assert/strict";

import { DOWN, LEFT, RIGHT, UP, type Cell } from "../../graph.js";
import {
  CELL_HEIGHT,
  CELL_WIDTH,
  cellX,
  columnsFor,
  GUTTER,
  lanePaths,
  markers,
  rows,
} from "../../webview/geometry.js";
import { GRAPH, nodeColumn } from "../graph-fixture.js";

const flat = { y: (row: number): number => row * CELL_HEIGHT };

function cell(row: number, col: number, sides: number, lane = 0): Cell {
  return { row, col, sides, lane };
}

describe("geometry", () => {
  it("fits commit columns into a width: 3 cells each, 3 more for the connectors", () => {
    assert.equal(columnsFor(0), 1);
    assert.equal(columnsFor(6 * CELL_WIDTH), 1);
    assert.equal(columnsFor((3 * 8 + 3) * CELL_WIDTH), 8);
    assert.equal(columnsFor((3 * 8 + 5) * CELL_WIDTH), 8);
    assert.equal(cellX(0), GUTTER);
  });

  it("draws a straight run as one segment", () => {
    const paths = lanePaths(
      [cell(0, 0, RIGHT), cell(0, 1, LEFT | RIGHT), cell(0, 2, LEFT | RIGHT), cell(0, 3, LEFT)],
      flat,
    );
    const y = CELL_HEIGHT / 2;
    assert.equal(
      paths.get(0),
      // The ends are spokes from their centres; the middle is one run, edge to edge.
      `M${String(GUTTER + 4.5)} ${String(y)}L${String(GUTTER + 9)} ${String(y)}` +
        `M${String(GUTTER + 31.5)} ${String(y)}L${String(GUTTER + 27)} ${String(y)}` +
        `M${String(GUTTER + 9)} ${String(y)}L${String(GUTTER + 27)} ${String(y)}`,
    );
  });

  it("rounds a corner", () => {
    const paths = lanePaths([cell(0, 0, DOWN), cell(1, 0, UP | RIGHT), cell(1, 1, LEFT)], flat);
    const d = paths.get(0) ?? "";
    // Down into the corner cell, a curve through its centre, out to its right edge.
    assert.match(d, /M40\.5 22L40\.5 28\.5Q40\.5 33 45 33L45 33/u);
  });

  it("keeps each lane's lines apart, and a via breaks the lane under the vertical", () => {
    const paths = lanePaths(
      [
        cell(0, 1, DOWN, 1),
        cell(1, 0, LEFT, 0),
        cell(1, 1, UP | DOWN, 1),
        cell(1, 2, RIGHT, 0),
        cell(2, 1, UP, 1),
      ],
      flat,
    );
    assert.deepEqual(new Set(paths.keys()), new Set([0, 1]));
    // Lane 0 never reaches column 1's centre.
    assert.doesNotMatch(paths.get(0) ?? "", /49\.5 33/u);
  });

  it("draws every cell of the fixture", () => {
    const paths = lanePaths(GRAPH.grid.cells, flat);
    assert.deepEqual(new Set(paths.keys()), new Set(GRAPH.lanes.map((_, index) => index)));
  });

  it("puts ref markers above the grid, a row higher when they'd overlap", () => {
    const found = markers(GRAPH);
    assert.deepEqual(
      found.map((marker) => [marker.col, marker.label, marker.labelRow]),
      [
        [nodeColumn(1), "v1.0", 0],
        [nodeColumn(3), "origin/main", 0],
      ],
    );
    const placement = rows(GRAPH, found);
    assert.equal(placement.inserted, 1);
    assert.equal(placement.y(0), CELL_HEIGHT);
    assert.equal(placement.labelY(0), 0);
    assert.equal(placement.height, (GRAPH.grid.height + 1) * CELL_HEIGHT);

    const crowded = structuredClone(GRAPH);
    const commits = crowded.commits.map((commit) =>
      commit.column === nodeColumn(2) && commit.lanes.includes(0)
        ? { ...commit, refs: ["release"] }
        : commit,
    );
    const stacked = markers({ ...crowded, commits });
    assert.deepEqual(
      stacked.map((marker) => marker.labelRow),
      [0, 1, 0],
    );
    assert.equal(rows(GRAPH, stacked).labelY(1), 0);
  });
});
