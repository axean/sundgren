// End to end: the pluck command in a real VS Code, running the sundgren built from this repo on
// a fresh git repo per test (docs/examples/skip_divide/project). Linux only: it reads /proc.
import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  appendFileSync,
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import type { Outcome } from "../../extension.js";

// Set by .vscode-test.mjs.
const ROOT = process.env["SUNDGREN_TEST_ROOT"] ?? "";
const BINARY = process.env["SUNDGREN_BIN"] ?? path.join(ROOT, "target", "debug", "sundgren");
const PYTHON = path.join(ROOT, ".venv", "bin", "python");
const TEST_FILE = path.join("tests", "test_calc.py");
const TARGET = "test_divide";
const fixtures = path.join(tmpdir(), "fixtures");

const SLOW_TEST = `

def test_slow():
    import time

    time.sleep(30)
`;

interface Fixture {
  readonly directory: string;
  /** Written by the fixture's python wrapper: "<pid> <args>" per python process sundgren starts. */
  readonly pids: string;
}

interface FixtureOptions {
  readonly extraTest?: string;
  /** Make the python wrapper sleep on sundgren's `--check` tool check. */
  readonly isCheckSlow?: boolean;
}

function git(cwd: string, ...arguments_: string[]): string {
  return execFileSync("git", arguments_, { cwd, encoding: "utf8", stdio: "pipe" });
}

/** The example project in a new git repo, with `.venv/bin/python` wrapping the repo's venv. */
function makeFixture(options: FixtureOptions = {}): Fixture {
  const directory = mkdtempSync(path.join(fixtures, "repo-"));
  cpSync(path.join(ROOT, "docs", "examples", "skip_divide", "project"), directory, {
    recursive: true,
  });
  if (options.extraTest !== undefined) {
    appendFileSync(path.join(directory, TEST_FILE), options.extraTest);
  }
  const pids = path.join(directory, "pids");
  const wrapper = path.join(directory, ".venv", "bin", "python");
  mkdirSync(path.dirname(wrapper), { recursive: true });
  const slowCheck =
    options.isCheckSlow === true ? `case "$*" in *"import coverage"*) exec sleep 30 ;; esac\n` : "";
  writeFileSync(
    wrapper,
    `#!/bin/sh\necho "$$ $*" >> '${pids}'\n${slowCheck}exec '${PYTHON}' "$@"\n`,
  );
  chmodSync(wrapper, 0o755);
  writeFileSync(path.join(directory, ".gitignore"), ".venv/\npids\n");
  git(directory, "init", "-q");
  git(directory, "add", "-A");
  git(
    directory,
    "-c",
    "user.name=test",
    "-c",
    "user.email=test@example.com",
    "commit",
    "-qm",
    "base",
  );
  return { directory, pids };
}

async function openTest(file: string): Promise<vscode.TextEditor> {
  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
  return vscode.window.showTextDocument(document);
}

/** The 0-based line of `def test_divide(`, the test every case plucks. */
function targetLine(document: vscode.TextDocument): number {
  const line = document
    .getText()
    .split("\n")
    .findIndex((text) => text.startsWith(`def ${TARGET}(`));
  assert.notEqual(line, -1, `no ${TARGET} in ${document.uri.fsPath}`);
  return line;
}

function pluck(document: vscode.TextDocument): Thenable<Outcome> {
  return vscode.commands.executeCommand<Outcome>(
    "sundgren.pluckTest",
    document.uri,
    targetLine(document),
    TARGET,
  );
}

function pause(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 50));
}

async function waitFor(what: string, isDone: () => boolean, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!isDone()) {
    if (Date.now() > deadline) {
      assert.fail(`timed out waiting for ${what}`);
    }
    await pause();
  }
}

/** The outcome, if it is of `kind`; fails the test otherwise. */
function expectKind<K extends Outcome["kind"]>(
  outcome: Outcome,
  kind: K,
): Extract<Outcome, { kind: K }> {
  if (outcome.kind !== kind) {
    assert.fail(`expected ${kind}, got ${JSON.stringify(outcome)}`);
  }
  return outcome as Extract<Outcome, { kind: K }>;
}

