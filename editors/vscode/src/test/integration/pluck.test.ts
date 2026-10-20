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
  readlinkSync,
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

interface RustFixtureOptions {
  /** A build script that records its PID and sleeps, so the build takes 30 s. */
  readonly isBuildSlow?: boolean;
  /** A test that records its PID and sleeps 30 s. */
  readonly isTestSlow?: boolean;
}

/** The PIDs the slow build script or test wrote to `pids` in a Rust fixture. */
function rustPids(directory: string): number[] {
  const file = path.join(directory, "pids");
  return existsSync(file)
    ? readFileSync(file, "utf8")
        .split("\n")
        .filter((line) => line !== "")
        .map(Number)
    : [];
}

// A crate in a new git repo. By default its test sits above the code it covers, so `#[ignore]`
// shifts that code down.
function makeRustFixture(options: RustFixtureOptions = {}): string {
  const directory = mkdtempSync(path.join(fixtures, "crate-"));
  const pids = path.join(directory, "pids");
  const record = String.raw`std::fs::OpenOptions::new().create(true).append(true).open(${JSON.stringify(pids)}).and_then(|mut f| std::io::Write::write_all(&mut f, format!("{}\n", std::process::id()).as_bytes())).unwrap();`;
  const sleep = "std::thread::sleep(std::time::Duration::from_secs(30));";
  mkdirSync(path.join(directory, "src"));
  writeFileSync(
    path.join(directory, "Cargo.toml"),
    '[package]\nname = "fixture"\nversion = "0.1.0"\nedition = "2021"\n',
  );
  const slowTest =
    options.isTestSlow === true ? `\n#[test]\nfn slow() {\n    ${record}\n    ${sleep}\n}\n` : "";
  writeFileSync(path.join(directory, "src", "lib.rs"), RUST_LIB + slowTest);
  if (options.isBuildSlow === true) {
    writeFileSync(
      path.join(directory, "build.rs"),
      `fn main() {\n    ${record}\n    ${sleep}\n}\n`,
    );
  }
  writeFileSync(path.join(directory, ".gitignore"), "target/\nCargo.lock\npids\n");
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

/** Live processes and their parents, from /proc. */
function processTable(): Map<number, number> {
  const parents = new Map<number, number>();
  for (const entry of readdirSync("/proc")) {
    const pid = Number(entry);
    if (!Number.isSafeInteger(pid)) {
      continue;
    }
    try {
      const stat = readFileSync(`/proc/${entry}/stat`, "utf8");
      const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
      if (fields[0] !== "Z") {
        parents.set(pid, Number(fields[1]));
      }
    } catch {
      // Exited while we looked.
    }
  }
  return parents;
}

/** `pids` and all their live descendants. */
function withDescendants(pids: readonly number[]): number[] {
  const parents = processTable();
  const found = new Set(pids);
  let isGrowing = true;
  while (isGrowing) {
    isGrowing = false;
    for (const [pid, parent] of parents) {
      if (!found.has(parent) || found.has(pid)) {
        continue;
      }

      found.add(pid);
      isGrowing = true;
    }
  }
  return [...found];
}

/** Live processes whose working directory is under `directory`, with their command names. */
function processesIn(directory: string): string[] {
  const found: string[] = [];
  for (const entry of readdirSync("/proc")) {
    const pid = Number(entry);
    if (!Number.isSafeInteger(pid) || !isAlive(pid)) {
      continue;
    }
    try {
      const cwd = readlinkSync(`/proc/${entry}/cwd`);
      if (cwd.startsWith(directory)) {
        const name = readFileSync(`/proc/${entry}/comm`, "utf8").trim();
        // VS Code's own git extension runs git in open repos; that isn't ours.
        if (name !== "git") {
          found.push(`${entry} ${name} in ${cwd}`);
        }
      }
    } catch {
      // Exited, or not ours to read.
    }
  }
  return found;
}

function leftovers(): string[] {
  return readdirSync(tmpdir()).filter((name) => name.startsWith("sundgren-"));
}

function worktrees(directory: string): string[] {
  return git(directory, "worktree", "list", "--porcelain")
    .split("\n")
    .filter((line) => line.startsWith("worktree "));
}

interface CancelCase {
  /** The fixture repo. */
  readonly directory: string;
  readonly document: vscode.TextDocument;
  readonly running: Thenable<Outcome>;
  /** Whether the pluck is in the phase to cancel in. */
  readonly isReady: () => boolean;
  /** PIDs the fixture recorded (test processes), checked too. */
  readonly pids: () => number[];
}

async function lensTitles(document: vscode.TextDocument): Promise<string[]> {
  const lenses = await vscode.commands.executeCommand<vscode.CodeLens[]>(
    "vscode.executeCodeLensProvider",
    document.uri,
  );
  return lenses.map((lens) => lens.command?.title ?? "");
}

// Cancels the running pluck once it is in the phase to cancel in, then checks that it came back
// within 3 s and that every process, worktree and temp dir it made is gone.
async function cancelAndCheck(c: CancelCase): Promise<void> {
  await waitFor("the phase to cancel in", c.isReady, 120_000);
  const sundgren = processesMentioning(c.directory);
  assert.ok(sundgren.length > 0, "no sundgren process found");
  const tree = withDescendants([...sundgren, ...c.pids()]);
  const names = tree.map((pid) => processName(pid)).join(", ");
  const started = Date.now();
  await vscode.commands.executeCommand("sundgren.cancel");
  const outcome = await within(c.running, 3000, "the cancel");
  const elapsed = Date.now() - started;
  assert.deepEqual(outcome, { kind: "cancelled", wasKilled: false });
  console.log(`      cancel: ${String(elapsed)} ms; stopped ${String(tree.length)}: ${names}`);
  const stillAlive = (): number[] => tree.filter((pid) => isAlive(pid));
  await waitFor(
    `processes ${stillAlive().join(", ")} to exit`,
    () => stillAlive().length === 0,
    1000,
  );
  assert.deepEqual(processesIn(tmpdir()), []);
  assert.deepEqual(processesIn(c.directory), []);
  const trees = worktrees(c.directory);
  assert.equal(trees.length, 1, trees.join("\n"));
  await waitFor(`${leftovers().join(", ")} to be removed`, () => leftovers().length === 0, 1000);
  const titles = await lensTitles(c.document);
  assert.ok(
    titles.length > 0 && titles.every((title) => title === "sg: pluck this test"),
    titles.join(", "),
  );
}

function processName(pid: number): string {
  try {
    return readFileSync(`/proc/${String(pid)}/comm`, "utf8").trim();
  } catch {
    return String(pid);
  }
}

/** A cancel case for a Python fixture: the PIDs are the ones its python wrapper logged. */
function pythonCase(
  fixture: Fixture,
  document: vscode.TextDocument,
  running: Thenable<Outcome>,
): Omit<CancelCase, "isReady"> {
  return {
    directory: fixture.directory,
    document,
    running,
    pids: () => pidLines(fixture).map((line) => Number(line.split(" ", 1)[0])),
  };
}

/** Starts plucking `doubles` in a Rust fixture and cancels once `pids` has an entry. */
async function cancelRust(options: RustFixtureOptions): Promise<void> {
  const directory = makeRustFixture(options);
  const editor = await openTest(path.join(directory, "src", "lib.rs"));
  const line = RUST_LIB.split("\n").indexOf("    fn doubles() {");
  const running = pluck(editor.document, line, "doubles");
  await cancelAndCheck({
    directory,
    document: editor.document,
    running,
    isReady: () => rustPids(directory).length > 0,
    pids: () => rustPids(directory),
  });
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
    assert.equal(worktrees(fixture.directory).length, 1);
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

  it("refuses when the interpreter lacks pytest and coverage, and says what to do", async () => {
    const fixture = makeFixture({ isPythonBare: true });
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const outcome = await pluck(editor.document);
    assert.match(expectKind(outcome, "refused").message, /Python: Select Interpreter/u);
    assert.ok(
      pidLines(fixture).every((l) => !l.includes("-m pytest")),
      "tests ran anyway",
    );
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
    await cancelAndCheck({
      ...pythonCase(fixture, editor.document, running),
      isReady: () => {
        if (pytestSeen === 0 && pidLines(fixture).some((l) => l.includes("-m pytest"))) {
          pytestSeen = Date.now();
        }
        return pytestSeen !== 0 && Date.now() - pytestSeen > 1000;
      },
    });
  });

  it("cancels during sundgren's check", async () => {
    const fixture = makeFixture({ checkSleep: 30 });
    const editor = await openTest(path.join(fixture.directory, TEST_FILE));
    const running = pluck(editor.document);
    await cancelAndCheck({
      ...pythonCase(fixture, editor.document, running),
      isReady: () => pidLines(fixture).some((l) => l.includes("import coverage")),
    });
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
    await cancelAndCheck({
      ...pythonCase(fixture, editor.document, running),
      isReady: () => pidLines(fixture).some((l) => l.includes("-m pytest")),
    });
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
    // In the committed file: `double` (9-11); the patched tree has it one line further down,
    // below the inserted `#[ignore]`. The test's own body (4-6) is left out.
    assert.deepEqual(Object.fromEntries(result.lostLines), { "src/lib.rs": [9, 10, 11] });
    assert.equal(result.linesLost, 3);
    assert.ok(result.functions.every((f) => !f.name.endsWith("doubles")));
  });

  it("cancels a Rust pluck while cargo builds", async () => {
    await cancelRust({ isBuildSlow: true });
  });

  it("cancels a Rust pluck while the tests run", async () => {
    await cancelRust({ isTestSlow: true });
  });
});
