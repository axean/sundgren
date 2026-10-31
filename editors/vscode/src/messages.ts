// Messages between the extension and its pages. No VS Code, no DOM.

import type { Graph } from "./graph.js";

/** From the extension to the history page. */
export type ToPage =
  | { readonly type: "graph"; readonly graph: Graph }
  | { readonly type: "error"; readonly message: string };

/** A control in a page's toolbar: what the user wants to change. */
export type Control = "base" | "target" | "comments";

const CONTROLS = new Set<string>(["base", "target", "comments"]);

/** A file at a commit: `oid` is a commit id in hex, `path` relative to the repository. */
export interface Revision {
  readonly oid: string;
  readonly path: string;
}

/** A file a branch changed: `path` from commit `from` to commit `to`. */
export interface Change {
  readonly path: string;
  readonly from: string;
  readonly to: string;
}

/**
From a page to the extension: it has loaded; it has drawn the graph of `oid`; the user asked to see
`line` of the file (the outline page); or, on the containment page, to change `control`, to see
two revisions side by side (at `line` of the right one), or to see a branch's changes.
*/
export type FromPage =
  | { readonly type: "ready" }
  | { readonly type: "rendered"; readonly oid: string; readonly rows: number }
  | { readonly type: "reveal"; readonly line: number }
  | { readonly type: "choose"; readonly control: Control }
  | {
      readonly type: "diff";
      readonly left: Revision;
      readonly right: Revision;
      readonly title: string;
      readonly line?: number;
    }
  | { readonly type: "changes"; readonly title: string; readonly changes: readonly Change[] };

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isLine(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 1;
}

/** A commit id: hex only, so it can't be taken for an option or another kind of revision. */
function isOid(value: unknown): value is string {
  return typeof value === "string" && /^[\da-f]{4,64}$/u.test(value);
}

function isPath(value: unknown): value is string {
  return typeof value === "string" && value !== "" && !value.includes("\0");
}

function isRevision(value: unknown): value is Revision {
  return isObject(value) && isOid(value["oid"]) && isPath(value["path"]);
}

function isChange(value: unknown): value is Change {
  return isObject(value) && isPath(value["path"]) && isOid(value["from"]) && isOid(value["to"]);
}

function parseDiff(value: Fields): FromPage | undefined {
  const { left, right, title, line } = value;
  if (typeof title !== "string" || !isRevision(left) || !isRevision(right)) {
    return undefined;
  }
  if (line === undefined) {
    return { type: "diff", left, right, title };
  }
  return isLine(line) ? { type: "diff", left, right, title, line } : undefined;
}

function parseChanges(value: Fields): FromPage | undefined {
  const { title, changes } = value;
  return typeof title === "string" &&
    Array.isArray(changes) &&
    changes.every((change) => isChange(change))
    ? { type: "changes", title, changes }
    : undefined;
}

/** `value` as a message from a page, or undefined if it isn't one. */
export function parseFromPage(value: unknown): FromPage | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  switch (value["type"]) {
    case "ready": {
      return { type: "ready" };
    }
    case "rendered": {
      const { oid, rows } = value;
      return typeof oid === "string" && typeof rows === "number"
        ? { type: "rendered", oid, rows }
        : undefined;
    }
    case "reveal": {
      const { line } = value;
      return isLine(line) ? { type: "reveal", line } : undefined;
    }
    case "choose": {
      const { control } = value;
      return typeof control === "string" && CONTROLS.has(control)
        ? { type: "choose", control: control as Control }
        : undefined;
    }
    case "diff": {
      return parseDiff(value);
    }
    case "changes": {
      return parseChanges(value);
    }
    default: {
      return undefined;
    }
  }
}
