// The VS Code side: a code lens above each test, the pluck command, progress with Cancel, and
// the results. The checks and the sundgren process handling live in plain modules.
/* eslint-disable security/detect-non-literal-fs-filename -- temp files and workspace paths */

import { mkdtempSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import { parseCheck, planFingerprint, type CheckResult } from "./check.js";
import { insertionPatch, patchedLine, pluckInsertions } from "./patch.js";
import { checkPreconditions, type Ready } from "./preconditions.js";
import { findInterpreter } from "./python.js";
import { headline, notSkippedProblem, parseReport, type PluckResult } from "./report.js";
import {
  locateSundgren,
  readJson,
  runSundgren,
  type RunHandle,
  type RunResult,
} from "./sundgren.js";
import { findTests, isPythonTestFile, type FoundTest, type Language } from "./tests.js";

const CONFIRMED_KEY = "sundgren.confirmedPlans";

type Progress = vscode.Progress<{ message?: string }>;

/** How a pluck ended; returned by the `sundgren.pluckTest` command, for tests and scripts. */
export type Outcome =
  | { readonly kind: "done"; readonly summary: string; readonly result: PluckResult }
  | { readonly kind: "refused"; readonly message: string }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "cancelled"; readonly wasKilled: boolean }
  | { readonly kind: "declined" }
  | { readonly kind: "busy" };

/** The one pluck that may run at a time. */
interface Pluck {
  readonly file: string;
  readonly line: number;
  handle: RunHandle | undefined;
  isCancelling: boolean;
}

function languageOf(document: vscode.TextDocument): Language | undefined {
  if (document.uri.scheme !== "file") {
    return undefined;
  }
  if (document.languageId === "rust") {
    return "rust";
  }
  const isPytestFile =
    document.languageId === "python" && isPythonTestFile(path.basename(document.uri.fsPath));
  return isPytestFile ? "python" : undefined;
}

class Extension implements vscode.CodeLensProvider {
  private readonly lensesChanged = new vscode.EventEmitter<void>();
  private readonly output = vscode.window.createOutputChannel("sundgren");
  private readonly lostLineDecoration = vscode.window.createTextEditorDecorationType({
    isWholeLine: true,
    backgroundColor: new vscode.ThemeColor("diffEditor.removedLineBackground"),
    overviewRulerColor: new vscode.ThemeColor("editorOverviewRuler.deletedForeground"),
    overviewRulerLane: vscode.OverviewRulerLane.Left,
  });
  private readonly highlights = new Map<string, readonly number[]>();
  private active: Pluck | undefined;
  readonly onDidChangeCodeLenses = this.lensesChanged.event;

  constructor(private readonly context: vscode.ExtensionContext) {}

