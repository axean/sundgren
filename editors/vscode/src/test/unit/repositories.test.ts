import * as assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, realpathSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { submodules } from "../../git.js";
import { repositoriesBelow, repositoryChoices } from "../../repositories.js";
import { git } from "../fixture.js";

describe("repositoryChoices", () => {
  const found = [
    { root: "/w/meta", repositories: ["/w/meta", "/w/meta/local/tool", "/w/meta/vendor/lib"] },
    { root: "/w/other", repositories: ["/w/other"] },
  ];

  it("lists each open folder's repositories under its name", () => {
    assert.deepEqual(
      repositoryChoices(found, undefined, undefined).map((choice) => choice.label),
      ["meta", "meta/local/tool", "meta/vendor/lib", "other"],
    );
  });

  it("puts the current one first, then the active file's, and adds them if not found", () => {
    const choices = repositoryChoices(found, "/elsewhere/repo", "/w/meta/vendor/lib");
    assert.deepEqual(
      choices.map((choice) => [choice.label, choice.isCurrent, choice.isActive]),
      [
        ["meta/vendor/lib", true, false],
        ["repo", false, true],
        ["meta", false, false],
        ["meta/local/tool", false, false],
        ["other", false, false],
      ],
    );
  });

  it("lists a repository once", () => {
    const twice = [...found, { root: "/w/meta/local/tool", repositories: ["/w/meta/local/tool"] }];
    const choices = repositoryChoices(twice, "/w/meta", "/w/meta");
    assert.deepEqual(
      choices.map((choice) => choice.path),
      ["/w/meta", "/w/meta/local/tool", "/w/meta/vendor/lib", "/w/other"],
    );
    assert.equal(choices[1]?.label, "meta/local/tool");
  });

  it("labels repositories under an open folder that is not one", () => {
    const below = [{ root: "/w/code", repositories: ["/w/code/a", "/w/code/b/c"] }];
    assert.deepEqual(
      repositoryChoices(below, undefined, undefined).map((choice) => choice.label),
      ["code/a", "code/b/c"],
    );
  });
});

describe("repositoriesBelow", () => {
  const temporary = mkdtempSync(path.join(tmpdir(), "sundgren-below-"));
  // As readdir gives it: macOS's temporary folder is behind a symbolic link.
  const made = realpathSync(temporary);
  const folder = (...parts: string[]): string => {
    const directory = path.join(made, ...parts);
    mkdirSync(directory, { recursive: true });
    return directory;
  };
  folder(".git");
  const a = path.dirname(folder("a", ".git"));
  const inner = path.dirname(folder("a", "inner", ".git"));
  const deep = path.dirname(folder("b", "c", "d", ".git"));
  folder("e", "f", "g", "h", ".git");
  writeFileSync(path.join(folder("worktree"), ".git"), "gitdir: /elsewhere/.git/worktrees/w\n");
  folder("node_modules", "x", ".git");
  folder("target", "x", ".git");
  folder(".hidden", "x", ".git");

  it("finds folders with a .git folder, down to the depth, not the folder itself", async () => {
    assert.deepEqual(await repositoriesBelow(made, 3), [a, inner, deep]);
    assert.deepEqual(await repositoriesBelow(made, 1), [a]);
    assert.deepEqual(await repositoriesBelow(made, 0), []);
  });

  it("finds nothing in a folder that is not there", async () => {
    assert.deepEqual(await repositoriesBelow(path.join(made, "missing"), 3), []);
  });
});

function repository(parent: string, name: string): string {
  const directory = path.join(parent, name);
  mkdirSync(directory);
  git(directory, "init", "-q", "-b", "main");
  git(directory, "commit", "-q", "--allow-empty", "-m", name);
  return directory;
}

function addSubmodule(repository_: string, url: string, at: string): void {
  git(repository_, "-c", "protocol.file.allow=always", "submodule", "add", "-q", url, at);
  git(repository_, "commit", "-q", "-m", `add ${at}`);
}

describe("submodules", () => {
  it("lists checked-out submodules, nested ones too", async () => {
    const made = mkdtempSync(path.join(tmpdir(), "sundgren-submodules-"));
    // As git prints it: macOS's temporary folder is behind a symbolic link.
    const parent = realpathSync(made);
    const leaf = repository(parent, "leaf");
    const middle = repository(parent, "middle");
    addSubmodule(middle, leaf, "leaf");
    const top = repository(parent, "top");
    addSubmodule(top, middle, "deps/middle");
    git(
      top,
      "-c",
      "protocol.file.allow=always",
      "submodule",
      "update",
      "-q",
      "--init",
      "--recursive",
    );
    assert.deepEqual(await submodules(top), [
      path.join(top, "deps", "middle"),
      path.join(top, "deps", "middle", "leaf"),
    ]);
    assert.deepEqual(await submodules(leaf), []);
  });
});
