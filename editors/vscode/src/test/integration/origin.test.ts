// `sundgren: Show Origin` end to end, with the sundgren built at the repo root.
import * as assert from "node:assert/strict";
import path from "node:path";

import * as vscode from "vscode";

import type {
  InspectOriginArguments,
  InspectResult,
  OriginArguments,
  OriginResult,
} from "../../extension.js";
import { makeOriginRepositories } from "../fixture.js";

const ROOT = process.env["SUNDGREN_TEST_ROOT"] ?? "";
const BINARY = path.join(ROOT, "target", "debug", "sundgren");

function originTabs(): string[] {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .filter(
      (tab) =>
        tab.input instanceof vscode.TabInputWebview &&
        tab.input.viewType.endsWith("sundgren.origin"),
    )
    .map((tab) => tab.label);
}

describe("show origin", () => {
  before(async () => {
    await vscode.workspace
      .getConfiguration("sundgren")
      .update("path", BINARY, vscode.ConfigurationTarget.Global);
  });

  after(async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  it("finds where a file comes from at each commit, and draws a row for each", async () => {
    const { file, other } = makeOriginRepositories();
    const result = await vscode.commands.executeCommand<OriginResult>("sundgren.showOrigin", {
      file,
      other,
      refs: ["v1", "main"],
    } satisfies OriginArguments);
    assert.deepEqual(result, {
      kind: "shown",
      path: "copy.py",
      references: 2,
      found: 2,
      rendered: 2,
    });
    assert.deepEqual(originTabs(), ["Origin: copy.py"]);
  });

  it("inspects one commit's best file line by line", async () => {
    const { file, other } = makeOriginRepositories();
    const result = await vscode.commands.executeCommand<InspectResult>("sundgren.inspectOrigin", {
      file,
      other,
      ref: "v1",
    } satisfies InspectOriginArguments);
    // The header comment, the def line and eight more.
    assert.deepEqual(result, { kind: "shown", lines: 10, rendered: 10 });
  });

  it("reports a commit the other repository doesn't have", async () => {
    const { file, other } = makeOriginRepositories();
    const result = await vscode.commands.executeCommand<OriginResult>("sundgren.showOrigin", {
      file,
      other,
      refs: ["no-such-tag"],
    } satisfies OriginArguments);
    assert.deepEqual(result, { kind: "failed", message: "no-such-tag is not a commit" });
  });
});
