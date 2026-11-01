// The installed platform .vsix shows a history with its bundled sundgren (see run.ts).
import * as assert from "node:assert/strict";

import * as vscode from "vscode";

import type { ShowResult } from "../../extension.js";
import { makeRepository } from "../fixture.js";

const EXPECTED = process.env["SUNDGREN_EXPECTED_BINARY"] ?? "";

describe("platform .vsix", () => {
  it("shows a history with the bundled sundgren", async () => {
    assert.equal(vscode.workspace.getConfiguration("sundgren").get("path"), "");
    const result = await vscode.commands.executeCommand<ShowResult>("sundgren.showHistory", {
      repository: makeRepository(),
      base: "main",
    });
    assert.ok(result.kind === "shown", JSON.stringify(result));
    assert.equal(result.binary, EXPECTED);
    assert.equal(result.rendered, result.lanes);
  });
});
