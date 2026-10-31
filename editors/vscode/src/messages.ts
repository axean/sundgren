// Messages between the extension and the history page. No VS Code, no DOM.

import type { Graph } from "./graph.js";

/** From the extension to the page. */
export type ToPage =
  | { readonly type: "graph"; readonly graph: Graph }
  | { readonly type: "error"; readonly message: string };

/** A control in a page's toolbar: what the user wants to change. */
export type Control = "base" | "target";

const CONTROLS = new Set<string>(["base", "target"]);

/**
From a page to the extension: it has loaded, it has drawn the graph of `oid`, the user asked to
see `line` of the file (the outline page), or to change `control` (the containment page).
*/
export type FromPage =
  | { readonly type: "ready" }
  | { readonly type: "rendered"; readonly oid: string; readonly rows: number }
  | { readonly type: "reveal"; readonly line: number }
  | { readonly type: "choose"; readonly control: Control };

/** `value` as a message from the page, or undefined if it isn't one. */
export function parseFromPage(value: unknown): FromPage | undefined {
  if (typeof value !== "object" || value === null || !("type" in value)) {
    return undefined;
  }
  if (value.type === "ready") {
    return { type: "ready" };
  }
  if (value.type === "choose") {
    return "control" in value && typeof value.control === "string" && CONTROLS.has(value.control)
      ? { type: "choose", control: value.control as Control }
      : undefined;
  }
  if (value.type === "reveal") {
    return "line" in value &&
      typeof value.line === "number" &&
      Number.isSafeInteger(value.line) &&
      value.line >= 1
      ? { type: "reveal", line: value.line }
      : undefined;
  }
  return value.type === "rendered" &&
    "oid" in value &&
    typeof value.oid === "string" &&
    "rows" in value &&
    typeof value.rows === "number"
    ? { type: "rendered", oid: value.oid, rows: value.rows }
    : undefined;
}
