// Integration tests: run in a real VS Code by @vscode/test-cli (`npm run test:integration`).
/* eslint-disable sonarjs/no-os-command-from-path -- the git on PATH, as the tests use */
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import path from "node:path";

import { defineConfig } from "@vscode/test-cli";

const root = path.resolve(import.meta.dirname, "../..");
const temporary = path.join(import.meta.dirname, ".vscode-test", "tmp");
rmSync(temporary, { recursive: true, force: true });
// The folder VS Code opens: a repository with one commit.
const workspace = path.join(temporary, "workspace");
mkdirSync(workspace, { recursive: true });
const environment = { ...process.env, GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_NOSYSTEM: "1" };
const identity = ["-c", "user.name=Ada", "-c", "user.email=ada@example.com"];
execFileSync("git", ["-C", workspace, "init", "-q", "-b", "main"], { env: environment });
execFileSync("git", ["-C", workspace, ...identity, "commit", "-q", "--allow-empty", "-m", "a"], {
  env: environment,
});

export default defineConfig({
  // CI also runs the oldest version package.json's engines.vscode allows.
  version: process.env["VSCODE_TEST_VERSION"] ?? "stable",
  files: "out/test/integration/**/*.test.js",
  workspaceFolder: workspace,
  mocha: { ui: "bdd", timeout: 60_000 },
  launchArgs: ["--disable-extensions", "--disable-workspace-trust"],
  env: { TMPDIR: temporary, SUNDGREN_TEST_ROOT: root },
});
