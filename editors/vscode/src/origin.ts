// The sundgren.origin/0 document (`sundgren origin`) and the origin page's messages. Shared by the
// extension and the webview; no VS Code, no DOM.

import {
  isLineFound,
  type Comments,
  type Containment,
  type LineFound,
  type Meta,
  type Side,
} from "./containment.js";

/** How much of a file is found in one file of a reference. */
export interface Source {
  readonly path: string;
  readonly exact: number;
  readonly fuzzy: number;
  /** (exact + fuzzy) / the file's lines. */
  readonly share: number;
  /** Lines found exactly here that are in at most two files of the reference, over the lines. */
  readonly distinctive: number;
}

/** Where a file's lines come from at one reference; no `best` if no file shares a line. */
export interface Origin {
  readonly rev: string;
  readonly oid: string;
  readonly best?: Source;
  readonly second?: Source;
  readonly gap?: number;
  /** With `--lines`: every line of the file and how it was found in the best file. */
  readonly lines_found?: readonly LineFound[];
}

export interface FileOrigin {
  readonly path: string;
  readonly lines: number;
  readonly comment_lines: number;
  /** One per reference, in the order given. */
  readonly origins: readonly Origin[];
}

export interface OriginReference {
  readonly rev: string;
  readonly oid: string;
  readonly files: number;
}

export interface OriginDocument {
  readonly schema: "sundgren.origin/0";
  readonly repository: string;
  readonly other: string;
  readonly threshold: number;
  readonly comments: Comments;
  readonly references: readonly OriginReference[];
  readonly files: readonly FileOrigin[];
}

/** From the extension to the origin page. */
export type ToOriginPage =
  | {
      readonly type: "start";
      /** The file, relative to its repository. */
      readonly path: string;
      /** The other repository's folder name. */
      readonly other: string;
      readonly refs: readonly string[];
      /** How similar an edited line must be, 0 to 1. */
      readonly threshold: number;
      readonly meta?: Meta;
    }
  | { readonly type: "result"; readonly file: FileOrigin }
  | { readonly type: "failed"; readonly message: string };

/** The commit id the page and the inspect panel use for the work tree: git's null id. */
export const WORK_TREE = "0".repeat(40);

/** A lead at least this big over the second file is clear: `sundgren contain`'s LOW. */
export const CLEAR_LEAD = 0.2;

/** How clear an origin is: a clear lead, a close call between files, or no file at all. */
export function verdict(origin: Origin): "clear" | "close" | "none" {
  if (origin.best === undefined) {
    return "none";
  }
  return (origin.gap ?? 0) >= CLEAR_LEAD ? "clear" : "close";
}

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isShare(value: unknown): value is number {
  return typeof value === "number" && value >= 0 && value <= 1;
}

function isSource(value: unknown): value is Source {
  return (
    isObject(value) &&
    typeof value["path"] === "string" &&
    isCount(value["exact"]) &&
    isCount(value["fuzzy"]) &&
    isShare(value["share"]) &&
    isShare(value["distinctive"])
  );
}

function isOrigin(value: unknown): value is Origin {
  if (!isObject(value) || typeof value["rev"] !== "string" || typeof value["oid"] !== "string") {
    return false;
  }
  const { best, second, gap, lines_found: lines } = value;
  return (
    (best === undefined || isSource(best)) &&
    (second === undefined || isSource(second)) &&
    (gap === undefined || isShare(gap)) &&
    (lines === undefined || (Array.isArray(lines) && lines.every((line) => isLineFound(line))))
  );
}

function isFileOrigin(value: unknown): value is FileOrigin {
  return (
    isObject(value) &&
    typeof value["path"] === "string" &&
    isCount(value["lines"]) &&
    isCount(value["comment_lines"]) &&
    Array.isArray(value["origins"]) &&
    value["origins"].every((origin) => isOrigin(origin))
  );
}

function isReference(value: unknown): value is OriginReference {
  return (
    isObject(value) &&
    typeof value["rev"] === "string" &&
    typeof value["oid"] === "string" &&
    isCount(value["files"])
  );
}

/** `value` as an origin document, or undefined if it isn't one. */
export function parseOrigin(value: unknown): OriginDocument | undefined {
  return isObject(value) &&
    value["schema"] === "sundgren.origin/0" &&
    typeof value["repository"] === "string" &&
    typeof value["other"] === "string" &&
    typeof value["threshold"] === "number" &&
    (value["comments"] === "ignored" || value["comments"] === "counted") &&
    Array.isArray(value["references"]) &&
    value["references"].every((reference) => isReference(reference)) &&
    Array.isArray(value["files"]) &&
    value["files"].every((file) => isFileOrigin(file))
    ? (value as unknown as OriginDocument)
    : undefined;
}

/** One file at `oid` as a containment side: no commits, no base. */
function side(rev: string, oid: string, lines: number, comments: number): Side {
  return {
    rev,
    oid,
    fork: oid,
    commits: 0,
    files: 1,
    lines,
    comment_lines: comments,
    files_read: 0,
  };
}

/**
`file`'s origin as the inspect page reads a containment result: the file (at WORK_TREE) is A, the
best file at the reference is B, and A's lines are looked up in B. Undefined without a best file
or without its lines.
*/
export function inspection(
  document: OriginDocument,
  file: FileOrigin,
  origin: Origin,
): Containment | undefined {
  const { best, lines_found: lines } = origin;
  if (best === undefined || lines === undefined) {
    return undefined;
  }
  const share = { lines: file.lines, exact: best.exact, fuzzy: best.fuzzy };
  return {
    schema: "sundgren.containment/0",
    repository: document.repository,
    base: "",
    base_oid: "",
    threshold: document.threshold,
    comments: document.comments,
    a: side(file.path, WORK_TREE, file.lines, file.comment_lines),
    b: side(origin.rev, origin.oid, 0, 0),
    a_in_b: {
      ...share,
      in_order: 0,
      containment: best.share,
      files: [{ path: file.path, ...share }],
      lines_found: lines,
    },
    // One way only: the inspect page doesn't show B's lines looked up in A.
    b_in_a: { lines: 0, exact: 0, fuzzy: 0, in_order: 0, files: [] },
    // Not shown on the inspect page.
    relation: "a-in-b",
  };
}
