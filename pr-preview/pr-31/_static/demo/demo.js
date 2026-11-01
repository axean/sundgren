// Stands in for VS Code around the extension's history page (webview.js): answers its messages
// from the views in graph.js (every Show mode, pages growing by demo.page commits) and follows
// the docs site's light/dark theme. The branch count is fixed at the default.
"use strict";

const demo = window.sundgrenDemoGraphs;
const BASE = "main";

/** The Show mode a view was made with: ["--show", "local"] or ["--like", "export"]. */
function modeOf(view) {
  return view.show[0] === "--like" ? "like" : view.show[1];
}

/** The view closest to what the page asks for: same mode, the page nearest in size. */
function closest(show, commits) {
  const same = demo.graphs.filter((v) => modeOf(v) === show);
  const pool = same.length > 0 ? same : demo.graphs;
  pool.sort(
    (a, b) => Math.abs(a.commits - commits) - Math.abs(b.commits - commits),
  );
  return pool[0].graph;
}

function send(message) {
  window.postMessage(message, window.origin);
}

window.acquireVsCodeApi = () => ({
  postMessage(message) {
    if (message.type === "ready") {
      const view = { show: "local", branches: 12, commits: demo.page };
      send({ type: "setup", base: BASE, view, step: demo.page, states: [] });
      send({ type: "graph", graph: closest("local", demo.page) });
    } else if (message.type === "view") {
      send({ type: "graph", graph: closest(message.show, message.commits) });
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
