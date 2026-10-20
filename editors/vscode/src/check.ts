// The `sundgren run --check` result (`sundgren.check/1`): the plan, and why it can't run if not.

export interface Problem {
  readonly code: string;
  readonly message: string;
}

export interface CheckResult {
  readonly isOk: boolean;
  readonly repository: string;
  readonly baseRef: string;
  readonly preset: string;
  /** The interpreter, if the preset uses `{python}`. */
  readonly python: string | undefined;
  readonly commands: readonly string[];
  readonly problems: readonly Problem[];
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function text(v: unknown): string {
  return typeof v === "string" ? v : "";
}

function parseProblems(v: unknown): Problem[] | undefined {
  const raw = Array.isArray(v) ? (v as unknown[]) : [];
  const problems: Problem[] = [];
  for (const p of raw) {
    if (!isObject(p) || typeof p["code"] !== "string" || typeof p["message"] !== "string") {
      return undefined;
    }
    problems.push({ code: p["code"], message: p["message"] });
  }
  return problems;
}

export function parseCheck(stdout: string): CheckResult | undefined {
  let raw: unknown;
  try {
    raw = JSON.parse(stdout) as unknown;
  } catch {
    return undefined;
  }
  if (!isObject(raw) || raw["schema"] !== "sundgren.check/1" || typeof raw["ok"] !== "boolean") {
    return undefined;
  }
  const problems = parseProblems(raw["problems"]);
  if (problems === undefined) {
    return undefined;
  }
  const commands = raw["commands"];
  return {
    isOk: raw["ok"],
    repository: text(raw["repo"]),
    baseRef: text(raw["base_ref"]),
    preset: text(raw["preset"]),
    python: typeof raw["python"] === "string" ? raw["python"] : undefined,
    commands: Array.isArray(commands) ? commands.filter((c) => typeof c === "string") : [],
    problems,
  };
}

/** A stable key for "the user confirmed running exactly this". */
export function planFingerprint(c: CheckResult): string {
  return JSON.stringify([c.repository, c.preset, c.python ?? "", c.commands]);
}
