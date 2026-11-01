// The sundgren.graph/2 document (schema/graph.schema.json) and a check that JSON is one.
// Shared by the extension and the webview; no VS Code, no DOM. The schema's nulls become
// missing optional fields here.

/** A ref as given (`main`, `origin/main`) and the commit it points at. */
export interface Named {
  readonly name: string;
  readonly oid: string;
}

/** Which stretch of time is drawn; `older` and `newer` are the offsets of the next frames. */
export interface Frame {
  readonly offset: number;
  /** Missing at the start of history. */
  readonly older?: number;
  /** Missing at the tip. */
  readonly newer?: number;
  readonly columns: number;
}

export type LaneKind = "reference" | "local" | "remote";

export interface Lane {
  readonly name: string;
  /** The branch both twins belong to: `feat-b` for `origin/feat-b`. */
  readonly branch: string;
  readonly kind: LaneKind;
  readonly row: number;
  /** Colour family; missing for protected lanes. */
  readonly family?: number;
  /** Index of the other lane of the same branch, if drawn. */
  readonly twin?: number;
  readonly protected: boolean;
  readonly pinned: boolean;
  /** Nothing ahead of the base: drawn faded. */
  readonly merged: boolean;
  /** No twin at all: `remote` is drawn dashed. */
  readonly only?: "local" | "remote";
  readonly ahead: number;
  readonly behind: number;
  /** How much of its work is in the target, 0 to 1. */
  readonly score?: number;
  /** The commit it leaves from; missing for the reference. */
  readonly fork?: string;
  readonly tip: string;
  /** It forked before the frame: how many of its own commits are earlier. */
  readonly before?: number;
  /** It goes on right of the frame. */
  readonly after: boolean;
}

export interface Commit {
  readonly oid: string;
  readonly parents: readonly string[];
  readonly author: string;
  /** Unix seconds. */
  readonly committed: number;
  readonly subject: string;
  readonly refs: readonly string[];
  readonly column: number;
  readonly lanes: readonly number[];
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
  readonly schema: "sundgren.graph/2";
  readonly repository: string;
  readonly base: Named;
  /** Missing when none was given. */
  readonly target?: Named;
  readonly threshold: number;
  readonly frame: Frame;
  readonly candidates: number;
  readonly lanes: readonly Lane[];
  /** Oldest first. */
  readonly commits: readonly Commit[];
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

function parseFrame(value: unknown): Frame | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { offset, columns } = value;
  const older = new Optional(value["older"], isCount);
  const newer = new Optional(value["newer"], isCount);
  return !isCount(offset) || !isCount(columns) || columns < 1 || !older.isValid || !newer.isValid
    ? undefined
    : { offset, columns, ...field("older", older.value), ...field("newer", newer.value) };
}

const LANE_KINDS = new Set<unknown>(["reference", "local", "remote"]);
const SIDES_ONLY = new Set<unknown>(["local", "remote"]);
const NODES = new Set<unknown>(["commit", "connector"]);

function isLaneKind(value: unknown): value is LaneKind {
  return LANE_KINDS.has(value);
}

function isOnly(value: unknown): value is "local" | "remote" {
  return SIDES_ONLY.has(value);
}

function isNode(value: unknown): value is "commit" | "connector" {
  return NODES.has(value);
}

/** The lane's required fields, if they are all there. */
function laneBasics(value: Fields): Omit<Lane, keyof LaneOptions> | undefined {
  const { name, branch, kind, row, ahead, behind, tip, after } = value;
  const flags = [value["protected"], value["pinned"], value["merged"]];
  if (
    typeof after !== "boolean" ||
    !isString(name) ||
    !isString(branch) ||
    !isLaneKind(kind) ||
    !isCount(row) ||
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
    row,
    protected: value["protected"] === true,
    pinned: value["pinned"] === true,
    merged: value["merged"] === true,
    ahead,
    behind,
    tip,
    after,
  };
}

type LaneOptions = Pick<Lane, "family" | "twin" | "only" | "score" | "fork" | "before">;

function parseLane(value: unknown): Lane | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const basics = laneBasics(value);
  const family = new Optional(value["family"], isCount);
  const twin = new Optional(value["twin"], isCount);
  const only = new Optional(value["only"], isOnly);
  const score = new Optional(value["score"], isNumber);
  const fork = new Optional(value["fork"], isOid);
  const before = new Optional(value["before"], isCount);
  const options = [family, twin, only, score, fork, before];
  if (basics === undefined || options.some((option) => !option.isValid)) {
    return undefined;
  }
  return {
    ...basics,
    ...field("family", family.value),
    ...field("twin", twin.value),
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
  const { oid, parents, author, committed, subject, refs, column, lanes } = value;
  return isOid(oid) &&
    isArrayOf(parents, isOid) &&
    isString(author) &&
    isNumber(committed) &&
    Number.isSafeInteger(committed) &&
    isString(subject) &&
    isArrayOf(refs, isString) &&
    isCount(column) &&
    isArrayOf(lanes, isCount)
    ? { oid, parents, author, committed, subject, refs, column, lanes }
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
  const { lanes, commits, grid } = graph;
  const isLaneIndex = (index: number): boolean => index < lanes.length;
  return (
    lanes.every(
      (lane) => lane.row < grid.height && (lane.twin === undefined || isLaneIndex(lane.twin)),
    ) &&
    commits.every((commit) => commit.lanes.every((lane) => isLaneIndex(lane))) &&
    grid.cells.every(
      (cell) =>
        cell.row < grid.height &&
        cell.col < grid.width &&
        isLaneIndex(cell.lane) &&
        (cell.commit === undefined || cell.commit < commits.length),
    )
  );
}

/** `value` as a graph document, or undefined if it isn't one. */
export function parseGraph(value: unknown): Graph | undefined {
  if (!isObject(value) || value["schema"] !== "sundgren.graph/2") {
    return undefined;
  }
  const { repository, threshold, candidates } = value;
  const base = parseNamed(value["base"]);
  const target = new Optional(
    value["target"],
    (item): item is Named => parseNamed(item) !== undefined,
  );
  const frame = parseFrame(value["frame"]);
  const lanes = parseAll(value["lanes"], parseLane);
  const commits = parseAll(value["commits"], parseCommit);
  const grid = parseGrid(value["grid"]);
  if (
    base === undefined ||
    frame === undefined ||
    lanes === undefined ||
    commits === undefined ||
    grid === undefined ||
    !target.isValid ||
    !isString(repository) ||
    !isNumber(threshold) ||
    !isCount(candidates)
  ) {
    return undefined;
  }
  const graph: Graph = {
    schema: "sundgren.graph/2",
    repository,
    base,
    ...field("target", target.value === undefined ? undefined : parseNamed(target.value)),
    threshold,
    frame,
    candidates,
    lanes,
    commits,
    grid,
  };
  return isConsistent(graph) ? graph : undefined;
}
