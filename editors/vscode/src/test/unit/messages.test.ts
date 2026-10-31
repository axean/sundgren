import * as assert from "node:assert/strict";

import { parseFromPage } from "../../messages.js";

describe("parseFromPage", () => {
  it("accepts the page's messages", () => {
    assert.deepEqual(parseFromPage({ type: "ready" }), { type: "ready" });
    assert.deepEqual(parseFromPage({ type: "rendered", oid: "abc", rows: 3, extra: 1 }), {
      type: "rendered",
      oid: "abc",
      rows: 3,
    });
    assert.deepEqual(parseFromPage({ type: "reveal", line: 12 }), { type: "reveal", line: 12 });
    assert.deepEqual(parseFromPage({ type: "choose", control: "base" }), {
      type: "choose",
      control: "base",
    });
  });

  it("rejects anything else", () => {
    for (const value of [
      undefined,
      "ready",
      {},
      { type: "rendered", oid: "abc" },
      { type: "x" },
      { type: "reveal" },
      { type: "reveal", line: 0 },
      { type: "reveal", line: 1.5 },
      { type: "choose" },
      { type: "choose", control: "colour" },
    ]) {
      assert.equal(parseFromPage(value), undefined);
    }
  });
});
