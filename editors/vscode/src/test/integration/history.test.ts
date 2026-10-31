// End to end in a real VS Code, with the sundgren built at the repo root (`cargo build`).
import * as assert from "node:assert/strict";
import { existsSync } from "node:fs";
import path from "node:path";

import * as vscode from "vscode";

import type { ShowArguments, ShowResult } from "../../extension.js";
import { makeRepository } from "../fixture.js";

const ROOT = process.env["SUNDGREN_TEST_ROOT"] ?? "";
const BINARY = path.join(ROOT, "target", "debug", "sundgren");

async function show(arguments_: ShowArguments): Promise<ShowResult> {
  const result = await vscode.commands.executeCommand<ShowResult>(
    "sundgren.showHistory",
    arguments_,
  );
  return result;
}

function historyTabs(): string[] {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .filter(
      (tab) =>
        tab.input instanceof vscode.TabInputWebview &&
        tab.input.viewType.endsWith("sundgren.history"),
    )
    .map((tab) => tab.label);
}

describe("show history", () => {
  before(async () => {
    assert.ok(ROOT !== "", "SUNDGREN_TEST_ROOT is not set; run through .vscode-test.mjs");
    assert.ok(existsSync(BINARY), `${BINARY} is missing; run \`cargo build\` at the repo root`);
    await vscode.workspace
      .getConfiguration("sundgren")
      .update("path", BINARY, vscode.ConfigurationTarget.Global);
  });

  after(async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  it("shows a branch in one panel, reused for the next", async () => {
    const repository = makeRepository();
    const main = await show({ repository, rev: "main" });
    assert.deepEqual(main, {
      kind: "shown",
      rev: "main",
      commits: 4,
      truncated: false,
      binary: BINARY,
      rendered: 4,
    });
    assert.deepEqual(historyTabs(), ["History: main"]);

    const feature = await show({ repository, rev: "feature" });
    assert.equal(feature.kind === "shown" && feature.commits, 2);
    assert.equal(feature.kind === "shown" && feature.rendered, 2);
    assert.deepEqual(historyTabs(), ["History: feature"]);
  });

  it("limits the commits to sundgren.maxCommits", async () => {
    const settings = vscode.workspace.getConfiguration("sundgren");
    await settings.update("maxCommits", 3, vscode.ConfigurationTarget.Global);
    try {
      const result = await show({ repository: makeRepository(), rev: "main" });
      assert.equal(result.kind === "shown" && result.commits, 3);
      assert.equal(result.kind === "shown" && result.rendered, 3);
      assert.equal(result.kind === "shown" && result.truncated, true);
    } finally {
      await settings.update("maxCommits", undefined, vscode.ConfigurationTarget.Global);
    }
  });

  it("says why when the revision doesn't exist", async () => {
    const result = await show({ repository: makeRepository(), rev: "nope" });
    assert.deepEqual(result, { kind: "failed", message: "nope is not a commit" });
  });

  it("takes the repository from the open folder", async () => {
    const result = await show({ rev: "HEAD" });
    assert.equal(result.kind, "shown");
  });
});
