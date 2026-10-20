// Integration tests: run in a real VS Code by @vscode/test-cli (`npm run test:integration`).
import { defineConfig } from "@vscode/test-cli";

export default defineConfig({
  files: "out/test/integration/**/*.test.js",
  workspaceFolder: "src/test/integration/fixture",
  mocha: { ui: "bdd", timeout: 20_000 },
  launchArgs: ["--disable-extensions", "--disable-workspace-trust"],
});
