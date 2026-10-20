// End to end: the pluck command in a real VS Code, running the sundgren built from this repo on
// a fresh git repo per test (docs/examples/skip_divide/project). Linux only: it reads /proc.
import * as assert from "node:assert/strict";
import {
  appendFileSync,
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
import {
  BINARY,
  commitAll,
  expectKind,
  type Fixture,
  fixtures,
  git,
  makeFixture,
  openTest,
  pidLines,
  pluck,
  PYTHON,
  ROOT,
  TEST_FILE,
  targetLine,
} from "../fixture.js";

const RUST_LIB = `#[cfg(test)]
mod tests {
    #[test]
    fn doubles() {
        assert_eq!(super::double(2), 4);
    }
}

pub fn double(x: i32) -> i32 {
    x * 2
}
`;

const SLOW_TEST = `

def test_slow():
    import time

    time.sleep(30)
`;

/** A crate whose test sits above the code it covers, so `#[ignore]` shifts that code down. */
function makeRustFixture(): string {
  const directory = mkdtempSync(path.join(fixtures, "crate-"));
  mkdirSync(path.join(directory, "src"));
  writeFileSync(
    path.join(directory, "Cargo.toml"),
    '[package]\nname = "fixture"\nversion = "0.1.0"\nedition = "2021"\n',
  );
  writeFileSync(path.join(directory, "src", "lib.rs"), RUST_LIB);
  writeFileSync(path.join(directory, ".gitignore"), "target/\nCargo.lock\n");
  git(directory, "init", "-q");
  commitAll(directory);
  return directory;
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
    // The sundgren.path setting decides which sundgren runs.
    const parents = new Set(pidLines(fixture).map((line) => line.split(" ", 2)[1]));
    assert.deepEqual([...parents], [BINARY]);
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
    const fixture = makeFixture({ checkSleep: 30 });
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

  it("compares the commit --check saw, even if HEAD moves during the run", async () => {
    const fixture = makeFixture({ checkSleep: 2 });
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const running = pluck(editor.document);
    await waitFor(
      "sundgren's check",
      () => pidLines(fixture).some((l) => l.includes("import coverage")),
      20_000,
    );
    // Two more lines in divide: plucking test_divide would lose 5 lines on this commit.
    const calc = path.join(fixture.directory, "mypackage", "calc.py");
    const text = readFileSync(calc, "utf8");
    writeFileSync(
      calc,
      text.replace("def divide(a, b):\n", "def divide(a, b):\n    a = a\n    b = b\n"),
    );
    commitAll(fixture.directory);
    const { result } = expectKind(await running, "done");
    assert.equal(result.linesLost, 3);
  });

  it("reports lost lines in the committed file when the pluck shifts them (Rust)", async () => {
    const directory = makeRustFixture();
    const editor = await openTest(path.join(directory, "src", "lib.rs"));
    const line = RUST_LIB.split("\n").indexOf("    fn doubles() {");
    const { result } = expectKind(await pluck(editor.document, line, "doubles"), "done");
    // In the committed file: the test's body (4-6) and `double` (9-11). The patched tree has
    // them one line further down, below the inserted `#[ignore]`.
    assert.deepEqual(Object.fromEntries(result.lostLines), { "src/lib.rs": [4, 5, 6, 9, 10, 11] });
  });
});
