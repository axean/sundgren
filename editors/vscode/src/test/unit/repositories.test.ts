import * as assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { submodules } from "../../git.js";
import { repositoryChoices } from "../../repositories.js";
import { git } from "../fixture.js";

describe("repositoryChoices", () => {
  const found = [
    { top: "/w/meta", submodules: ["/w/meta/local/tool", "/w/meta/vendor/lib"] },
    { top: "/w/other", submodules: [] },
  ];

  it("lists each open folder's repository, then its submodules under its name", () => {
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
    const twice = [...found, { top: "/w/meta/local/tool", submodules: [] }];
    const choices = repositoryChoices(twice, "/w/meta", "/w/meta");
    assert.deepEqual(
      choices.map((choice) => choice.path),
      ["/w/meta", "/w/meta/local/tool", "/w/meta/vendor/lib", "/w/other"],
    );
    assert.equal(choices[1]?.label, "meta/local/tool");
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
