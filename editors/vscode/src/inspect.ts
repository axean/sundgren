// The inspect page's messages and grouping: one branch's lines next to where they were found in
// the target. Shared by the extension and the webview; no VS Code, no DOM.

import type { Comments, Containment, LineFound, Meta } from "./containment.js";

/** From the extension to the inspect page. */
export type ToInspectPage =
  | {
      readonly type: "inspect";
      readonly branch: string;
      readonly target: string;
      readonly base: string;
      readonly comments: Comments;
      readonly threshold: number;
      /** `sundgren contain --lines branch target`. */
      readonly result: Containment;
      /** Where to scroll to: a file, and a line in it. */
      readonly focus?: { readonly path: string; readonly line?: number };
      readonly meta?: Meta;
    }
  | { readonly type: "error"; readonly branch: string; readonly message: string };

/** Lines of one item (or outside items: no `item`), in file order. */
export interface ItemGroup {
  readonly item: string | undefined;
  readonly lines: readonly LineFound[];
}

/** One file's lines, by item in order of first line. */
export interface FileGroup {
  readonly path: string;
  readonly items: readonly ItemGroup[];
}

/** `lines` (in file order, as `--lines` gives them) by file, then by item. */
export function groupLines(lines: readonly LineFound[]): FileGroup[] {
  const files: { path: string; items: Map<string, LineFound[]> }[] = [];
  for (const line of lines) {
    let file = files.at(-1);
    if (file?.path !== line.path) {
      file = { path: line.path, items: new Map() };
      files.push(file);
    }
    // "" for lines outside items: item names are never empty.
    const key = line.item ?? "";
    const group = file.items.get(key);
    if (group === undefined) {
      file.items.set(key, [line]);
    } else {
      group.push(line);
    }
  }
  return files.map((file) => ({
    path: file.path,
    items: [...file.items].map(([key, grouped]) => ({
      item: key === "" ? undefined : key,
      lines: grouped,
    })),
  }));
}

/** The text of each line of `lines`, by `path:line`: to show the line a match points at. */
export function textAt(lines: readonly LineFound[]): Map<string, string> {
  return new Map(lines.map((line) => [`${line.path}:${String(line.line)}`, line.text]));
}

/** Whether a line is worth showing by default: edited, or not found. */
export function isDifferent(line: LineFound): boolean {
  return line.found === "fuzzy" || line.found === "missing";
}

/** A stretch of an item's lines as shown: one line, or a run of lines folded away. */
export type Segment =
  | { readonly kind: "line"; readonly line: LineFound }
  | { readonly kind: "fold"; readonly lines: readonly LineFound[] };

/**
`lines` as shown: with `isFolding`, each run of lines that are the same (exact) or left out
(comments) is one fold; without, every line on its own.
*/
export function segments(lines: readonly LineFound[], isFolding: boolean): Segment[] {
  if (!isFolding) {
    return lines.map((line) => ({ kind: "line", line }));
  }
  const out: Segment[] = [];
  let run: LineFound[] = [];
  for (const line of lines) {
    if (isDifferent(line)) {
      if (run.length > 0) {
        out.push({ kind: "fold", lines: run });
        run = [];
      }
      out.push({ kind: "line", line });
    } else {
      run.push(line);
    }
  }
  if (run.length > 0) {
    out.push({ kind: "fold", lines: run });
  }
  return out;
}

/** How a set of lines was found, counted. */
export interface Tally {
  readonly exact: number;
  readonly fuzzy: number;
  readonly missing: number;
  readonly comment: number;
}

export function tally(lines: readonly LineFound[]): Tally {
  const count = (found: LineFound["found"]): number =>
    lines.filter((line) => line.found === found).length;
  return {
    exact: count("exact"),
    fuzzy: count("fuzzy"),
    missing: count("missing"),
    comment: count("comment"),
  };
}

/** Longer lines than this are marked between their common start and end only: no full diff. */
const DIFF_LIMIT = 500;

const SEGMENTER = new Intl.Segmenter();

/** The characters of `text` as a reader sees them (grapheme clusters). */
export function characters(text: string): string[] {
  return Array.from(SEGMENTER.segment(text), (part) => part.segment);
}

/** For two long lines: everything between their common start and their common end. */
function middle(a: readonly string[], b: readonly string[]): boolean[] {
  let start = 0;
  while (start < a.length && start < b.length && a.at(start) === b.at(start)) {
    start += 1;
  }
  let end = 0;
  while (
    end < a.length - start &&
    end < b.length - start &&
    a.at(a.length - 1 - end) === b.at(b.length - 1 - end)
  ) {
    end += 1;
  }
  return a.map((_, index) => index >= start && index < a.length - end);
}

/** lengths[row][column]: the longest common subsequence of a[row..] and b[column..]. */
function lcsTable(a: readonly string[], b: readonly string[]): number[][] {
  const rows: number[][] = [Array.from({ length: b.length + 1 }, () => 0)];
  for (let row = a.length - 1; row >= 0; row -= 1) {
    const below = rows.at(-1) ?? [];
    const values: number[] = [0];
    let next = 0;
    for (let column = b.length - 1; column >= 0; column -= 1) {
      next =
        a.at(row) === b.at(column)
          ? (below.at(column + 1) ?? 0) + 1
          : Math.max(below.at(column) ?? 0, next);
      values.push(next);
    }
    rows.push(values.toReversed());
  }
  return rows.toReversed();
}

/**
Which characters of `text` are not in `other`: those outside a longest common subsequence of the
two. For very long lines, everything between the common start and the common end.
*/
export function changed(text: string, other: string): boolean[] {
  const a = characters(text);
  const b = characters(other);
  if (a.length * b.length > DIFF_LIMIT * DIFF_LIMIT) {
    return middle(a, b);
  }
  const lengths = lcsTable(a, b);
  const marks: boolean[] = [];
  let here = 0;
  let there = 0;
  while (here < a.length && there < b.length) {
    if (a.at(here) === b.at(there)) {
      marks.push(false);
      here += 1;
      there += 1;
    } else if ((lengths.at(here + 1)?.at(there) ?? 0) >= (lengths.at(here)?.at(there + 1) ?? 0)) {
      marks.push(true);
      here += 1;
    } else {
      there += 1;
    }
  }
  while (marks.length < a.length) {
    marks.push(true);
  }
  return marks;
}
