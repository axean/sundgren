// Stands in for VS Code around the extension's history page (webview.js): hands it the demo graph
// (graph.js) and follows the docs site's light/dark theme.
"use strict";

window.acquireVsCodeApi = () => ({
  postMessage(message) {
    if (message.type !== "ready") {
      return;
    }
    window.postMessage(
      { type: "graph", graph: window.sundgrenDemoGraph },
      window.origin,
    );
  },
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
