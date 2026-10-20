// Smoke test in a real VS Code: the extension activates and registers its commands.
import * as assert from "node:assert/strict";
import * as vscode from "vscode";

const EXTENSION_ID = "axean.sundgren";

describe("extension", () => {
  it("activates", async () => {
    const extension = vscode.extensions.getExtension(EXTENSION_ID);
    assert.ok(extension, `${EXTENSION_ID} is not installed`);
    await extension.activate();
    assert.equal(extension.isActive, true);
  });

  it("registers its commands", async () => {
    const commands = await vscode.commands.getCommands(true);
    for (const command of ["sundgren.pluckTest", "sundgren.cancel"]) {
      assert.ok(commands.includes(command), `${command} is not registered`);
    }
  });
});
