// Integration tests: run in a real VS Code by @vscode/test-cli (`npm run test:integration`).
import { mkdirSync, rmSync } from "node:fs";
import path from "node:path";

import { defineConfig } from "@vscode/test-cli";

const root = path.resolve(import.meta.dirname, "../..");
const temporary = path.join(import.meta.dirname, ".vscode-test", "tmp");
rmSync(temporary, { recursive: true, force: true });
mkdirSync(temporary, { recursive: true });

export default defineConfig({
  // CI also runs the oldest version package.json's engines.vscode allows.
  version: process.env["VSCODE_TEST_VERSION"] ?? "stable",
  files: "out/test/integration/**/*.test.js",
  // This repository: the folder the "open folder" test reads the history of.
  workspaceFolder: root,
  mocha: { ui: "bdd", timeout: 60_000 },
  launchArgs: ["--disable-extensions", "--disable-workspace-trust"],
  env: { TMPDIR: temporary, SUNDGREN_TEST_ROOT: root },
});
