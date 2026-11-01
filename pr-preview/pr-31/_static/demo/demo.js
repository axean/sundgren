// Stands in for VS Code around the extension's history page (webview.js): answers its messages
// from the canned views in graph.js (every Show mode, pages growing by demo.page commits, the
// first page with each branch hidden, and each run of days), and follows the docs site's
// light/dark theme. Base and Like open a small stand-in for VS Code's branch picker, with the
// branches the demo has views of.
"use strict";

const demo = window.sundgrenDemoGraphs;
const BASE = "main";
let current = {
  show: "local",
  branches: 12,
  commits: demo.page,
  pins: [],
  hides: [],
};

/** The Show mode a view was made with: ["--show", "local"] or ["--like", "export"]. */
function modeOf(view) {
  return view.show[0] === "--like" ? "like" : view.show[1];
}

/** How alike two lists of hidden branches are: 1 for the same, 0 for nothing in common. */
function alike(a, b) {
  const all = new Set([...a, ...b]);
  if (all.size === 0) {
    return 1;
  }
  return a.filter((name) => b.includes(name)).length / all.size;
}

/** How alike two date intervals are: 1 for the same (or both none), 0 for no overlap. */
function alikeDates(a, b) {
  if (a === undefined || b === undefined) {
    return a === b ? 1 : 0;
  }
  const overlap = Math.min(a[1], b[1]) - Math.max(a[0], b[0]);
  const span = Math.max(a[1], b[1]) - Math.min(a[0], b[0]);
  return overlap <= 0 ? 0 : overlap / span;
}

/** The days a page asks for, if any; an open end goes to the edge of time. */
function datesOf(view) {
  if (view.since === undefined && view.until === undefined) {
    return undefined;
  }
  return [view.since ?? 0, view.until ?? Number.MAX_SAFE_INTEGER];
}

/**
 * The canned view closest to `view`: same mode, the same days, the same branches hidden, the
 * nearest page.
 */
function closest(view) {
  const same = demo.graphs.filter((v) => modeOf(v) === view.show);
  const pool = same.length > 0 ? same : demo.graphs;
  const dates = datesOf(view);
  const score = (v) => 2 * alikeDates(v.dates, dates) + alike(v.hides, view.hides);
  const sorted = pool.toSorted(
    (a, b) =>
      score(b) - score(a) ||
      Math.abs(a.commits - view.commits) - Math.abs(b.commits - view.commits),
  );
  return sorted[0].graph;
}

function send(message) {
  window.postMessage(message, window.origin);
}

/** Run `view`, as the extension does: say it is running, then post its graph. */
function run(view) {
  current = view;
  send({ type: "running", view });
  setTimeout(() => send({ type: "graph", graph: closest(view) }), 150);
}

/** The branches the demo can rank by (`--like`). */
function likes() {
  return [
    ...new Set(
      demo.graphs.filter((v) => modeOf(v) === "like").map((v) => v.show[1]),
    ),
  ];
}

/** A small stand-in for VS Code's quick pick: `title`, then `items`; `onPick` gets the one picked. */
function quickPick(title, items, note, onPick) {
  document.querySelector(".demo-pick")?.remove();
  const box = document.createElement("div");
  box.className = "demo-pick";
  box.setAttribute("role", "dialog");
  box.style.cssText =
    "position:fixed;z-index:20;top:12px;left:50%;transform:translateX(-50%);width:min(420px,90vw);" +
    "padding:6px;border:1px solid var(--border);border-radius:6px;background:var(--bg);" +
    "box-shadow:0 4px 16px rgb(0 0 0 / 30%);font:inherit;color:var(--text)";
  const head = document.createElement("div");
  head.textContent = title;
  head.style.cssText = "padding:4px 6px;color:var(--muted)";
  box.append(head);
  const close = () => {
    box.remove();
    removeEventListener("keydown", onKey);
    removeEventListener("pointerdown", onOutside);
  };
  for (const item of items) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = item;
    button.style.cssText =
      "display:block;width:100%;padding:4px 6px;border:0;border-radius:3px;text-align:left;" +
      "background:transparent;color:var(--text);font:inherit;cursor:pointer";
    button.addEventListener("mouseenter", () => (button.style.background = "var(--hover)"));
    button.addEventListener("mouseleave", () => (button.style.background = "transparent"));
    button.addEventListener("click", () => {
      close();
      onPick(item);
    });
    box.append(button);
  }
  const foot = document.createElement("div");
  foot.textContent = note;
  foot.style.cssText = "padding:4px 6px;color:var(--muted);font-size:0.85em";
  box.append(foot);
  function onKey(event) {
    if (event.key === "Escape") {
      close();
    }
  }
  function onOutside(event) {
    if (!box.contains(event.target)) {
      close();
    }
  }
  addEventListener("keydown", onKey);
  addEventListener("pointerdown", onOutside);
  document.body.append(box);
  box.querySelector("button")?.focus();
}

function choose(control) {
  if (control === "base") {
    quickPick(
      `Base: the reference line (now ${BASE})`,
      [BASE],
      "The demo has views of one base; in VS Code, any branch.",
      () => run(current),
    );
  } else if (control === "like") {
    quickPick(
      "Like: show the branches whose work is most found in this one",
      likes(),
      "The demo ranks by these; in VS Code, any branch.",
      (like) => run({ ...current, show: "like", like }),
    );
  }
}

window.acquireVsCodeApi = () => ({
  postMessage(message) {
    if (message.type === "ready") {
      send({ type: "setup", base: BASE, view: current, step: demo.page, states: [] });
      send({ type: "graph", graph: closest(current) });
    } else if (message.type === "view") {
      const view = { ...message };
      delete view.type;
      run(view);
    } else if (message.type === "choose") {
      choose(message.control);
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
