// The sundgren.graph/3 document (schema/graph.schema.json) and a check that JSON is one.
// Shared by the extension and the webview; no VS Code, no DOM. The schema's nulls become
// missing optional fields here.

/** A ref as given (`main`, `origin/main`) and the commit it points at. */
export interface Named {
  readonly name: string;
  readonly oid: string;
}

export type ShowMode = "local" | "recent" | "mine" | "like";

/** How the branches besides the base were chosen; `like` only in mode `like`. */
export interface Show {
  readonly mode: ShowMode;
  readonly like?: Named;
}

/** How much history is drawn, and whether there is older history. */
export interface Page {
  readonly commits: number;
  readonly more: boolean;
}

export type LaneKind = "reference" | "local" | "remote";

export interface Lane {
  readonly name: string;
  /** The branch both twins belong to: `feat-b` for `origin/feat-b`. */
  readonly branch: string;
  readonly kind: LaneKind;
  readonly column: number;
  /** Grid row of its label. */
  readonly label_row: number;
  /** Colour family; missing for protected lanes. */
  readonly family?: number;
  /** Index of the other lane of the same branch, if drawn. */
  readonly twin?: number;
  /** The remote ref at the same commit (in sync): it has no lane of its own. */
  readonly remote?: string;
  readonly protected: boolean;
  readonly pinned: boolean;
  /** Nothing ahead of the base: drawn faded. */
  readonly merged: boolean;
  /** No twin at all. */
  readonly only?: "local" | "remote";
  readonly ahead: number;
  readonly behind: number;
  /** Mode `like`: how much of its work is in the `like` branch, 0 to 1. */
  readonly score?: number;
  /** The commit it leaves from; missing for the reference. */
  readonly fork?: string;
  readonly tip: string;
  /** It goes on below the page: how many of its own commits are not drawn. */
  readonly before?: number;
}

export interface Commit {
  readonly oid: string;
  readonly parents: readonly string[];
  readonly author: string;
  /** Unix seconds. */
  readonly committed: number;
  readonly subject: string;
  /** The message after the subject; "" if none. */
  readonly body: string;
  readonly refs: readonly string[];
  readonly row: number;
  readonly lanes: readonly number[];
}

/** A run of commits on one lane that can be drawn as one row. */
export interface Fold {
  readonly lane: number;
  /** First and last grid row, inclusive. */
  readonly rows: readonly [number, number];
  /** Indices into commits, newest first. */
  readonly commits: readonly number[];
}

/** Lines leave the centre towards these sides. */
export const UP = 1;
export const RIGHT = 2;
export const DOWN = 4;
export const LEFT = 8;

export interface Cell {
  readonly row: number;
  readonly col: number;
  /** UP | RIGHT | DOWN | LEFT. */
  readonly sides: number;
  readonly lane: number;
  readonly node?: "commit" | "connector";
  /** Index into commits, for a commit node. */
  readonly commit?: number;
}

export interface Grid {
  readonly width: number;
  readonly height: number;
  readonly cells: readonly Cell[];
}