  private async pluck(uri?: vscode.Uri, line?: number, name?: string): Promise<Outcome> {
    if (this.active !== undefined) {
      const choice = await vscode.window.showWarningMessage(
        "A pluck is already running.",
        "Cancel it",
      );
      if (choice === "Cancel it") {
        this.cancel();
      }
      return { kind: "busy" };
    }
    const target = targetTest(uri, line, name);
    if ("error" in target) {
      void vscode.window.showErrorMessage(`sundgren: ${target.error}`);
      return { kind: "refused", message: target.error };
    }
    const pluck: Pluck = {
      file: target.document.uri.fsPath,
      line: target.test.line,
      handle: undefined,
      isCancelling: false,
    };
    this.active = pluck;
    this.lensesChanged.fire();
    try {
      return await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: `sundgren: plucking ${target.test.qualifiedName}`,
          cancellable: true,
        },
        async (progress, token) => {
          token.onCancellationRequested(() => {
            this.cancel();
          });
          return this.pluckSteps(target.document, target.test, pluck, progress);
        },
      );
    } finally {
      this.active = undefined;
      this.lensesChanged.fire();
    }
  }

  private async pluckSteps(
    document: vscode.TextDocument,
    test: FoundTest,
    pluck: Pluck,
    progress: Progress,
  ): Promise<Outcome> {
    progress.report({ message: "checking…" });
    const config = vscode.workspace.getConfiguration();
    const ready = await checkPreconditions({
      file: document.uri.fsPath,
      language: test.language,
      test,
      documentText: document.getText(),
      isDocumentDirty: document.isDirty,
      isWorkspaceTrusted: vscode.workspace.isTrusted,
      platform: process.platform,
      pythonTestingCwd: config.get<string>("python.testing.cwd"),
      pytestArgs: config.get<unknown[]>("python.testing.pytestArgs"),
      linkedProjects: config.get<unknown[]>("rust-analyzer.linkedProjects"),
    });
    if (!ready.isOk) {
      return this.refuse(ready.message);
    }
    if (pluck.isCancelling) {
      return this.cancelled(undefined);
    }
    const binary = locateSundgren({
      setting: vscode.workspace.getConfiguration("sundgren").get<string>("path") ?? "",
      bundled: this.context.asAbsolutePath(path.join("bundled", "sundgren")),
      repository: ready.repository,
      pathEnvironment: process.env["PATH"],
    });
    if (binary === undefined) {
      return this.refuse(
        "sundgren wasn't found: reinstall the extension, or install sundgren with `uv tool install sundgren`.",
      );
    }
    if ("error" in binary) {
      return this.refuse(binary.error);
    }
    const work = mkdtempSync(path.join(tmpdir(), "sundgren-vscode-"));
    try {
      return await this.checkAndRun({
        binary: binary.path,
        ready,
        document,
        test,
        pluck,
        progress,
        work,
      });
    } finally {
      rmSync(work, { recursive: true, force: true });
    }
  }

  private async checkAndRun(run: {
    readonly binary: string;
    readonly ready: Ready;
    readonly document: vscode.TextDocument;
    readonly test: FoundTest;
    readonly pluck: Pluck;
    readonly progress: Progress;
    readonly work: string;
  }): Promise<Outcome> {
    const { binary, ready, test, pluck, work } = run;
    const patch = path.join(work, "pluck.patch");
    writeFileSync(
      patch,
      insertionPatch(ready.path, ready.text, pluckInsertions(ready.test, ready.text)),
    );
    const commandArguments = await runArguments(ready, test.language, patch, run.document.uri);
    const check = await this.step(
      pluck,
      binary,
      [...commandArguments, "--base", "HEAD", "--check"],
      ready.repository,
    );
    if (check.isCancelled) {
      return this.cancelled(check);
    }
    if (check.code === 2 && check.stderr.includes("unexpected argument '--check'")) {
      return this.refuse(
        `The sundgren at ${binary} is too old for this extension (it has no --check). Update it, or clear sundgren.path to use the bundled one.`,
      );
    }
    const plan = parseCheck(check.stdout);
    if (plan === undefined) {
      return this.failed("sundgren's check gave no readable result.", check);
    }
    if (!plan.isOk) {
      for (const problem of plan.problems) {
        this.output.appendLine(`Refused (${problem.code}): ${problem.message}`);
      }
      return this.refuse(checkRefusal(plan));
    }
    if (!(await this.isConfirmed(plan, test))) {
      return { kind: "declined" };
    }
    run.progress.report({
      message: "running the tests on the last commit and with the test plucked…",
    });
    const report = path.join(work, "report.json");
    // The exact commit --check resolved, so a commit made meanwhile can't change what's compared.
    const output = ["--base", plan.baseRef, "--format", "json", "--full", "-o", report];
    const result = await this.step(
      pluck,
      binary,
      [...commandArguments, ...output],
      ready.repository,
    );
    if (result.isCancelled) {
      return this.cancelled(result);
    }
    if (result.code !== 0) {
      return this.failed(`sundgren exited with ${String(result.code ?? result.signal)}.`, result);
    }
    const exclude = {
      path: ready.path,
      name: ready.test.name,
      line: patchedLine(ready.test, pluckInsertions(ready.test, ready.text)),
    };
    const parsed = parseReport(readJson(report), exclude);
    if (parsed === undefined) {
      return this.failed("sundgren's report couldn't be read.", result);
    }
    const notSkipped =
      test.language === "python"
        ? notSkippedProblem(parsed, ready.path, ready.test.qualifiedName)
        : undefined;
    return notSkipped === undefined
      ? this.show(ready, test, parsed)
      : this.failed(notSkipped, result);
  }

  private async step(
    pluck: Pluck,
    binary: string,
    commandArguments: readonly string[],
    cwd: string,
  ): Promise<RunResult> {
    this.output.appendLine(`$ ${binary} ${commandArguments.join(" ")}`);
    const handle = runSundgren(binary, commandArguments, cwd, (chunk) => {
      this.output.append(chunk);
    });
    pluck.handle = handle;
    if (pluck.isCancelling) {
      handle.cancel();
    }
    const result = await handle.done;
    pluck.handle = undefined;
    return result;
  }

  private async isConfirmed(plan: CheckResult, test: FoundTest): Promise<boolean> {
    const known = this.context.workspaceState.get<string[]>(CONFIRMED_KEY) ?? [];
    const fingerprint = planFingerprint(plan);
    // Automated extension tests can't answer a modal dialog.
    const isTest = this.context.extensionMode === vscode.ExtensionMode.Test;
    if (isTest || known.includes(fingerprint)) {
      return true;
    }
    const detail = [
      `sundgren will run the tests twice in temporary checkouts of ${plan.repository}: on the last commit, and with ${test.qualifiedName} skipped.`,
      "",
      `Preset: ${plan.preset}`,
      ...(plan.python === undefined ? [] : [`Python: ${plan.python}`]),
      "Commands:",
      ...plan.commands.map((c) => `  ${c}`),
    ].join("\n");
    const choice = await vscode.window.showInformationMessage(
      "sundgren: run this?",
      { modal: true, detail },
      "Run",
    );
    if (choice !== "Run") {
      return false;
    }
    await this.context.workspaceState.update(CONFIRMED_KEY, [...known, fingerprint]);
    return true;
  }

  private refuse(message: string): Outcome {
    this.output.appendLine(`Refused: ${message}`);
    void vscode.window.showWarningMessage(`sundgren: ${message}`);
    return { kind: "refused", message };
  }

  private failed(message: string, result: RunResult): Outcome {
    this.output.appendLine(message);
    const tail = result.stderr.trim().split("\n").slice(-5).join("\n");
    const text = tail === "" ? message : `${message}\n${tail}`;
    void vscode.window.showErrorMessage(`sundgren: ${text}`, "Show output").then((choice) => {
      if (choice === "Show output") {
        this.output.show(true);
      }
    });
    return { kind: "failed", message: text };
  }

  private cancelled(result: RunResult | undefined): Outcome {
    this.output.appendLine("Cancelled.");
    const wasKilled = result?.isKilled === true;
    if (wasKilled) {
      void vscode.window.showWarningMessage(
        "sundgren: cancelled, but sundgren didn't stop in time and was killed; its temporary checkouts may be left over (`git worktree prune` removes them).",
      );
    } else {
      void vscode.window.showInformationMessage("sundgren: pluck cancelled.");
    }
    return { kind: "cancelled", wasKilled };
  }

  private show(ready: Ready, test: FoundTest, result: PluckResult): Outcome {
    this.highlights.clear();
    for (const [file, lines] of result.lostLines) {
      this.highlights.set(realPath(path.join(ready.repository, file)), lines);
    }
    this.applyHighlights();
    const summary = headline(test.qualifiedName, result);
    this.output.appendLine(summary);
    for (const [file, lines] of result.lostLines) {
      this.output.appendLine(`  ${file}: lost lines ${lines.join(", ")}`);
    }
    for (const lost of result.functions) {
      this.output.appendLine(`  ${lost.path}: ${lost.name} ${lost.status.replace("_", " ")}`);
    }
    const first = result.functions.at(0);
    const actions = first === undefined ? ["Show output"] : [`Go to ${first.name}`, "Show output"];
    void vscode.window.showInformationMessage(`sundgren: ${summary}`, ...actions).then((choice) => {
      if (choice === "Show output") {
        this.output.show(true);
      } else if (choice !== undefined && first !== undefined) {
        void revealLine(path.join(ready.repository, first.path), first.start);
      }
    });
    return { kind: "done", summary, result };
  }

  private applyHighlights(): void {
    for (const editor of vscode.window.visibleTextEditors) {
      const lines = this.highlights.get(realPath(editor.document.uri.fsPath)) ?? [];
      const ranges = lines.map((l) => new vscode.Range(l - 1, 0, l - 1, 0));
      editor.setDecorations(this.lostLineDecoration, ranges);
    }
  }

  /** Stop the running pluck: tell sundgren at once, and show that it's stopping. */
  cancel(): void {
    const active = this.active;
    if (active === undefined || active.isCancelling) {
      return;
    }
    active.isCancelling = true;
    active.handle?.cancel();
    this.lensesChanged.fire();
  }

  register(): void {
    const selector: vscode.DocumentSelector = [
      { scheme: "file", language: "python" },
      { scheme: "file", language: "rust" },
    ];
    this.context.subscriptions.push(
      this.output,
      this.lostLineDecoration,
      this.lensesChanged,
      vscode.languages.registerCodeLensProvider(selector, this),
      vscode.commands.registerCommand(
        "sundgren.pluckTest",
        (uri?: vscode.Uri, line?: number, name?: string) => this.pluck(uri, line, name),
      ),
      vscode.commands.registerCommand("sundgren.cancel", () => {
        this.cancel();
      }),
      vscode.window.onDidChangeVisibleTextEditors(() => {
        this.applyHighlights();
      }),
      vscode.workspace.onDidChangeTextDocument((event) => {
        if (this.highlights.delete(realPath(event.document.uri.fsPath))) {
          this.applyHighlights();
        }
      }),
      vscode.workspace.onDidGrantWorkspaceTrust(() => {
        this.lensesChanged.fire();
      }),
    );
  }

  provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
    const language = languageOf(document);
    if (language === undefined || !vscode.workspace.isTrusted) {
      return [];
    }
    return findTests(language, document.getText()).map((test) => {
      const range = new vscode.Range(test.line, 0, test.line, 0);
      const active = this.active;
      if (active?.file === document.uri.fsPath && active.line === test.line) {
        const title = active.isCancelling ? "sg: cancelling…" : "sg: plucking… (click to cancel)";
        return new vscode.CodeLens(range, { title, command: "sundgren.cancel" });
      }
      return new vscode.CodeLens(range, {
        title: "sg: pluck this test",
        tooltip: `Skip ${test.qualifiedName} in a patch and see what happens to coverage`,
        command: "sundgren.pluckTest",
        arguments: [document.uri, test.line, test.name],
      });
    });
  }
}

