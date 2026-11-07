import * as assert from "node:assert/strict";

import {
  clock,
  dayKey,
  dayLabel,
  fileKinds,
  fileKindsText,
  formatTime,
} from "../../webview/format.js";

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

function stat(filesAdded: number, filesModified: number, filesDeleted: number) {
  return { filesAdded, filesModified, filesDeleted };
}

describe("file kinds", () => {
  it("keeps the kinds that aren't zero, added, modified, deleted", () => {
    assert.deepEqual(fileKinds(stat(1, 1, 1)), [
      ["added", 1],
      ["modified", 1],
      ["deleted", 1],
    ]);
    assert.deepEqual(fileKinds(stat(0, 1, 0)), [["modified", 1]]);
    assert.deepEqual(fileKinds(stat(0, 0, 4)), [["deleted", 4]]);
    assert.deepEqual(fileKinds(stat(2, 3, 0)), [
      ["added", 2],
      ["modified", 3],
    ]);
    assert.deepEqual(fileKinds(stat(0, 0, 0)), []);
  });

  it("says them in words", () => {
    const all = fileKinds(stat(2, 3, 1));
    assert.equal(fileKindsText(all), "2 added, 3 modified, 1 deleted");
    const modified = fileKinds(stat(0, 5, 0));
    assert.equal(fileKindsText(modified), "5 modified");
    assert.equal(fileKindsText([]), "");
  });
});
