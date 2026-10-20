// The patch that plucks a test: a unified diff that inserts a skip marker, in the form
// `git apply` accepts. Built from the committed file text; nothing on disk changes.

import type { FoundTest } from "./tests.js";

export const SKIP_REASON = "sundgren: plucked";
const CONTEXT = 3;

export interface Insertion {
  /** 0-based line the new lines go before. */
  readonly before: number;
  readonly lines: readonly string[];
}

// eslint-disable-next-line security/detect-unsafe-regex -- anchored per line; input is one file
const IMPORT_PYTEST = /^import[ \t]+(?:[\w.]+[ \t]*,[ \t]*)*pytest[ \t]*(?:,|$|#)/mu;

/** Whether a module-level `pytest` name is bound, so `@pytest.mark.skip` works as is. */
export function hasPytestImport(text: string): boolean {
  return IMPORT_PYTEST.test(text);
}

/** The lines to insert for `test`, in file order. */
export function pluckInsertions(test: FoundTest, text: string): Insertion[] {
  if (test.language === "rust") {
    return [{ before: test.line, lines: [`${test.indent}#[ignore]`] }];
  }
  // Above all the test's decorators: one that wraps the function without functools.wraps would
  // otherwise hide the mark from pytest.
  const skip: Insertion = {
    before: test.lensLine,
    lines: [`${test.indent}@pytest.mark.skip(reason="${SKIP_REASON}")`],
  };
  return hasPytestImport(text)
    ? [skip]
    : [{ before: test.topLevelLine, lines: ["import pytest"] }, skip];
}

/** 1-based line of the test's `def` or `fn` once `insertions` (all above it) are applied. */
export function patchedLine(test: FoundTest, insertions: readonly Insertion[]): number {
  const added = insertions
    .filter((ins) => ins.before <= test.line)
    .reduce((sum, ins) => sum + ins.lines.length, 0);
  return test.line + added + 1;
}

/** A git-style unified diff of `path` with `insertions` applied to `text` (LF line endings). */
export function insertionPatch(
  path: string,
  text: string,
  insertions: readonly Insertion[],
): string {
  if (text.includes("\r")) {
    throw new Error("files with CR line endings are not supported");
  }
  const hasFinalNewline = text.endsWith("\n");
  const lines = (hasFinalNewline ? text.slice(0, -1) : text).split("\n");
  const sorted = insertions.toSorted((a, b) => a.before - b.before);
  for (const ins of sorted) {
    if (ins.lines.length === 0 || ins.before < 0 || ins.before > lines.length) {
      throw new Error(`bad insertion at line ${String(ins.before)}`);
    }
  }
  const out = [`diff --git a/${path} b/${path}`, `--- a/${path}`, `+++ b/${path}`];
  let added = 0;
  for (const group of groupInsertions(sorted)) {
    const hunk = buildHunk(lines, hasFinalNewline, group, added);
    out.push(hunk.header, ...hunk.body);
    added += hunk.added;
  }
  return `${out.join("\n")}\n`;
}

/** One hunk for insertions close enough to share context; `before` counts earlier hunks' lines. */
function buildHunk(
  lines: readonly string[],
  hasFinalNewline: boolean,
  group: readonly Insertion[],
  before: number,
): { header: string; body: string[]; added: number } {
  const first = group.at(0)?.before ?? 0;
  const last = group.at(-1)?.before ?? first;
  const start = Math.max(0, first - CONTEXT);
  const end = Math.min(lines.length, last + CONTEXT);
  // Several insertions may go before the same line (an import right above a test); keep all, in order.
  const byLine = new Map<number, string[]>();
  for (const ins of group) {
    byLine.set(ins.before, [...(byLine.get(ins.before) ?? []), ...ins.lines]);
  }
  const body: string[] = [];
  let added = 0;
  for (let index = start; index <= end; index += 1) {
    const inserted = byLine.get(index) ?? [];
    body.push(...inserted.map((l) => `+${l}`));
    added += inserted.length;
    if (index >= end) {
      continue;
    }
    body.push(` ${lines.at(index) ?? ""}`);
    if (!hasFinalNewline && index === lines.length - 1) {
      body.push(String.raw`\ No newline at end of file`);
    }
  }
  const oldCount = end - start;
  const oldStart = oldCount === 0 ? start : start + 1;
  const header = `@@ -${range(oldStart, oldCount)} +${range(start + 1 + before, oldCount + added)} @@`;
  return { header, body, added };
}

function range(start: number, count: number): string {
  return count === 1 ? String(start) : `${String(start)},${String(count)}`;
}

/** Insertions whose context windows touch share one hunk. */
function groupInsertions(sorted: readonly Insertion[]): Insertion[][] {
  const groups: Insertion[][] = [];
  for (const ins of sorted) {
    const current = groups.at(-1);
    const previous = current?.at(-1);
    if (previous !== undefined && ins.before - previous.before <= 2 * CONTEXT) {
      current?.push(ins);
    } else {
      groups.push([ins]);
    }
  }
  return groups;
}
