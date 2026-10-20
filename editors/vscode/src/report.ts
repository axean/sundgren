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
  readonly basePct: number | null;
  readonly patchedPct: number | null;
  /** Repo-relative path -> lost line numbers (1-based, in the patched tree). */
  readonly lostLines: ReadonlyMap<string, readonly number[]>;
  readonly functions: readonly LostFunction[];
}

type Json = Record<string, unknown>;

function isObject(v: unknown): v is Json {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function count(v: unknown): number | undefined {
  return typeof v === "number" && Number.isInteger(v) && v >= 0 ? v : undefined;
}

function pct(v: unknown): number | null | undefined {
  return v === null ? null : typeof v === "number" ? v : undefined;
}

export function parseReport(raw: unknown): PluckResult | undefined {
  if (!isObject(raw) || raw["schema"] !== "sundgren.delta/1") {
    return undefined;
  }
  const summary = raw["summary"];
  const files = raw["files"];
  if (!isObject(summary) || !Array.isArray(files)) {
    return undefined;
  }
  const lines = summary["lines"];
  const branches = summary["branches"];
  const coverage = summary["coverage"];
  if (!isObject(lines) || !isObject(branches) || !isObject(coverage)) {
    return undefined;
  }
  const linesLost = count(lines["lost"]);
  const branchesLost = count(branches["lost"]);
  const basePct = pct(coverage["base_pct"]);
  const patchedPct = pct(coverage["patched_pct"]);
  if (linesLost === undefined || branchesLost === undefined || basePct === undefined || patchedPct === undefined) {
    return undefined;
  }
  const lostLines = new Map<string, number[]>();
  const functions: LostFunction[] = [];
  for (const file of files) {
    if (!isObject(file) || typeof file["path"] !== "string" || !Array.isArray(file["lines"])) {
      return undefined;
    }
    const path = file["path"];
    for (const entry of file["lines"]) {
      if (!isObject(entry)) {
        return undefined;
      }
      if (entry["category"] === "lost" && typeof entry["line"] === "number") {
        lostLines.set(path, [...(lostLines.get(path) ?? []), entry["line"]]);
      }
    }
    for (const fn of Array.isArray(file["functions"]) ? file["functions"] : []) {
      if (!isObject(fn)) {
        return undefined;
      }
      const status = fn["status"];
      if ((status === "went_dark" || status === "partial_loss") && typeof fn["name"] === "string") {
        const start = typeof fn["start"] === "number" ? fn["start"] : 1;
        functions.push({ path, name: fn["name"], status, start });
      }
    }
  }
  return { linesLost, branchesLost, basePct, patchedPct, lostLines, functions };
}

/** One sentence for the notification. */
export function headline(testName: string, r: PluckResult): string {
  if (r.linesLost === 0 && r.branchesLost === 0) {
    return `Plucking ${testName} loses no coverage.`;
  }
  const parts = [`${String(r.linesLost)} line${r.linesLost === 1 ? "" : "s"} lost`];
  if (r.branchesLost > 0) {
    parts.push(`${String(r.branchesLost)} branch${r.branchesLost === 1 ? "" : "es"} lost`);
  }
  const dark = r.functions.filter((f) => f.status === "went_dark").map((f) => f.name);
  if (dark.length > 0) {
    parts.push(`${dark.join(", ")} went dark`);
  }
  const coverage =
    r.basePct !== null && r.patchedPct !== null ? ` (coverage ${String(r.basePct)}% → ${String(r.patchedPct)}%)` : "";
  return `Plucking ${testName}: ${parts.join(", ")}${coverage}.`;
}
