import * as assert from "node:assert/strict";

import { clock, dayKey, dayLabel, formatTime } from "../../webview/format.js";

describe("formatTime", () => {
  it("formats local time to the minute", () => {
    const seconds = new Date(2026, 0, 2, 3, 4, 59).getTime() / 1000;
    assert.equal(formatTime(seconds), "2026-01-02 03:04");
  });
});

describe("time axis", () => {
  const seconds = new Date(2025, 9, 30, 10, 37).getTime() / 1000;

  it("labels the day, with the year only if it isn't the given one", () => {
    assert.equal(dayLabel(seconds, 2025), "Oct 30");
    assert.equal(dayLabel(seconds, 2026), "Oct 30 '25");
  });

  it("gives the time and the day", () => {
    assert.equal(clock(seconds), "10:37");
    assert.equal(dayKey(seconds), "2025-10-30");
  });
});
