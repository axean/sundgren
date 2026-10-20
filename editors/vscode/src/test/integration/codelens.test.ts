// The "sg: pluck this test" code lenses on the fixture's test files, at the def or fn lines.
import * as assert from "node:assert/strict";
import path from "node:path";
import * as vscode from "vscode";

const TITLE = "sg: pluck this test";

async function lensLines(file: string): Promise<number[]> {
  const folder = vscode.workspace.workspaceFolders?.[0];
  assert.ok(folder, "no workspace folder");
  const uri = vscode.Uri.file(path.join(folder.uri.fsPath, file));
  await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri));
  // The provider can register after the document opens; ask until lenses show up.
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const lenses = await vscode.commands.executeCommand<vscode.CodeLens[] | undefined>(
      "vscode.executeCodeLensProvider",
      uri,
    );
    const lines = (lenses ?? [])
      .filter((lens) => lens.command?.title === TITLE)
      .map((lens) => lens.range.start.line);
    if (lines.length > 0) {
      return lines.toSorted((a, b) => a - b);
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return [];
}

describe("code lens", () => {
  it("is above each Python test", async () => {
    assert.deepEqual(await lensLines("test_x.py"), [7, 12, 17]);
  });

  it("is above each Rust test that isn't ignored", async () => {
    assert.deepEqual(await lensLines("lib.rs"), [7]);
  });
});
