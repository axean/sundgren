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

  it("maps the branches around a base in one panel, reused for the next", async () => {
    const repository = makeRepository();
    const main = await show({ repository, base: "main" });
    assert.ok(main.kind === "shown", JSON.stringify(main));
    assert.equal(main.base, "main");
    // Local branches first, by default.
    assert.equal(main.show, "local");
    assert.equal(main.binary, BINARY);
    assert.equal(main.rendered, main.lanes);
    assert.deepEqual(historyTabs(), ["History: main"]);

    const feature = await show({ repository, base: "feature", like: "main" });
    assert.ok(feature.kind === "shown", JSON.stringify(feature));
    assert.equal(feature.show, "like");
    assert.equal(feature.like, "main");
    assert.equal(feature.rendered, feature.lanes);
    assert.deepEqual(historyTabs(), ["History: feature"]);
  });

  it("says why when the base doesn't exist", async () => {
    const result = await show({ repository: makeRepository(), base: "nope" });
    assert.equal(result.kind, "failed");
  });

  it("takes the repository from a Source Control row", async () => {
    const repository = makeRepository();
    // What a scm/sourceControl menu passes: the row's SourceControl, with its root.
    const result = await vscode.commands.executeCommand<ShowResult>("sundgren.showHistory", {
      rootUri: vscode.Uri.file(repository),
    });
    assert.ok(result.kind === "shown", JSON.stringify(result));
    assert.equal(result.base, "main");
  });

  it("takes the repository from the open folder", async () => {
    // A base, so it doesn't ask: a CI checkout has no default branch to take.
    const result = await show({ base: "HEAD" });
    assert.equal(result.kind, "shown");
  });
});
