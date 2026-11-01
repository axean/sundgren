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
    // main is checked out, so there is no target.
    assert.equal(main.target, undefined);
    assert.equal(main.binary, BINARY);
    assert.equal(main.rendered, main.lanes);
    assert.deepEqual(historyTabs(), ["History: main"]);

    const feature = await show({ repository, base: "feature", target: "main" });
    assert.ok(feature.kind === "shown", JSON.stringify(feature));
    assert.equal(feature.target, "main");
    assert.equal(feature.rendered, feature.lanes);
    assert.deepEqual(historyTabs(), ["History: feature"]);
  });

  it("says why when the base doesn't exist", async () => {
    const result = await show({ repository: makeRepository(), base: "nope", target: false });
    assert.equal(result.kind, "failed");
  });

  it("takes the repository from the open folder", async () => {
    const result = await show({ target: false });
    assert.equal(result.kind, "shown");
  });
});
