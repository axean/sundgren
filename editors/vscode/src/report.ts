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
  /** Repository-relative path -> lost line numbers (1-based, in the patched tree). */
  readonly lostLines: ReadonlyMap<string, readonly number[]>;
  readonly functions: readonly LostFunction[];
}

type Json = Record<string, unknown>;

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

/** Lost lines and functions of one report file, or undefined if it's malformed. */
function parseFile(
  file: unknown,
): { path: string; lost: number[]; functions: LostFunction[] } | undefined {
  if (!isObject(file) || typeof file["path"] !== "string" || !Array.isArray(file["lines"])) {
    return undefined;
  }
  const path = file["path"];
  const lost: number[] = [];
  const entries = list(file["lines"]);
  for (const entry of entries) {
    if (!isObject(entry)) {
      return undefined;
    }
    if (entry["category"] === "lost" && typeof entry["line"] === "number") {
      lost.push(entry["line"]);
    }
  }
  const functions: LostFunction[] = [];
  const records = list(file["functions"]);
  for (const record of records) {
    if (!isObject(record)) {
      return undefined;
    }
    const status = record["status"];
    const name = record["name"];
    if (typeof name !== "string" || (status !== "went_dark" && status !== "partial_loss")) {
      continue;
    }
    const start = typeof record["start"] === "number" ? record["start"] : 1;
    functions.push({ path, name, status, start });
  }
  return { path, lost, functions };
}

export function parseReport(raw: unknown): PluckResult | undefined {
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
  const files = list(raw["files"]);
  for (const file of files) {
    const parsed = parseFile(file);
    if (parsed === undefined) {
      return undefined;
    }
    if (parsed.lost.length > 0) {
      lostLines.set(parsed.path, parsed.lost);
    }
    functions.push(...parsed.functions);
  }
  return { linesLost, branchesLost, basePct, patchedPct, lostLines, functions };
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
