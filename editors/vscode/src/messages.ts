// Messages between the extension and its pages. No VS Code, no DOM.

import type { Meta } from "./containment.js";
import type { BranchState } from "./git.js";
import { isShowMode, type Graph, type ShowMode } from "./graph.js";

/** From the extension to the history page. */
export type ToPage =
  | {
      /** A new map: what it is of, before the first graph. */
      readonly type: "setup";
      /** The repository's path; the docs demo has none. */
      readonly repository?: string;
      readonly base: string;
      /** The view shown first, or last when the page reloads. */
      readonly view: View;
      /** How many more base commits "Load older" asks for. */
      readonly step: number;
      readonly states: readonly BranchState[];
      readonly meta?: Meta;
    }
  /** `sundgren graph` is running for `view`; `progress` is the last line it printed on stderr. */
  | { readonly type: "running"; readonly view?: View; readonly progress?: string }
  | { readonly type: "graph"; readonly graph: Graph }
  | { readonly type: "error"; readonly message: string };

/**
What the history map shows: branches chosen by `show` (`like` names the branch of mode
`like`), the first `branches` of them and `pins`, less `hides`, over `commits` first-parent
commits of the base.
*/
export interface View {
  readonly show: ShowMode;
  readonly like?: string;
  readonly branches: number;
  readonly commits: number;
  /** Branches ticked on in the page's branch list. */
  readonly pins: readonly string[];
  /** Branches ticked off. */
  readonly hides: readonly string[];
  /** The page's bounds, unix seconds: from `since` (instead of `commits` back) to `until`. */
  readonly since?: number;
  readonly until?: number;
}

/** Branches shown besides the base, at most. */
export const MAX_BRANCHES = 30;

/** Something clicked on the history map. */
export type Selection =
  | { readonly kind: "commit"; readonly oid: string }
  | { readonly kind: "lane"; readonly name: string };

/** A control in a page's toolbar: what the user wants to change; `like` picks a branch to rank by. */
export type Control = "repository" | "base" | "target" | "comments" | "like";

const CONTROLS = new Set<string>(["repository", "base", "target", "comments", "like"]);

/** A file at a commit: `oid` is a commit id in hex, `path` relative to the repository. */
export interface Revision {
  readonly oid: string;
  readonly path: string;
}

/**
From a page to the extension: it has loaded; it has drawn the graph of `oid`; the user asked to see
`line` of the file (the outline page); on the containment and history pages, to change `control`;
on the containment page, to inspect `branch` (at `path` and `line`) or see the history with it; on
the inspect page, to open a file at a commit; on the history page, another view, or a click.
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
  | { readonly type: "open"; readonly revision: Revision; readonly line?: number }
  | { readonly type: "history"; readonly branch?: string }
  | ({ readonly type: "view" } & View)
  | { readonly type: "select"; readonly selection: Selection };

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

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

/** A branch name as the page sends it: not empty, and not something git or sundgren reads as an option. */
function isBranchName(value: unknown): value is string {
  return (
    typeof value === "string" && value !== "" && !value.startsWith("-") && !value.includes("\0")
  );
}

function isTimeOrMissing(value: unknown): value is number | undefined {
  return value === undefined || (typeof value === "number" && Number.isSafeInteger(value));
}

function isBranchNames(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((name) => isBranchName(name));
}

function parseView(value: Fields): FromPage | undefined {
  const { show, like, branches, commits, pins, hides, since, until } = value;
  if (
    !isShowMode(show) ||
    !isCount(branches) ||
    branches > MAX_BRANCHES ||
    !isCount(commits) ||
    commits < 1 ||
    !isBranchNames(pins) ||
    !isBranchNames(hides) ||
    !isTimeOrMissing(since) ||
    !isTimeOrMissing(until)
  ) {
    return undefined;
  }
  const view = {
    type: "view",
    show,
    branches,
    commits,
    pins,
    hides,
    ...(since !== undefined && { since }),
    ...(until !== undefined && { until }),
  } as const;
  if (like === undefined) {
    return show === "like" ? undefined : view;
  }
  return isBranchName(like) ? { ...view, like } : undefined;
}

function parseSelection(value: unknown): Selection | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { kind, oid, name } = value;
  if (kind === "commit") {
    return isOid(oid) ? { kind, oid } : undefined;
  }
  return kind === "lane" && typeof name === "string" && name !== "" ? { kind, name } : undefined;
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

/** The messages about the history page: open it, another view, a click. */
function parseHistoryMessage(value: Fields): FromPage | undefined {
  switch (value["type"]) {
    case "history": {
      const { branch } = value;
      if (branch === undefined) {
        return { type: "history" };
      }
      return typeof branch === "string" && branch !== "" ? { type: "history", branch } : undefined;
    }
    case "view": {
      return parseView(value);
    }
    case "select": {
      const selection = parseSelection(value["selection"]);
      return selection === undefined ? undefined : { type: "select", selection };
    }
    default: {
      return undefined;
    }
  }
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
      return parseHistoryMessage(value);
    }
  }
}
