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
  });

  it("rejects anything else", () => {
    for (const value of [undefined, "ready", {}, { type: "rendered", oid: "abc" }, { type: "x" }]) {
      assert.equal(parseFromPage(value), undefined);
    }
  });
});
