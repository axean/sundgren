// The parts of a `sundgren.delta/1` report a pluck shows. Parsed defensively: anything that
// doesn't have the expected shape makes the whole report unreadable, never half-shown.

export interface LostFunction {
  readonly path: string;
  readonly name: string;
  readonly status: "went_dark" | "partial_loss";
  readonly start: number;
}

export interface PluckResult {
  readonly linesLost: number;
  readonly branchesLost: number;
  /** Line coverage before and after, in percent; undefined when nothing was executable. */
  readonly basePct: number | undefined;
  readonly patchedPct: number | undefined;
  // Repository-relative path -> lost line numbers, 1-based, in the base tree: the committed file
  // the user sees. (In the patched tree, lines after an inserted skip marker are shifted.)
  readonly lostLines: ReadonlyMap<string, readonly number[]>;
  readonly functions: readonly LostFunction[];
}

type Json = Record<string, unknown>;

/** The plucked test, whose own lines are left out: being skipped, they're trivially lost. */
export interface Exclude {
  /** Repository-relative path of the test file. */
  readonly path: string;
  /** The test function's name, e.g. `test_divide`. */
  readonly name: string;
}

// Whether a coverage function name is the test `name`: the plain name, a qualified one
// (`TestCalc.test_x`, `mycrate::tests::test_x`), or a mangled Rust symbol containing it.
export function isSameFunction(reported: string, name: string): boolean {
  return (
    reported === name ||
    reported.endsWith(`.${name}`) ||
    reported.endsWith(`::${name}`) ||
    (reported.startsWith("_") && reported.includes(`${String(name.length)}${name}`))
  );
}

