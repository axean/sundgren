import * as assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import {
  chooseBase,
  commitLinks,
  commitStats,
  parseBranchStates,
  parseBranches,
  parseChanges,
  parseNumstats,
  refsFingerprint,
  parseRefNames,
  mergedInto,
} from "../../git.js";
import { git, makeRepository } from "../fixture.js";

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

function kinds(filesAdded: number, filesModified: number, filesDeleted: number) {
  return { filesAdded, filesModified, filesDeleted };
}

describe("parseNumstats", () => {
  it("counts files by kind and lines per commit; binary files as files only", () => {
    const output = [
      "\u{1E}aaaa",
      "",
      ":100644 100644 1111111 2222222 M\tsrc/a.ts",
      ":000000 100644 0000000 3333333 A\tlogo.png",
      "3\t1\tsrc/a.ts",
      "-\t-\tlogo.png",
      "\u{1E}bbbb",
      "\u{1E}cccc",
      "",
      ":100644 100644 4444444 5555555 R087\tsrc/old.ts\tsrc/new.ts",
      ":100644 000000 6666666 0000000 D\tgone.ts",
      ":100644 120000 7777777 8888888 T\tlink",
      ":100644 100644 4444444 4444444 C100\tsrc/new.ts\tsrc/copy.ts",
      "0\t12\tsrc/{old => new}.ts",
      "0\t3\tgone.ts",
      "1\t1\tlink",
      "0\t0\tsrc/{new => copy}.ts",
      "",
    ].join("\n");
    assert.deepEqual(
      [...parseNumstats(output)],
      [
        ["aaaa", { files: 2, added: 3, removed: 1, ...kinds(1, 1, 0) }],
        ["bbbb", { files: 0, added: 0, removed: 0, ...kinds(0, 0, 0) }],
        ["cccc", { files: 4, added: 1, removed: 16, ...kinds(1, 2, 1) }],
      ],
    );
  });
});

describe("commitStats", () => {
  it("reads every commit asked for, a merge against its first parent", async () => {
    const repository = mkdtempSync(path.join(tmpdir(), "sundgren-stats-"));
    const commit = (file: string, text: string, message: string): string => {
      writeFileSync(path.join(repository, file), text);
      git(repository, "add", "--", file);
      git(repository, "commit", "-q", "-m", message);
      return git(repository, "rev-parse", "HEAD").trim();
    };
    git(repository, "init", "-q", "-b", "main");
    const first = commit("a.txt", "one\ntwo\nthree\n", "a");
    const second = commit("a.txt", "one\n2\nthree\nfour\n", "b");
    git(repository, "switch", "-q", "-c", "feature", first);
    commit("f.txt", "f\n", "f");
    git(repository, "switch", "-q", "main");
    git(repository, "merge", "-q", "--no-ff", "-m", "m", "feature");
    const merge = git(repository, "rev-parse", "HEAD").trim();

    const stats = await commitStats(repository, [merge, second, first], 2);
    const one = { filesAdded: 0, filesModified: 0, filesDeleted: 0 };
    assert.deepEqual(stats.get(first), { files: 1, added: 3, removed: 0, ...one, filesAdded: 1 });
    assert.deepEqual(stats.get(second), {
      files: 1,
      added: 2,
      removed: 1,
      ...one,
      filesModified: 1,
    });
    assert.deepEqual(stats.get(merge), { files: 1, added: 1, removed: 0, ...one, filesAdded: 1 });
  });
});

describe("refsFingerprint", () => {
  it("changes when a ref moves, appears or goes, or HEAD moves", async () => {
    const repository = makeRepository();
    const before = await refsFingerprint(repository);
    assert.notEqual(before, undefined);
    assert.equal(await refsFingerprint(repository), before);
    const seen = new Set([before]);
    const changes: string[][] = [
      ["commit", "-q", "--allow-empty", "-m", "c"],
      ["branch", "topic"],
      ["switch", "-q", "topic"],
      ["switch", "-q", "--detach", "HEAD~1"],
      ["branch", "-D", "-q", "topic"],
    ];
    for (const change of changes) {
      git(repository, ...change);
      const now = await refsFingerprint(repository);
      assert.ok(!seen.has(now), change.join(" "));
      seen.add(now);
    }
  });

  it("is undefined outside a repository", async () => {
    assert.equal(await refsFingerprint(tmpdir()), undefined);
  });
});

describe("mergedInto", () => {
  it("names branches and remote branches by their short names", () => {
    const output =
      "refs/heads/main\nrefs/heads/a/b\nrefs/remotes/origin/HEAD\nrefs/remotes/origin/x\n";
    assert.deepEqual(parseRefNames(output), ["main", "a/b", "origin/x"]);
  });

  it("lists the branches whose tips are in the base", async () => {
    const repository = makeRepository();
    git(repository, "branch", "behind", "main~1");
    git(repository, "switch", "-q", "-c", "open");
    git(repository, "commit", "-q", "--allow-empty", "-m", "o");
    const merged = await mergedInto(repository, "main");
    assert.deepEqual(
      merged.toSorted((a, b) => a.localeCompare(b)),
      ["behind", "feature", "main"],
    );
  });
});
