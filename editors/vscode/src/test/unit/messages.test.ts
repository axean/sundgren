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
    assert.deepEqual(parseFromPage({ type: "inspect", branch: "b" }), {
      type: "inspect",
      branch: "b",
    });
    assert.deepEqual(parseFromPage({ type: "inspect", branch: "b", path: "a.py", line: 4 }), {
      type: "inspect",
      branch: "b",
      path: "a.py",
      line: 4,
    });
    const revision = { oid: "abc1234", path: "src/a.rs" };
    assert.deepEqual(parseFromPage({ type: "open", revision, line: 3 }), {
      type: "open",
      revision,
      line: 3,
    });
    assert.deepEqual(parseFromPage({ type: "open", revision }), { type: "open", revision });
    assert.deepEqual(parseFromPage({ type: "choose", control: "comments" }), {
      type: "choose",
      control: "comments",
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
      { type: "inspect" },
      { type: "inspect", branch: "" },
      { type: "inspect", branch: "b", path: "" },
      { type: "inspect", branch: "b", line: 0 },
      { type: "open", revision: { oid: "--output=x", path: "a" } },
      { type: "open", revision: { oid: "HEAD", path: "a" } },
      { type: "open", revision: { oid: "abc1", path: "" } },
      { type: "open", revision: { oid: "abc1", path: "a" }, line: -1 },
    ]) {
      assert.equal(parseFromPage(value), undefined, JSON.stringify(value));
    }
  });
});