/** Resolves to the outcome, or fails if it takes longer than `timeoutMs`. */
async function within<T>(promise: Thenable<T>, timeoutMs: number, what: string): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      reject(new Error(`${what} took longer than ${String(timeoutMs)} ms`));
    }, timeoutMs);
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    clearTimeout(timer);
  }
}

function pidLines(fixture: Fixture): string[] {
  return existsSync(fixture.pids)
    ? readFileSync(fixture.pids, "utf8")
        .split("\n")
        .filter((line) => line !== "")
    : [];
}

/** PIDs of live processes whose command line mentions `text` (zombies count as gone). */
function processesMentioning(text: string): number[] {
  const found: number[] = [];
  for (const entry of readdirSync("/proc")) {
    const pid = Number(entry);
    if (!Number.isSafeInteger(pid) || pid === process.pid) {
      continue;
    }
    try {
      const commandLine = readFileSync(`/proc/${entry}/cmdline`, "utf8");
      if (commandLine.includes(text) && isAlive(pid)) {
        found.push(pid);
      }
    } catch {
      // Exited while we looked.
    }
  }
  return found;
}

function isAlive(pid: number): boolean {
  try {
    const stat = readFileSync(`/proc/${String(pid)}/stat`, "utf8");
    // The state follows the command name in parentheses: "pid (comm) S ...".
    return stat.slice(stat.lastIndexOf(")") + 2, stat.lastIndexOf(")") + 3) !== "Z";
  } catch {
    return false;
  }
}

function leftovers(): string[] {
  return readdirSync(tmpdir()).filter((name) => name.startsWith("sundgren-"));
}

function worktrees(fixture: Fixture): string[] {
  return git(fixture.directory, "worktree", "list", "--porcelain")
    .split("\n")
    .filter((line) => line.startsWith("worktree "));
}

/** Cancels the running pluck once `isReady` holds, and checks that everything stopped. */
async function cancelAndCheck(
  fixture: Fixture,
  running: Thenable<Outcome>,
  isReady: () => boolean,
): Promise<void> {
  await waitFor("the phase to cancel in", isReady, 20_000);
  const sundgren = processesMentioning(fixture.directory);
  const pythons = pidLines(fixture).map((line) => Number(line.split(" ", 1)[0]));
  assert.ok(sundgren.length > 0, "no sundgren process found");
  const started = Date.now();
  await vscode.commands.executeCommand("sundgren.cancel");
  const outcome = await within(running, 3000, "the cancel");
  assert.deepEqual(outcome, { kind: "cancelled", wasKilled: false });
  // Allow a moment for orphans to be reaped.
  const stillAlive = (): number[] => [...sundgren, ...pythons].filter((pid) => isAlive(pid));
  await waitFor(
    `processes ${stillAlive().join(", ")} to exit`,
    () => stillAlive().length === 0,
    1000,
  );
  assert.ok(Date.now() - started < 4000, "stopping took longer than 4 s");
  assert.equal(worktrees(fixture).length, 1, worktrees(fixture).join("\n"));
  assert.deepEqual(leftovers(), []);
}

