// `sundgren: Show Containment` end to end, with the sundgren built at the repo root.
import * as assert from "node:assert/strict";
import path from "node:path";

import * as vscode from "vscode";

import type { ContainmentArguments, ContainmentResult, InspectResult } from "../../extension.js";
import { makeContainmentRepository } from "../fixture.js";

const ROOT = process.env["SUNDGREN_TEST_ROOT"] ?? "";
const BINARY = path.join(ROOT, "target", "debug", "sundgren");

async function show(arguments_: ContainmentArguments): Promise<ContainmentResult> {
  return vscode.commands.executeCommand<ContainmentResult>("sundgren.showContainment", arguments_);
}

function containmentTabs(): string[] {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .filter(
      (tab) =>
        tab.input instanceof vscode.TabInputWebview &&
        tab.input.viewType.endsWith("sundgren.containment"),
    )
    .map((tab) => tab.label);
}

describe("show containment", () => {
  before(async () => {
    await vscode.workspace
      .getConfiguration("sundgren")
      .update("path", BINARY, vscode.ConfigurationTarget.Global);
  });

  after(async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  it("compares every other branch with the target and draws them all", async () => {
    const repository = makeContainmentRepository();
    const result = await show({ repository, base: "main", target: "dev" });
    assert.deepEqual(result, {
      kind: "shown",
      base: "main",
      target: "dev",
      compared: 2,
      failed: 0,
      rendered: 2,
    });
    assert.deepEqual(containmentTabs(), ["Containment: dev"]);
  });

  it("inspects a branch line by line", async () => {
    const repository = makeContainmentRepository();
    const result = await vscode.commands.executeCommand<InspectResult>("sundgren.inspect", {
      repository,
      base: "main",
      target: "dev",
      branch: "extract",
      path: "one.py",
    });
    if (result.kind !== "shown") {
      assert.fail(`not shown: ${JSON.stringify(result)}`);
    }
    assert.ok(result.lines > 0);
    assert.equal(result.rendered, result.lines);
  });

  it("reports a branch it can't compare, and draws the rest", async () => {
    const repository = makeContainmentRepository();
    const result = await show({
      repository,
      base: "main",
      target: "dev",
      branches: ["extract", "no-such-branch"],
    });
    assert.equal(result.kind === "shown" && result.failed, 1);
    assert.equal(result.kind === "shown" && result.rendered, 1);
  });
});
