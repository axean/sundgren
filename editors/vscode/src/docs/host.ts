// What the docs site's demos share: their data, the stand-in for VS Code's webview API, the site's
// theme, and a branch picker like VS Code's quick pick. The demos run the extension's page scripts
// in an iframe of the site, with data made by the docs build (tools/demo_*.py).

import { parseContainment, type Comments, type Containment } from "../containment.js";

/** The history demo's data (graph.js). */
export interface DemoGraphs {
  /** Base commits per page. */
  readonly page: number;
  /** UTC days with commits in the demo history, as unix seconds at their start. */
  readonly days: readonly number[];
  readonly graphs: readonly DemoGraph[];
}

/** A view made in advance: the arguments it was made with, and sundgren's graph JSON. */
export interface DemoGraph {
  /** `["--show", "local"]` or `["--like", "export"]`. */
  readonly show: readonly string[];
  readonly commits: number;
  readonly hides: readonly string[];
  readonly dates?: readonly [number, number];
  /** As sundgren printed it: the page parses it, as it does in VS Code. */
  readonly graph: unknown;
}

/** The containment and inspect demos' data (containment-data.js). */
export interface DemoContainment {
  readonly base: string;
  /** The target shown first. */
  readonly target: string;
  /** The branches that can be the target, in the data's order. */
  readonly names: readonly string[];
  /** By target: every other branch compared with it, comments ignored and counted. */
  readonly targets: ReadonlyMap<string, Readonly<Record<Comments, readonly DemoResult[]>>>;
}

export interface DemoResult {
  readonly branch: string;
  readonly result: Containment;
}

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStrings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isNumbers(value: unknown): value is number[] {
  return Array.isArray(value) && value.every((item) => typeof item === "number");
}

/** What a data script (graph.js, containment-data.js) set on the page: `name` on the global. */
function dataNamed(name: string): unknown {
  const value: unknown = Reflect.get(globalThis, name);
  return value;
}

function parseDemoGraph(value: unknown): DemoGraph | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const { show, commits, hides, dates, graph } = value;
  if (typeof commits !== "number" || !isStrings(show) || !isStrings(hides)) {
    return undefined;
  }
  if (dates === undefined) {
    return { show, commits, hides, graph };
  }
  const [since, until] = isNumbers(dates) ? dates : [];
  return since === undefined || until === undefined
    ? undefined
    : { show, commits, hides, dates: [since, until], graph };
}

/** The history demo's data, if graph.js set it. */
export function demoGraphs(): DemoGraphs | undefined {
  const value = dataNamed("sundgrenDemoGraphs");
  if (!isObject(value)) {
    return undefined;
  }
  const { page, days, graphs } = value;
  if (typeof page !== "number" || !isNumbers(days) || !Array.isArray(graphs)) {
    return undefined;
  }
  const parsed = graphs.map((item) => parseDemoGraph(item));
  const made = parsed.filter((item) => item !== undefined);
  return made.length === parsed.length ? { page, days, graphs: made } : undefined;
}

function parseResults(value: unknown): DemoResult[] | undefined {
  if (!Array.isArray(value)) {
    return undefined;
  }
  const results: DemoResult[] = [];
  for (const item of value) {
    const branch = isObject(item) ? item["branch"] : undefined;
    const result = isObject(item) ? parseContainment(item["result"]) : undefined;
    if (typeof branch !== "string" || result === undefined) {
      return undefined;
    }
    results.push({ branch, result });
  }
  return results;
}

/** The containment demos' data, if containment-data.js set it. */
export function demoContainment(): DemoContainment | undefined {
  const value = dataNamed("sundgrenDemoContainment");
  if (!isObject(value)) {
    return undefined;
  }
  const { base, target, targets } = value;
  if (typeof base !== "string" || typeof target !== "string" || !isObject(targets)) {
    return undefined;
  }
  const byTarget = new Map<string, Record<Comments, DemoResult[]>>();
  for (const [name, modes] of Object.entries(targets)) {
    const ignored = isObject(modes) ? parseResults(modes["ignored"]) : undefined;
    const counted = isObject(modes) ? parseResults(modes["counted"]) : undefined;
    if (ignored === undefined || counted === undefined) {
      return undefined;
    }
    byTarget.set(name, { ignored, counted });
  }
  return { base, target, names: Object.keys(targets), targets: byTarget };
}

/**
Stand in for VS Code: the page script gets its API from the global `acquireVsCodeApi`, as in a
webview, and its messages go to `onMessage`.
*/
export function standIn(onMessage: (message: unknown) => void): void {
  Object.defineProperty(globalThis, "acquireVsCodeApi", {
    value: () => ({ postMessage: onMessage }),
    configurable: true,
  });
}

/** Post `message` to the page, as VS Code does. */
export function post(message: unknown): void {
  postMessage(message, origin);
}

function parentRoot(): HTMLElement | undefined {
  try {
    // Not in a frame of the site (or not of the same origin): no parent to follow.
    return frameElement === null ? undefined : parent.document.documentElement;
  } catch {
    return undefined;
  }
}

function syncTheme(): void {
  const root = parentRoot();
  const isDark =
    root === undefined
      ? matchMedia("(prefers-color-scheme: dark)").matches
      : root.classList.contains("dark");
  document.body.classList.toggle("vscode-dark", isDark);
  document.body.classList.toggle("vscode-light", !isDark);
}

/** Follow the docs site's light or dark theme, as VS Code tells a webview with body classes. */
export function followTheme(): void {
  syncTheme();
  const root = parentRoot();
  if (root !== undefined) {
    new MutationObserver(syncTheme).observe(root, {
      attributes: true,
      attributeFilter: ["class"],
    });
  }
}

/** A list to pick from, like VS Code's quick pick, with an optional `note` under it. */
export function picker(
  title: string,
  options: readonly string[],
  current: string | undefined,
  onPick: (option: string) => void,
  note?: string,
): void {
  const backdrop = document.createElement("div");
  backdrop.className = "sg-picker-backdrop";
  const box = document.createElement("div");
  box.className = "sg-picker";
  box.setAttribute("role", "dialog");
  const heading = document.createElement("p");
  heading.className = "sg-picker-title";
  heading.textContent = title;
  box.append(heading);
  const close = (): void => {
    backdrop.remove();
  };
  for (const option of options) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = option === current ? "sg-picker-option current" : "sg-picker-option";
    button.textContent = option;
    button.addEventListener("click", () => {
      close();
      onPick(option);
    });
    box.append(button);
  }
  if (note !== undefined) {
    const foot = document.createElement("p");
    foot.className = "sg-picker-note";
    foot.textContent = note;
    box.append(foot);
  }
  backdrop.addEventListener("click", (event) => {
    if (event.target === backdrop) {
      close();
    }
  });
  backdrop.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      close();
    }
  });
  backdrop.append(box);
  document.body.append(backdrop);
  box.querySelector("button")?.focus();
}
