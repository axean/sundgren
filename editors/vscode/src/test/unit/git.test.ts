import * as assert from "node:assert/strict";

import { chooseBase, parseBranchStates, parseBranches, parseChanges } from "../../git.js";

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

describe("chooseBase", () => {
  it("prefers what origin/HEAD points at, then main or master", () => {
    assert.equal(chooseBase("origin/trunk", ["main"]), "origin/trunk");
    assert.equal(chooseBase(undefined, ["dev", "master", "main"]), "main");
    assert.equal(chooseBase("", ["dev", "origin/master"]), "origin/master");
    assert.equal(chooseBase(undefined, ["dev", "feature"]), undefined);
  });
});

describe("parseBranchStates", () => {
  it("joins local branches with their upstreams, and keeps remote-only ones", () => {
    const output = [
      "refs/heads/main\0origin/main\0\0/repo",
      "refs/heads/feature\0origin/feature\0ahead 2, behind 1\0",
      "refs/heads/old\0origin/old\0gone\0",
      "refs/heads/mine\0\0\0/repo-mine",
      "refs/remotes/origin/HEAD\0\0\0",
      "refs/remotes/origin/main\0\0\0",
      "refs/remotes/origin/feature\0\0\0",
      "refs/remotes/origin/theirs\0\0\0",
      "",
    ].join("\n");
    assert.deepEqual(parseBranchStates(output), [
      {
        name: "main",
        isRemote: false,
        upstream: "origin/main",
        ahead: 0,
        behind: 0,
        worktree: "/repo",
      },
      { name: "feature", isRemote: false, upstream: "origin/feature", ahead: 2, behind: 1 },
      { name: "old", isRemote: false, upstream: "origin/old", isGone: true },
      { name: "mine", isRemote: false, worktree: "/repo-mine" },
      { name: "origin/theirs", isRemote: true },
    ]);
    assert.deepEqual(parseBranchStates(""), []);
  });
});

describe("parseChanges", () => {
  it("counts changed and untracked files", () => {
    assert.deepEqual(parseChanges(" M a.py\nM  b.py\n?? c.py\n"), { changed: 2, untracked: 1 });
    assert.deepEqual(parseChanges(""), { changed: 0, untracked: 0 });
  });
});
