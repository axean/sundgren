// Stands in for VS Code around the extension's history page (webview.js): answers its messages
// from the views in graph.js, made by sundgren at one width, and follows the docs site's
// light/dark theme.
"use strict";

const demo = window.sundgrenDemoGraphs;
const BASE = "main";
const TARGET = "search";

function send(message) {
  window.postMessage(message, window.origin);
}

/** The view closest to what the page asks for: same branch count, nearest frame. */
function closest(branches, offset) {
  const counts = demo.graphs.map((v) => v.branches);
  const n = counts.includes(branches) ? branches : Math.max(...counts);
  const same = demo.graphs.filter((v) => v.branches === n);
  same.sort((a, b) => Math.abs(a.offset - offset) - Math.abs(b.offset - offset));
  return same[0].graph;
}

window.acquireVsCodeApi = () => ({
  postMessage(message) {
    if (message.type === "ready") {
      send({ type: "setup", base: BASE, target: TARGET, branches: 3, states: [] });
      send({ type: "graph", graph: closest(3, 0) });
    } else if (message.type === "view") {
      send({ type: "graph", graph: closest(message.branches, message.offset) });
    }
  },
  getState() {
    return undefined;
  },
  setState() {},
});

function parentRoot() {
  try {
    return window.parent === window
      ? undefined
      : window.parent.document.documentElement;
  } catch {
    return undefined;
  }
}

function syncTheme() {
  const root = parentRoot();
  const isDark =
    root === undefined
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : root.classList.contains("dark");
  document.body.classList.toggle("vscode-dark", isDark);
  document.body.classList.toggle("vscode-light", !isDark);
}

syncTheme();
const root = parentRoot();
if (root !== undefined) {
  new MutationObserver(syncTheme).observe(root, {
    attributes: true,
    attributeFilter: ["class"],
  });
}