export interface Graph {
  readonly schema: "sundgren.graph/3";
  readonly repository: string;
  readonly base: Named;
  readonly show: Show;
  readonly threshold: number;
  readonly page: Page;
  /** Branches that can be shown besides the base and the `like` one, in `show` order. */
  readonly branches: readonly string[];
  readonly lanes: readonly Lane[];
  /** Newest first (by row). */
  readonly commits: readonly Commit[];
  readonly folds: readonly Fold[];
  readonly grid: Grid;
}

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isNumber(value: unknown): value is number {
  return typeof value === "number";
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isOid(value: unknown): value is string {
  return (
    typeof value === "string" &&
    (value.length === 40 || value.length === 64) &&
    /^[\da-f]+$/u.test(value)
  );
}

function isArrayOf<T>(value: unknown, isItem: (item: unknown) => item is T): value is T[] {
  return Array.isArray(value) && value.every((item) => isItem(item));
}

/** A value that may be null (or left out): undefined then, else `value` if `isItem` holds. */
class Optional<T> {
  readonly isValid: boolean;
  readonly value: T | undefined;

  constructor(value: unknown, isItem: (item: unknown) => item is T) {
    const isMissing = value === null || value === undefined;
    this.isValid = isMissing || isItem(value);
    this.value = !isMissing && isItem(value) ? value : undefined;
  }
}

/** `{ [key]: value }`, or nothing if `value` is undefined. */
function field<K extends string, T>(key: K, value: T | undefined): Partial<Record<K, T>> {
  return value === undefined ? {} : ({ [key]: value } as Record<K, T>);
}

function parseNamed(value: unknown): Named | undefined {
  return isObject(value) && isString(value["name"]) && isOid(value["oid"])
    ? { name: value["name"], oid: value["oid"] }
    : undefined;
}

function isNamed(value: unknown): value is Named {
  return parseNamed(value) !== undefined;
}

const SHOW_MODES = new Set<unknown>(["local", "recent", "mine", "like"]);
const LANE_KINDS = new Set<unknown>(["reference", "local", "remote"]);
const SIDES_ONLY = new Set<unknown>(["local", "remote"]);
const NODES = new Set<unknown>(["commit", "connector"]);

export function isShowMode(value: unknown): value is ShowMode {
  return SHOW_MODES.has(value);
}

function isLaneKind(value: unknown): value is LaneKind {
  return LANE_KINDS.has(value);
}

function isOnly(value: unknown): value is "local" | "remote" {
  return SIDES_ONLY.has(value);
}

function isNode(value: unknown): value is "commit" | "connector" {
  return NODES.has(value);
}

function parseShow(value: unknown): Show | undefined {
  if (!isObject(value) || !isShowMode(value["mode"])) {
    return undefined;
  }
  const like = new Optional(value["like"], isNamed);
  const named = like.value === undefined ? undefined : parseNamed(like.value);
  return like.isValid ? { mode: value["mode"], ...field("like", named) } : undefined;
}

function parsePage(value: unknown): Page | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { commits, more } = value;
  return typeof more === "boolean" && isCount(commits) && commits >= 1
    ? { commits, more }
    : undefined;
}

type LaneOptions = Pick<Lane, "family" | "twin" | "remote" | "only" | "score" | "fork" | "before">;

/** The lane's required fields, if they are all there. */
function laneBasics(value: Fields): Omit<Lane, keyof LaneOptions> | undefined {
  const { name, branch, kind, column, label_row, ahead, behind, tip } = value;
  const flags = [value["protected"], value["pinned"], value["merged"]];
  if (
    !isString(name) ||
    !isString(branch) ||
    !isLaneKind(kind) ||
    !isCount(column) ||
    !isCount(label_row) ||
    !isCount(ahead) ||
    !isCount(behind) ||
    !isOid(tip) ||
    flags.some((flag) => typeof flag !== "boolean")
  ) {
    return undefined;
  }
  return {
    name,
    branch,
    kind,
    column,
    label_row,
    protected: value["protected"] === true,
    pinned: value["pinned"] === true,
    merged: value["merged"] === true,
    ahead,
    behind,
    tip,
  };
}

function parseLane(value: unknown): Lane | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const basics = laneBasics(value);
  const family = new Optional(value["family"], isCount);
  const twin = new Optional(value["twin"], isCount);
  const remote = new Optional(value["remote"], isString);
  const only = new Optional(value["only"], isOnly);
  const score = new Optional(value["score"], isNumber);
  const fork = new Optional(value["fork"], isOid);
  const before = new Optional(value["before"], isCount);
  const options = [family, twin, remote, only, score, fork, before];
  if (basics === undefined || options.some((option) => !option.isValid)) {
    return undefined;
  }
  return {
    ...basics,
    ...field("family", family.value),
    ...field("twin", twin.value),
    ...field("remote", remote.value),
    ...field("only", only.value),
    ...field("score", score.value),
    ...field("fork", fork.value),
    ...field("before", before.value),
  };
}

