// The patch that plucks a test: a unified diff that inserts a skip marker, in the form
// `git apply` accepts. Built from the committed file text; nothing on disk changes.

import type { FoundTest } from "./tests.js";

export const SKIP_REASON = "sundgren: plucked";
const CONTEXT = 3;

interface Insertion {
  /** 0-based line the new lines go before. */
  readonly before: number;
  readonly lines: readonly string[];
}

/** Whether a module-level `pytest` name is bound, so `@pytest.mark.skip` works as is. */
export function importsPytest(text: string): boolean {
  return /^import[ \t]+(?:[\w.]+[ \t]*,[ \t]*)*pytest[ \t]*(?:,|$|#)/mu.test(text);
}

/** The lines to insert for `test`, in file order. */
export function pluckInsertions(test: FoundTest, text: string): Insertion[] {
  if (test.language === "rust") {
    return [{ before: test.line, lines: [`${test.indent}#[ignore]`] }];
  }
  const skip: Insertion = { before: test.line, lines: [`${test.indent}@pytest.mark.skip(reason="${SKIP_REASON}")`] };
  return importsPytest(text) ? [skip] : [{ before: test.topLevelLine, lines: ["import pytest"] }, skip];
}

/** A git-style unified diff of `path` with `insertions` applied to `text` (LF line endings). */
export function insertionPatch(path: string, text: string, insertions: readonly Insertion[]): string {
  if (text.includes("\r")) {
    throw new Error("files with CR line endings are not supported");
  }
  const endsWithNewline = text.endsWith("\n");
  const lines = (endsWithNewline ? text.slice(0, -1) : text).split("\n");
  const sorted = [...insertions].sort((a, b) => a.before - b.before);
  for (const ins of sorted) {
    if (ins.before < 0 || ins.before > lines.length || ins.lines.length === 0) {
      throw new Error(`bad insertion at line ${String(ins.before)}`);
    }
  }
  const out = [`diff --git a/${path} b/${path}`, `--- a/${path}`, `+++ b/${path}`];
  let added = 0;
  for (const group of groupInsertions(sorted)) {
    const first = group[0];
    const last = group.at(-1);
    if (first === undefined || last === undefined) {
      continue;
    }
    const start = Math.max(0, first.before - CONTEXT);
    const end = Math.min(lines.length, last.before + CONTEXT);
    const body: string[] = [];
    let groupAdded = 0;
    let next = 0;
    for (let i = start; i <= end; i += 1) {
      const ins = group[next];
      if (ins !== undefined && ins.before === i) {
        body.push(...ins.lines.map((l) => `+${l}`));
        groupAdded += ins.lines.length;
        next += 1;
      }
      if (i < end) {
        body.push(` ${lines[i] ?? ""}`);
        if (i === lines.length - 1 && !endsWithNewline) {
          body.push("\\ No newline at end of file");
        }
      }
    }
    const oldCount = end - start;
    const oldStart = oldCount === 0 ? start : start + 1;
    out.push(`@@ -${range(oldStart, oldCount)} +${range(start + 1 + added, oldCount + groupAdded)} @@`, ...body);
    added += groupAdded;
  }
  return `${out.join("\n")}\n`;
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
    if (current !== undefined && previous !== undefined && ins.before - previous.before <= 2 * CONTEXT) {
      current.push(ins);
    } else {
      groups.push([ins]);
    }
  }
  return groups;
}