/** `file` with symlinks resolved, as git reports paths; `file` itself if it doesn't exist. */
function realPath(file: string): string {
  try {
    return realpathSync(file);
  } catch {
    return file;
  }
}

async function revealLine(file: string, line: number): Promise<void> {
  const document = await vscode.workspace.openTextDocument(file);
  const editor = await vscode.window.showTextDocument(document);
  const position = new vscode.Position(Math.max(0, line - 1), 0);
  editor.selection = new vscode.Selection(position, position);
  editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenter);
}

/** The test the command is for: from the code lens arguments, else the one under the cursor. */
function targetTest(
  uri?: vscode.Uri,
  line?: number,
  name?: string,
): { document: vscode.TextDocument; test: FoundTest } | { error: string } {
  const document =
    uri === undefined
      ? vscode.window.activeTextEditor?.document
      : vscode.workspace.textDocuments.find((d) => d.uri.toString() === uri.toString());
  if (document === undefined) {
    return { error: "Open the test file first." };
  }
  const language = languageOf(document);
  if (language === undefined) {
    return { error: "This isn't a Rust file or a pytest test file (test_*.py or *_test.py)." };
  }
  const tests = findTests(language, document.getText());
  const cursor = vscode.window.activeTextEditor?.selection.active.line ?? 0;
  const test =
    line === undefined
      ? tests.findLast((t) => t.line <= cursor)
      : tests.find((t) => t.line === line && t.name === name);
  return test === undefined
    ? { error: "No test at the cursor: put the cursor in a test, or use the lens above it." }
    : { document, test };
}

