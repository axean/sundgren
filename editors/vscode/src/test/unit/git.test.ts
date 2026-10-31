import * as assert from "node:assert/strict";

import { parseBranches } from "../../git.js";

describe("parseBranches", () => {
  it("lists the current branch, then local, then remote branches", () => {
    const output = [
      "refs/heads/feature\0 ",
      "refs/heads/main\0*",
      "refs/remotes/origin/HEAD\0 ",
      "refs/remotes/origin/main\0 ",
      "refs/heads/topic/x\0 ",
      "",
    ].join("\n");
    assert.deepEqual(parseBranches(output), [
      { name: "main", isRemote: false, isCurrent: true },
      { name: "feature", isRemote: false, isCurrent: false },
      { name: "topic/x", isRemote: false, isCurrent: false },
      { name: "origin/main", isRemote: true, isCurrent: false },
    ]);
  });

  it("ignores other refs and empty output", () => {
    assert.deepEqual(parseBranches(""), []);
    assert.deepEqual(parseBranches("refs/tags/v1\0 \nrefs/stash\0 "), []);
  });
});
