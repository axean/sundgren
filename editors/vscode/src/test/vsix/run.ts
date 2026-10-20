// Smoke test of a platform .vsix (Ruff style): install it into a fresh VS Code with the VS Code
// CLI, then run bundled.test.ts against the installed copy, so the test exercises exactly what a
// user gets, bundled binary included. Usage: node out/test/vsix/run.js <path/to/sundgren.vsix>
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";

import {
  downloadAndUnzipVSCode,
  resolveCliArgsFromVSCodeExecutablePath,
  runTests,
} from "@vscode/test-electron";

async function main(): Promise<void> {
  const vsix = path.resolve(process.argv[2] ?? "");
  if (!vsix.endsWith(".vsix") || !existsSync(vsix)) {
    throw new Error("usage: run.js <path/to/sundgren.vsix>");
  }
  const extensionRoot = path.resolve(__dirname, "../../..");
  const work = path.join(extensionRoot, ".vscode-test", "vsix");
  rmSync(work, { recursive: true, force: true });
  const extensions = path.join(work, "extensions");
  const userData = path.join(work, "user-data");
  const temporary = path.join(work, "tmp");
  for (const directory of [extensions, userData, temporary]) {
    mkdirSync(directory, { recursive: true });
  }

  const executable = await downloadAndUnzipVSCode(process.env["VSCODE_TEST_VERSION"] ?? "stable");
  const [cli, ...cliArguments] = resolveCliArgsFromVSCodeExecutablePath(executable);
  if (cli === undefined) {
    throw new Error("no VS Code CLI");
  }
  execFileSync(
    cli,
    [...cliArguments, "--extensions-dir", extensions, "--install-extension", vsix],
    { stdio: "inherit" },
  );
  const installed = readdirSync(extensions).find((name) => name.startsWith("axean.sundgren-"));
  if (installed === undefined) {
    throw new Error(`the .vsix didn't install into ${extensions}`);
  }

  // No sundgren on PATH, so only the bundled binary can be found.
  const searchPath = (process.env["PATH"] ?? "")
    .split(path.delimiter)
    .filter((directory) => !existsSync(path.join(directory, "sundgren")))
    .join(path.delimiter);
  await runTests({
    vscodeExecutablePath: executable,
    // The installed copy, loaded as the extension under test so it runs in test mode.
    extensionDevelopmentPath: path.join(extensions, installed),
    extensionTestsPath: path.join(__dirname, "index.js"),
    launchArgs: [
      "--disable-extensions",
      "--disable-workspace-trust",
      "--user-data-dir",
      userData,
      "--extensions-dir",
      extensions,
    ],
    extensionTestsEnv: {
      PATH: searchPath,
      TMPDIR: temporary,
      SUNDGREN_TEST_ROOT: path.resolve(extensionRoot, "../.."),
      SUNDGREN_EXPECTED_BINARY: path.join(extensions, installed, "bundled", "sundgren"),
    },
  });
}

async function runOrFail(): Promise<void> {
  try {
    await main();
  } catch (error: unknown) {
    console.error(error);
    process.exitCode = 1;
  }
}

void runOrFail();
