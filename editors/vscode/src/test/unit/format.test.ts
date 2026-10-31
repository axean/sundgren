import * as assert from "node:assert/strict";

import { formatTime } from "../../webview/format.js";

describe("formatTime", () => {
  it("formats local time to the minute", () => {
    const seconds = new Date(2026, 0, 2, 3, 4, 59).getTime() / 1000;
    assert.equal(formatTime(seconds), "2026-01-02 03:04");
  });
});