/** sundgren's first check problem as a sentence, with a VS Code hint for a Python tool. */
function checkRefusal(plan: CheckResult): string {
  const problem = plan.problems.at(0);
  if (problem === undefined) {
    return "sundgren's check failed.";
  }
  const text = problem.message.trim();
  const end = /[.!?]$/u.test(text) ? "" : ".";
  const sentence = `${text.charAt(0).toUpperCase()}${text.slice(1)}${end}`;
  const isPythonTool = problem.code === "check_failed" && plan.python !== undefined;
  return isPythonTool
    ? `${sentence} Pick another with "Python: Select Interpreter", or create a .venv in the project.`
    : sentence;
}

async function runArguments(
  ready: Ready,
  language: Language,
  patch: string,
  file: vscode.Uri,
): Promise<string[]> {
  const base = ["run", "--repo", ready.repository, "--patch", patch];
  if (language === "rust") {
    return [...base, "--preset", "cargo-llvm-cov"];
  }
  const interpreter = await findInterpreter(ready.repository, file);
  const python = interpreter === undefined ? [] : ["--python", interpreter.path];
  return [...base, "--preset", "pytest", "--all-tests", ...python];
}

export function activate(context: vscode.ExtensionContext): void {
  const extension = new Extension(context);
  extension.register();
  // Disposed when VS Code shuts the extension down: a running pluck is cancelled, not orphaned.
  context.subscriptions.push({
    dispose: () => {
      extension.cancel();
    },
  });
}

export function deactivate(): void {
  // Everything is cleaned up through context.subscriptions.
}
