// The sundgren.containment/0 document (`sundgren contain`) and the containment page's messages.
// Shared by the extension and the webview; no VS Code, no DOM.

export type Relation = "same" | "a-in-b" | "b-in-a" | "overlap" | "unrelated" | "empty";

export interface FileShare {
  readonly path: string;
  readonly lines: number;
  readonly exact: number;
  readonly fuzzy: number;
}

/** Where most of an item's found lines are on the other side. */
export interface Counterpart {
  readonly path: string;
  /** Left out outside items, and in files sundgren doesn't read. */
  readonly item?: string;
  readonly lines: number;
}

/** One item's share, in a file sundgren reads; `item` is left out for lines outside items. */
export interface ItemShare {
  readonly path: string;
  readonly item?: string;
  readonly kind?: string;
  readonly lines: number;
  readonly exact: number;
  readonly fuzzy: number;
  readonly counterpart?: Counterpart;
}

export interface Direction {
  readonly lines: number;
  readonly exact: number;
  readonly fuzzy: number;
  readonly in_order: number;
  /** Left out when the side has no lines, like the two below. */
  readonly containment?: number;
  readonly exact_containment?: number;
  readonly ordered_containment?: number;
  readonly files: readonly FileShare[];
  /** Lines that are only a comment; this and the two below only if sundgren read some files. */
  readonly comment_lines?: number;
  readonly code_containment?: number;
  readonly items?: readonly ItemShare[];
}

export interface Side {
  readonly rev: string;
  readonly oid: string;
  readonly fork: string;
  readonly files: number;
  readonly lines: number;
  /** Files sundgren read (Python, Rust, C). */
  readonly files_read: number;
}

export interface Containment {
  readonly schema: "sundgren.containment/0";
  readonly repository: string;
  readonly base: string;
  readonly base_oid: string;
  readonly threshold: number;
  readonly a: Side;
  readonly b: Side;
  readonly a_in_b: Direction;
  readonly b_in_a: Direction;
  readonly relation: Relation;
}

/** From the extension to the containment page. */
export type ToContainmentPage =
  | {
      readonly type: "start";
      readonly target: string;
      readonly base: string;
      readonly branches: readonly string[];
    }
  | { readonly type: "result"; readonly branch: string; readonly result: Containment }
  | { readonly type: "failed"; readonly branch: string; readonly message: string }
  | { readonly type: "done" };

const RELATIONS = new Set<string>(["same", "a-in-b", "b-in-a", "overlap", "unrelated", "empty"]);

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isRatio(value: unknown): boolean {
  return value === undefined || (typeof value === "number" && value >= 0 && value <= 1);
}

function isFileShare(value: unknown): value is FileShare {
  return (
    isObject(value) &&
    typeof value["path"] === "string" &&
    isCount(value["lines"]) &&
    isCount(value["exact"]) &&
    isCount(value["fuzzy"])
  );
}

function isOptional(value: unknown, isValid: (value: unknown) => boolean): boolean {
  return value === undefined || isValid(value);
}

function isCounterpart(value: unknown): value is Counterpart {
  return (
    isObject(value) &&
    typeof value["path"] === "string" &&
    isOptional(value["item"], (item) => typeof item === "string") &&
    isCount(value["lines"])
  );
}

function isItemShare(value: unknown): value is ItemShare {
  return (
    isObject(value) &&
    typeof value["path"] === "string" &&
    isCount(value["lines"]) &&
    isCount(value["exact"]) &&
    isCount(value["fuzzy"]) &&
    isOptional(value["item"], (item) => typeof item === "string") &&
    isOptional(value["kind"], (kind) => typeof kind === "string") &&
    isOptional(value["counterpart"], isCounterpart)
  );
}

function isDirection(value: unknown): value is Direction {
  return (
    isObject(value) &&
    isCount(value["lines"]) &&
    isCount(value["exact"]) &&
    isCount(value["fuzzy"]) &&
    isCount(value["in_order"]) &&
    isRatio(value["containment"]) &&
    isRatio(value["exact_containment"]) &&
    isRatio(value["ordered_containment"]) &&
    Array.isArray(value["files"]) &&
    value["files"].every((file) => isFileShare(file)) &&
    isOptional(value["comment_lines"], isCount) &&
    isRatio(value["code_containment"]) &&
    isOptional(
      value["items"],
      (items) => Array.isArray(items) && items.every((item) => isItemShare(item)),
    )
  );
}

function isSide(value: unknown): value is Side {
  return (
    isObject(value) &&
    typeof value["rev"] === "string" &&
    typeof value["oid"] === "string" &&
    typeof value["fork"] === "string" &&
    isCount(value["files"]) &&
    isCount(value["lines"]) &&
    isCount(value["files_read"])
  );
}

/** `value` as a containment document, or undefined if it isn't one. */
export function parseContainment(value: unknown): Containment | undefined {
  return isObject(value) &&
    value["schema"] === "sundgren.containment/0" &&
    typeof value["repository"] === "string" &&
    typeof value["base"] === "string" &&
    typeof value["base_oid"] === "string" &&
    typeof value["threshold"] === "number" &&
    isSide(value["a"]) &&
    isSide(value["b"]) &&
    isDirection(value["a_in_b"]) &&
    isDirection(value["b_in_a"]) &&
    typeof value["relation"] === "string" &&
    RELATIONS.has(value["relation"])
    ? (value as unknown as Containment)
    : undefined;
}

/** The median of `values`, or undefined if there are none. */
export function median(values: readonly number[]): number | undefined {
  if (values.length === 0) {
    return undefined;
  }
  const sorted = values.toSorted((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const upper = sorted.at(middle) ?? 0;
  return sorted.length % 2 === 1 ? upper : ((sorted.at(middle - 1) ?? 0) + upper) / 2;
}

/** Order for the page: most contained in the target first; empty and failed ones last. */
export function compareResults(a: Containment | undefined, b: Containment | undefined): number {
  const score = (c: Containment | undefined): number => c?.a_in_b.containment ?? -1;
  return score(b) - score(a);
}
