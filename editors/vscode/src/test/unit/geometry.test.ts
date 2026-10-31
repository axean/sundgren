import * as assert from "node:assert/strict";

import { graphWidth, linePath, ROW_HEIGHT, tailPath, x, y } from "../../webview/geometry.js";

describe("geometry", () => {
  it("puts nodes in the middle of their cell", () => {
    assert.equal(x(0), 8);
    assert.equal(x(2), 40);
    assert.equal(y(0), ROW_HEIGHT / 2);
    assert.equal(y(3), 3 * ROW_HEIGHT + ROW_HEIGHT / 2);
    assert.equal(graphWidth(0), 16);
    assert.equal(graphWidth(3), 48);
  });

  it("draws a line in one column straight down", () => {
    assert.equal(linePath({ from: 1, to: 1, color: 0 }, 2), "M24 36V60");
  });

  it("draws a line between columns as an S-curve", () => {
    assert.equal(linePath({ from: 0, to: 1, color: 0 }, 1), "M8 12C8 24 24 24 24 36");
  });

  it("draws a tail one row past the last", () => {
    assert.equal(tailPath({ from: 0, to: 0, color: 0 }, 0), "M8 12V36");
  });
});
