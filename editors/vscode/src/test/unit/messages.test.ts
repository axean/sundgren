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
    const left = { oid: "abc1234", path: "a.py" };
    const right = { oid: "def5678", path: "b/c.py" };
    assert.deepEqual(parseFromPage({ type: "diff", left, right, title: "t", line: 3 }), {
      type: "diff",
      left,
      right,
      title: "t",
      line: 3,
    });
    assert.deepEqual(parseFromPage({ type: "diff", left, right, title: "t" }), {
      type: "diff",
      left,
      right,
      title: "t",
    });
    const changes = [{ path: "a.py", from: "abc1234", to: "def5678" }];
    assert.deepEqual(parseFromPage({ type: "changes", title: "t", changes }), {
      type: "changes",
      title: "t",
      changes,
    });
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
      {
        type: "diff",
        left: { oid: "--output=x", path: "a" },
        right: { oid: "abc1", path: "a" },
        title: "t",
      },
      {
        type: "diff",
        left: { oid: "abc1", path: "" },
        right: { oid: "abc1", path: "a" },
        title: "t",
      },
      {
        type: "diff",
        left: { oid: "abc1", path: "a" },
        right: { oid: "abc1", path: "a" },
        title: "t",
        line: 0,
      },
      { type: "diff", left: { oid: "abc1", path: "a" }, right: { oid: "abc1", path: "a" } },
      { type: "changes", title: "t", changes: [{ path: "a", from: "HEAD", to: "abc1" }] },
      { type: "changes", title: "t" },
    ]) {
      assert.equal(parseFromPage(value), undefined);
    }
  });
});
