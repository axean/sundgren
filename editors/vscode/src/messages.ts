// Messages between the extension and the history page. No VS Code, no DOM.

import type { Graph } from "./graph.js";

/** From the extension to the page. */
export type ToPage =
  | { readonly type: "graph"; readonly graph: Graph }
  | { readonly type: "error"; readonly message: string };

/** From the page to the extension: it has loaded, or it has drawn the graph of `oid`. */
export type FromPage =
  | { readonly type: "ready" }
  | { readonly type: "rendered"; readonly oid: string; readonly rows: number };

/** `value` as a message from the page, or undefined if it isn't one. */
export function parseFromPage(value: unknown): FromPage | undefined {
  if (typeof value !== "object" || value === null || !("type" in value)) {
    return undefined;
  }
  if (value.type === "ready") {
    return { type: "ready" };
  }
  return value.type === "rendered" &&
    "oid" in value &&
    typeof value.oid === "string" &&
    "rows" in value &&
    typeof value.rows === "number"
    ? { type: "rendered", oid: value.oid, rows: value.rows }
    : undefined;
}
