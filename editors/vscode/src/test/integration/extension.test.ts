import * as assert from "node:assert/strict";

import * as vscode from "vscode";

describe("extension", () => {
  it("activates", async () => {
    const extension = vscode.extensions.getExtension("axean.sundgren");
    assert.ok(extension);
    await extension.activate();
    assert.ok(extension.isActive);
  });

  it("registers its command", async () => {
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("sundgren.showHistory"));
  });
});