function parseCommit(value: unknown): Commit | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { oid, parents, author, committed, subject, body, refs, row, lanes } = value;
  return isOid(oid) &&
    isArrayOf(parents, isOid) &&
    isString(author) &&
    isNumber(committed) &&
    Number.isSafeInteger(committed) &&
    isString(subject) &&
    isString(body) &&
    isArrayOf(refs, isString) &&
    isCount(row) &&
    isArrayOf(lanes, isCount)
    ? { oid, parents, author, committed, subject, body, refs, row, lanes }
    : undefined;
}

function parseFold(value: unknown): Fold | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { lane, rows, commits } = value;
  if (!isCount(lane) || !isArrayOf(rows, isCount) || !isArrayOf(commits, isCount)) {
    return undefined;
  }
  const [first, last] = rows;
  return first !== undefined && last !== undefined && rows.length === 2 && first <= last
    ? { lane, rows: [first, last], commits }
    : undefined;
}

function parseCell(value: unknown): Cell | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { row, col, sides, lane } = value;
  const node = new Optional(value["node"], isNode);
  const commit = new Optional(value["commit"], isCount);
  return isCount(row) &&
    isCount(col) &&
    isCount(sides) &&
    sides <= 15 &&
    isCount(lane) &&
    node.isValid &&
    commit.isValid
    ? { row, col, sides, lane, ...field("node", node.value), ...field("commit", commit.value) }
    : undefined;
}

/** Each item of `value` parsed, or undefined if it isn't an array or an item doesn't parse. */
function parseAll<T>(value: unknown, parse: (item: unknown) => T | undefined): T[] | undefined {
  if (!Array.isArray(value)) {
    return undefined;
  }
  const items: T[] = [];
  for (const item of value) {
    const parsed = parse(item);
    if (parsed === undefined) {
      return undefined;
    }
    items.push(parsed);
  }
  return items;
}

function parseGrid(value: unknown): Grid | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { width, height } = value;
  const cells = parseAll(value["cells"], parseCell);
  return cells !== undefined && isCount(width) && isCount(height)
    ? { width, height, cells }
    : undefined;
}

/** Whether every index in `graph` points at something that is there. */
function isConsistent(graph: Graph): boolean {
  const { lanes, commits, folds, grid } = graph;
  const isLaneIndex = (index: number): boolean => index < lanes.length;
  const isCommitIndex = (index: number): boolean => index < commits.length;
  return (
    lanes.every(
      (lane) =>
        lane.column < grid.width &&
        lane.label_row < grid.height &&
        (lane.twin === undefined || isLaneIndex(lane.twin)),
    ) &&
    commits.every(
      (commit) => commit.row < grid.height && commit.lanes.every((lane) => isLaneIndex(lane)),
    ) &&
    folds.every(
      (fold) =>
        isLaneIndex(fold.lane) &&
        fold.rows[1] < grid.height &&
        fold.commits.every((commit) => isCommitIndex(commit)),
    ) &&
    grid.cells.every(
      (cell) =>
        cell.row < grid.height &&
        cell.col < grid.width &&
        isLaneIndex(cell.lane) &&
        (cell.commit === undefined || isCommitIndex(cell.commit)),
    )
  );
}

/** `value` as a graph document, or undefined if it isn't one. */
export function parseGraph(value: unknown): Graph | undefined {
  if (!isObject(value) || value["schema"] !== "sundgren.graph/3") {
    return undefined;
  }
  const { repository, threshold, branches } = value;
  const base = parseNamed(value["base"]);
  const show = parseShow(value["show"]);
  const page = parsePage(value["page"]);
  const lanes = parseAll(value["lanes"], parseLane);
  const commits = parseAll(value["commits"], parseCommit);
  const folds = parseAll(value["folds"], parseFold);
  const grid = parseGrid(value["grid"]);
  if (
    base === undefined ||
    show === undefined ||
    page === undefined ||
    lanes === undefined ||
    commits === undefined ||
    folds === undefined ||
    grid === undefined ||
    !isString(repository) ||
    !isNumber(threshold) ||
    !isArrayOf(branches, isString)
  ) {
    return undefined;
  }
  const graph: Graph = {
    schema: "sundgren.graph/3",
    repository,
    base,
    show,
    threshold,
    page,
    branches,
    lanes,
    commits,
    folds,
    grid,
  };
  return isConsistent(graph) ? graph : undefined;
}
