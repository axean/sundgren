import * as assert from "node:assert/strict";

import {
  chooseBase,
  commitLinks,
  parseBranchStates,
  parseBranches,
  parseChanges,
} from "../../git.js";

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

/** `remote`'s commit link host and the link to commit `abc`. */
function url(remote: string): string | undefined {
  const links = commitLinks(remote);
  return links === undefined ? undefined : `${links.host} ${links.prefix}abc`;
}

describe("commitLinks", () => {
  it("opens commits on GitHub", () => {
    const github = "github.com https://github.com/owner/repo/commit/abc";
    assert.equal(url("git@github.com:owner/repo.git"), github);
    assert.equal(url("https://github.com/owner/repo.git\n"), github);
    assert.equal(url("https://user:token@github.com/owner/repo"), github);
    assert.equal(url("ssh://git@github.com/owner/repo.git"), github);
    assert.equal(url("ssh://git@ssh.github.com:443/owner/repo.git"), github);
  });

  it("opens commits GitLab style on any other host", () => {
    const gitlab = "git.example.org https://git.example.org/group/sub/repo/-/commit/abc";
    assert.equal(url("git@git.example.org:group/sub/repo.git"), gitlab);
    assert.equal(url("ssh://git@git.example.org:2222/group/sub/repo.git"), gitlab);
    assert.equal(url("git+ssh://git.example.org/group/sub/repo"), gitlab);
    assert.equal(url("https://git.example.org/group/sub/repo.git/"), gitlab);
    assert.equal(
      url("https://git.example.org:8443/group/repo"),
      "git.example.org:8443 https://git.example.org:8443/group/repo/-/commit/abc",
    );
  });

  it("has none for local paths and unknown schemes", () => {
    for (const remote of [
      "/srv/git/repo.git",
      "../repo",
      "file:///srv/repo.git",
      "rsync://host/repo",
      "",
    ]) {
      assert.equal(commitLinks(remote), undefined, remote);
    }
  });
});
