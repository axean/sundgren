import * as assert from "node:assert/strict";

import { progressMessage } from "../../sundgren.js";

describe("progressMessage", () => {
  it("turns sundgren's progress lines into notification text", () => {
    assert.equal(
      progressMessage("sundgren: base: running tests…", "test_a"),
      "running the tests on the last commit…",
    );
    assert.equal(
      progressMessage("sundgren: patched: run 2/3: running tests…", "TestA::test_a"),
      "running the tests without TestA::test_a (run 2/3)…",
    );
    assert.equal(progressMessage("sundgren: comparing coverage", "test_a"), "comparing coverage…");
  });

  it("ignores other lines", () => {
    assert.equal(progressMessage("sundgren: base: done in 1.2s", "test_a"), undefined);
    assert.equal(progressMessage("sundgren: cancelled", "test_a"), undefined);
    assert.equal(progressMessage("", "test_a"), undefined);
  });
});