describe("pluck, end to end", () => {
  before(async () => {
    assert.ok(ROOT !== "", "SUNDGREN_TEST_ROOT is not set; run through .vscode-test.mjs");
    assert.ok(existsSync(BINARY), `${BINARY} is missing; run \`cargo build\` at the repo root`);
    assert.ok(existsSync(PYTHON), `${PYTHON} is missing; run \`uv sync\` at the repo root`);
    // sundgren's temp dirs land in TMPDIR, which .vscode-test.mjs makes private to this run.
    assert.equal(tmpdir(), process.env["TMPDIR"]);
    mkdirSync(fixtures, { recursive: true });
    await vscode.workspace
      .getConfiguration("sundgren")
      .update("path", BINARY, vscode.ConfigurationTarget.Global);
  });

  after(async () => {
    await vscode.workspace
      .getConfiguration("sundgren")
      .update("path", undefined, vscode.ConfigurationTarget.Global);
    rmSync(fixtures, { recursive: true, force: true });
  });

  afterEach(async () => {
    await vscode.commands.executeCommand("workbench.action.revertAndCloseActiveEditor");
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  it("plucks test_divide and reports what it covered", async () => {
    const fixture = makeFixture();
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const outcome = await pluck(editor.document);
    const { result } = expectKind(outcome, "done");
    assert.equal(result.linesLost, 3);
    assert.deepEqual(
      result.functions.map((f) => [f.path, f.name, f.status]),
      [["mypackage/calc.py", "divide", "went_dark"]],
    );
    assert.equal(worktrees(fixture).length, 1);
    assert.deepEqual(leftovers(), []);
  });

  it("works when the repo is opened through a symlink", async () => {
    const fixture = makeFixture();
    const link = `${fixture.directory}-link`;
    symlinkSync(fixture.directory, link);
    const editor = await openTest(path.join(link, TEST_FILE));
    const outcome = await pluck(editor.document);
    expectKind(outcome, "done");
  });

  it("refuses a dirty tree without running anything", async () => {
    const fixture = makeFixture();
    appendFileSync(path.join(fixture.directory, "mypackage", "calc.py"), "# edited\n");
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const started = Date.now();
    const outcome = await pluck(editor.document);
    assert.match(expectKind(outcome, "refused").message, /Commit or stash/u);
    assert.ok(Date.now() - started < 2000, "the refusal took longer than 2 s");
    assert.deepEqual(pidLines(fixture), []);
  });

  it("refuses an unsaved document without running anything", async () => {
    const fixture = makeFixture();
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const line = targetLine(editor.document);
    await editor.edit((edit) => {
      edit.insert(new vscode.Position(line + 1, 0), "    pass\n");
    });
    assert.equal(editor.document.isDirty, true);
    const outcome = await pluck(editor.document);
    assert.match(expectKind(outcome, "refused").message, /Save the file first/u);
    assert.deepEqual(pidLines(fixture), []);
  });

  it("cancels while the tests run, and stops and cleans up everything", async () => {
    const fixture = makeFixture({ extraTest: SLOW_TEST });
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const running = pluck(editor.document);
    // Cancel once pytest has been running for a moment (it then sleeps in test_slow).
    let pytestSeen = 0;
    await cancelAndCheck(fixture, running, () => {
      if (pytestSeen === 0 && pidLines(fixture).some((l) => l.includes("-m pytest"))) {
        pytestSeen = Date.now();
      }
      return pytestSeen !== 0 && Date.now() - pytestSeen > 1000;
    });
  });

  it("cancels during sundgren's check", async () => {
    const fixture = makeFixture({ isCheckSlow: true });
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const running = pluck(editor.document);
    await cancelAndCheck(fixture, running, () =>
      pidLines(fixture).some((l) => l.includes("import coverage")),
    );
    assert.ok(
      pidLines(fixture).every((l) => !l.includes("-m pytest")),
      "tests ran after the cancel",
    );
  });

  it("shows the plucking lens while it runs", async () => {
    const fixture = makeFixture({ extraTest: SLOW_TEST });
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const running = pluck(editor.document);
    const titles = async (): Promise<string[]> => {
      const lenses = await vscode.commands.executeCommand<vscode.CodeLens[]>(
        "vscode.executeCodeLensProvider",
        editor.document.uri,
      );
      return lenses.map((lens) => lens.command?.title ?? "");
    };
    let current: string[] = [];
    const deadline = Date.now() + 5000;
    while (current.every((t) => !t.startsWith("sg: plucking")) && Date.now() < deadline) {
      current = await titles();
      await pause();
    }
    assert.ok(current.includes("sg: plucking… (click to cancel)"), current.join(", "));
    await cancelAndCheck(fixture, running, () =>
      pidLines(fixture).some((l) => l.includes("-m pytest")),
    );
  });
});
