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

/**
From a page to the extension: it has loaded; it has drawn the graph of `oid`; the user asked to see
`line` of the file (the outline page); on the containment page, to change `control` or to inspect
`branch` (at `path` and `line`); on the inspect page, to open a file at a commit.
*/
export type FromPage =
  | { readonly type: "ready" }
  | { readonly type: "rendered"; readonly oid: string; readonly rows: number }
  | { readonly type: "reveal"; readonly line: number }
  | { readonly type: "choose"; readonly control: Control }
  | {
      readonly type: "inspect";
      readonly branch: string;
      readonly path?: string;
      readonly line?: number;
    }
  | { readonly type: "open"; readonly revision: Revision; readonly line?: number };

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

function parseInspect(value: Fields): FromPage | undefined {
  const { branch, path, line } = value;
  if (typeof branch !== "string" || branch === "") {
    return undefined;
  }
  if (path !== undefined && !isPath(path)) {
    return undefined;
  }
  if (line !== undefined && !isLine(line)) {
    return undefined;
  }
  return {
    type: "inspect",
    branch,
    ...(path !== undefined && { path }),
    ...(line !== undefined && { line }),
  };
}

function parseOpen(value: Fields): FromPage | undefined {
  const { revision, line } = value;
  if (!isRevision(revision)) {
    return undefined;
  }
  if (line === undefined) {
    return { type: "open", revision };
  }
  return isLine(line) ? { type: "open", revision, line } : undefined;
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
    case "inspect": {
      return parseInspect(value);
    }
    case "open": {
      return parseOpen(value);
    }
    default: {
      return undefined;
    }
  }
}