function isObject(v: unknown): v is Json {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function count(v: unknown): number | undefined {
  return typeof v === "number" && Number.isSafeInteger(v) && v >= 0 ? v : undefined;
}

/** A percentage, `undefined` for JSON null, or `false` if it isn't either. */
function percent(v: unknown): number | undefined | false {
  if (v === null) {
    return undefined;
  }
  return typeof v === "number" ? v : false;
}

function list(v: unknown): unknown[] {
  return Array.isArray(v) ? (v as unknown[]) : [];
}

interface ParsedFile {
  readonly path: string;
  /** Lost lines, base-tree numbers, without the excluded test's own. */
  readonly lost: number[];
  readonly functions: LostFunction[];
  /** Lost lines and branches left out because they're the excluded test's own. */
  readonly excludedLines: number;
  readonly excludedBranches: number;
}

/** The patched-tree line range of each function in `records` that is the excluded test. */
function excludedRanges(records: readonly Json[], name: string | undefined): [number, number][] {
  if (name === undefined) {
    return [];
  }
  const ranges: [number, number][] = [];
  for (const record of records) {
    if (
      typeof record["name"] === "string" &&
      typeof record["start"] === "number" &&
      typeof record["end"] === "number" &&
      isSameFunction(record["name"], name)
    ) {
      ranges.push([record["start"], record["end"]]);
    }
  }
  return ranges;
}

function isInside(line: unknown, ranges: readonly [number, number][]): boolean {
  return typeof line === "number" && ranges.some(([a, b]) => line >= a && line <= b);
}

/** Lost line entries as base-tree numbers, and how many fell inside `ranges`; undefined if malformed. */
function lostLinesOf(
  entries: readonly unknown[],
  ranges: readonly [number, number][],
): { lost: number[]; excluded: number } | undefined {
  const lost: number[] = [];
  let excluded = 0;
  for (const entry of entries) {
    if (!isObject(entry)) {
      return undefined;
    }
    if (entry["category"] !== "lost") {
      continue;
    }
    // `line` is the patched-tree number, as function ranges are; `base_line` is the number in the
    // committed file, which is what gets highlighted.
    if (isInside(entry["line"], ranges)) {
      excluded += 1;
    } else if (typeof entry["base_line"] === "number") {
      lost.push(entry["base_line"]);
    }
  }
  return { lost, excluded };
}

function lostFunctionsOf(
  path: string,
  records: readonly Json[],
  excluded: string | undefined,
): LostFunction[] {
  const functions: LostFunction[] = [];
  for (const record of records) {
    const { name, status } = record;
    if (
      typeof name !== "string" ||
      (status !== "went_dark" && status !== "partial_loss") ||
      (excluded !== undefined && isSameFunction(name, excluded))
    ) {
      continue;
    }
    const start = typeof record["start"] === "number" ? record["start"] : 1;
    functions.push({ path, name, status, start });
  }
  return functions;
}

/** Lost lines and functions of one report file, or undefined if it's malformed. */
function parseFile(file: unknown, exclude: Exclude | undefined): ParsedFile | undefined {
  if (!isObject(file) || typeof file["path"] !== "string" || !Array.isArray(file["lines"])) {
    return undefined;
  }
  const path = file["path"];
  const records = list(file["functions"]);
  if (records.some((record) => !isObject(record))) {
    return undefined;
  }
  const objects = records as Json[];
  const ranges = excludedRanges(objects, exclude?.path === path ? exclude.name : undefined);
  const lines = lostLinesOf(list(file["lines"]), ranges);
  if (lines === undefined) {
    return undefined;
  }
  const excludedBranches = list(file["branches"]).filter(
    (entry) =>
      isObject(entry) && entry["category"] === "branch_lost" && isInside(entry["line"], ranges),
  ).length;
  return {
    path,
    lost: lines.lost,
    functions: lostFunctionsOf(path, objects, ranges.length > 0 ? exclude?.name : undefined),
    excludedLines: lines.excluded,
    excludedBranches,
  };
}

// The report as a pluck shows it. With `exclude`, the plucked test's own lines, branches and
// function are left out and the lost counts recounted without them; the coverage percentages
// would still include those lines, so they're left out too.
export function parseReport(raw: unknown, exclude?: Exclude): PluckResult | undefined {
  if (!isObject(raw) || raw["schema"] !== "sundgren.delta/1" || !isObject(raw["summary"])) {
    return undefined;
  }
  const summary = raw["summary"];
  const lines = isObject(summary["lines"]) ? summary["lines"] : {};
  const branches = isObject(summary["branches"]) ? summary["branches"] : {};
  const coverage = isObject(summary["coverage"]) ? summary["coverage"] : {};
  const linesLost = count(lines["lost"]);
  const branchesLost = count(branches["lost"]);
  const basePct = percent(coverage["base_pct"]);
  const patchedPct = percent(coverage["patched_pct"]);
  if (
    linesLost === undefined ||
    branchesLost === undefined ||
    basePct === false ||
    patchedPct === false ||
    !Array.isArray(raw["files"])
  ) {
    return undefined;
  }
  const lostLines = new Map<string, number[]>();
  const functions: LostFunction[] = [];
  let excludedLines = 0;
  let excludedBranches = 0;
  const files = list(raw["files"]);
  for (const file of files) {
    const parsed = parseFile(file, exclude);
    if (parsed === undefined) {
      return undefined;
    }
    if (parsed.lost.length > 0) {
      lostLines.set(parsed.path, parsed.lost);
    }
    functions.push(...parsed.functions);
    excludedLines += parsed.excludedLines;
    excludedBranches += parsed.excludedBranches;
  }
  const isExcluded = excludedLines + excludedBranches > 0;
  return {
    linesLost: Math.max(0, linesLost - excludedLines),
    branchesLost: Math.max(0, branchesLost - excludedBranches),
    basePct: isExcluded ? undefined : basePct,
    patchedPct: isExcluded ? undefined : patchedPct,
    lostLines,
    functions,
  };
}

function plural(n: number, word: string, suffix: string): string {
  return `${String(n)} ${word}${n === 1 ? "" : suffix}`;
}

/** One sentence for the notification. */
export function headline(testName: string, r: PluckResult): string {
  if (r.linesLost === 0 && r.branchesLost === 0) {
    return `Plucking ${testName} loses no coverage.`;
  }
  const parts = [`${plural(r.linesLost, "line", "s")} lost`];
  if (r.branchesLost > 0) {
    parts.push(`${plural(r.branchesLost, "branch", "es")} lost`);
  }
  const dark = r.functions.filter((f) => f.status === "went_dark").map((f) => f.name);
  if (dark.length > 0) {
    parts.push(`${dark.join(", ")} went dark`);
  }
  let coverage = "";
  if (r.basePct !== undefined && r.patchedPct !== undefined) {
    coverage = ` (coverage ${String(r.basePct)}% → ${String(r.patchedPct)}%)`;
  }
  return `Plucking ${testName}: ${parts.join(", ")}${coverage}.`;
}
