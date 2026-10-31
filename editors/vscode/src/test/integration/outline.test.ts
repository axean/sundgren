// `sundgren: Show Outline` end to end, with the sundgren built at the repo root.
import * as assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import type { OutlineArguments, OutlineResult } from "../../extension.js";

const ROOT = process.env["SUNDGREN_TEST_ROOT"] ?? "";
const BINARY = path.join(ROOT, "target", "debug", "sundgren");

async function show(arguments_?: OutlineArguments): Promise<OutlineResult> {
  return vscode.commands.executeCommand<OutlineResult>("sundgren.showOutline", arguments_);
}

function outlineTabs(): string[] {
  return vscode.window.tabGroups.all
    .flatMap((group) => group.tabs)
    .filter(
      (tab) =>
        tab.input instanceof vscode.TabInputWebview &&
        tab.input.viewType.endsWith("sundgren.outline"),
    )
    .map((tab) => tab.label);
}

describe("show outline", () => {
  const directory = mkdtempSync(path.join(tmpdir(), "sundgren-outline-"));
  const python = path.join(directory, "shapes.py");
  writeFileSync(python, "class Shape:\n    def area(self):\n        return 0  # none\n");
  const text = path.join(directory, "notes.txt");
  writeFileSync(text, "not code\n");

  before(async () => {
    await vscode.workspace
      .getConfiguration("sundgren")
      .update("path", BINARY, vscode.ConfigurationTarget.Global);
  });

  after(async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  it("shows the items of the active editor's file", async () => {
    await vscode.window.showTextDocument(vscode.Uri.file(python));
    const result = await show();
    assert.deepEqual(result, {
      kind: "shown",
      path: python,
      language: "python",
      items: 2,
      rendered: 2,
    });
    assert.deepEqual(outlineTabs(), ["Outline: shapes.py"]);
  });

  it("shows a file it doesn't read, with no items", async () => {
    const result = await show({ file: text });
    assert.deepEqual(result, { kind: "shown", path: text, items: 0, rendered: 0 });
    assert.deepEqual(outlineTabs(), ["Outline: notes.txt"]);
  });

  it("fails without a file", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const result = await show();
    assert.equal(result.kind, "failed");
  });
});
