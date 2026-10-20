// Shared by the end-to-end tests: fixture repos and running the pluck command.
import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  appendFileSync,
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import * as vscode from "vscode";

import type { Outcome } from "../extension.js";

// Set by .vscode-test.mjs or vsix/run.ts.
export const ROOT = process.env["SUNDGREN_TEST_ROOT"] ?? "";
export const BINARY = process.env["SUNDGREN_BIN"] ?? path.join(ROOT, "target", "debug", "sundgren");
export const PYTHON = path.join(ROOT, ".venv", "bin", "python");
export const TEST_FILE = path.join("tests", "test_calc.py");
const TARGET = "test_divide";
export const fixtures = path.join(tmpdir(), "fixtures");

export interface Fixture {
  readonly directory: string;
  /**
  Written by the fixture's python wrapper, one line per python process sundgren starts:
  "<pid> <parent's executable> <args>".
  */
  readonly pids: string;
}

export interface FixtureOptions {
  readonly extraTest?: string;
  /** Seconds the python wrapper sleeps on sundgren's `--check` tool check. */
  readonly checkSleep?: number;
}

export function git(cwd: string, ...arguments_: string[]): string {
  return execFileSync("git", arguments_, { cwd, encoding: "utf8", stdio: "pipe" });
}

/** The example project in a new git repo, with `.venv/bin/python` wrapping the repo's venv. */
export function makeFixture(options: FixtureOptions = {}): Fixture {
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
    options.checkSleep === undefined
      ? ""
      : `case "$*" in *"import coverage"*) sleep ${String(options.checkSleep)} ;; esac\n`;
  writeFileSync(
    wrapper,
    `#!/bin/sh\necho "$$ $(readlink /proc/$PPID/exe) $*" >> '${pids}'\n${slowCheck}exec '${PYTHON}' "$@"\n`,
  );
  chmodSync(wrapper, 0o755);
  writeFileSync(path.join(directory, ".gitignore"), ".venv/\npids\n");
  git(directory, "init", "-q");
  commitAll(directory);
  return { directory, pids };
}

export function commitAll(directory: string): void {
  git(directory, "add", "-A");
  const identity = ["-c", "user.name=test", "-c", "user.email=test@example.com"];
  git(directory, ...identity, "commit", "-qm", "commit");
}

export async function openTest(file: string): Promise<vscode.TextEditor> {
  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
  return vscode.window.showTextDocument(document);
}

/** The 0-based line of `def test_divide(`, the test every case plucks. */
export function targetLine(document: vscode.TextDocument): number {
  const line = document
    .getText()
    .split("\n")
    .findIndex((text) => text.startsWith(`def ${TARGET}(`));
  assert.notEqual(line, -1, `no ${TARGET} in ${document.uri.fsPath}`);
  return line;
}

export function pluck(
  document: vscode.TextDocument,
  line = targetLine(document),
  name = TARGET,
): Thenable<Outcome> {
  return vscode.commands.executeCommand<Outcome>("sundgren.pluckTest", document.uri, line, name);
}

/** The outcome, if it is of `kind`; fails the test otherwise. */
export function expectKind<K extends Outcome["kind"]>(
  outcome: Outcome,
  kind: K,
): Extract<Outcome, { kind: K }> {
  if (outcome.kind !== kind) {
    assert.fail(`expected ${kind}, got ${JSON.stringify(outcome)}`);
  }
  return outcome as Extract<Outcome, { kind: K }>;
}

export function pidLines(fixture: Fixture): string[] {
  return existsSync(fixture.pids)
    ? readFileSync(fixture.pids, "utf8")
        .split("\n")
        .filter((line) => line !== "")
    : [];
}
