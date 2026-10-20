// The installed platform .vsix plucks a test with its bundled sundgren (see run.ts).
import * as assert from "node:assert/strict";
import { mkdirSync } from "node:fs";
import path from "node:path";

import * as vscode from "vscode";

import {
  expectKind,
  fixtures,
  makeFixture,
  openTest,
  pidLines,
  pluck,
  TEST_FILE,
} from "../fixture.js";

const EXPECTED = process.env["SUNDGREN_EXPECTED_BINARY"] ?? "";

describe("platform .vsix", () => {
  it("plucks a test with the bundled sundgren", async () => {
    assert.equal(vscode.workspace.getConfiguration("sundgren").get("path"), "");
    mkdirSync(fixtures, { recursive: true });
    const fixture = makeFixture();
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const { result } = expectKind(await pluck(editor.document), "done");
    assert.equal(result.linesLost, 3);
    const parents = new Set(pidLines(fixture).map((line) => line.split(" ", 2)[1]));
    assert.deepEqual([...parents], [EXPECTED]);
  });
});
