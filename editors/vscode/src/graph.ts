// The sundgren.graph/1 document (schema/graph.schema.json) and a check that JSON is one.
// Shared by the extension and the webview; no VS Code, no DOM.

export interface Line {
  readonly from: number;
  readonly to: number;
  readonly color: number;
}

export type RefKind = "head" | "branch" | "remote" | "tag" | "other";

export interface Ref {
  readonly name: string;
  readonly kind: RefKind;
}

export interface Commit {
  readonly oid: string;
  readonly parents: readonly string[];
  readonly author: { readonly name: string; readonly email: string };
  /** Unix seconds. */
  readonly authored: number;
  readonly committed: number;
  readonly subject: string;
  readonly refs: readonly Ref[];
  readonly column: number;
  readonly color: number;
  /** Lines from the row above into this row. */
  readonly lines: readonly Line[];
}

export interface Graph {
  readonly schema: "sundgren.graph/1";
  readonly repository: string;
  readonly rev: string;
  readonly oid: string;
  readonly truncated: boolean;
  readonly columns: number;
  readonly commits: readonly Commit[];
  /** Lines below the last row, to parents not listed. */
  readonly tail: readonly Line[];
}

const REF_KINDS = new Set<string>(["head", "branch", "remote", "tag", "other"]);

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isLine(value: unknown): value is Line {
  return (
    isObject(value) && isCount(value["from"]) && isCount(value["to"]) && isCount(value["color"])
  );
}

function isRef(value: unknown): value is Ref {
  return (
    isObject(value) &&
    typeof value["name"] === "string" &&
    typeof value["kind"] === "string" &&
    REF_KINDS.has(value["kind"])
  );
}

function isArrayOf<T>(value: unknown, isItem: (item: unknown) => item is T): value is T[] {
  return Array.isArray(value) && value.every((item) => isItem(item));
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isCommit(value: unknown): value is Commit {
  if (!isObject(value)) {
    return false;
  }
  const author = value["author"];
  return (
    isString(value["oid"]) &&
    isArrayOf(value["parents"], isString) &&
    isObject(author) &&
    isString(author["name"]) &&
    isString(author["email"]) &&
    Number.isSafeInteger(value["authored"]) &&
    Number.isSafeInteger(value["committed"]) &&
    isString(value["subject"]) &&
    isArrayOf(value["refs"], isRef) &&
    isCount(value["column"]) &&
    isCount(value["color"]) &&
    isArrayOf(value["lines"], isLine)
  );
}

/** `value` as a graph document, or undefined if it isn't one. */
export function parseGraph(value: unknown): Graph | undefined {
  return isObject(value) &&
    value["schema"] === "sundgren.graph/1" &&
    isString(value["repository"]) &&
    isString(value["rev"]) &&
    isString(value["oid"]) &&
    typeof value["truncated"] === "boolean" &&
    isCount(value["columns"]) &&
    isArrayOf(value["commits"], isCommit) &&
    isArrayOf(value["tail"], isLine)
    ? (value as unknown as Graph)
    : undefined;
}
