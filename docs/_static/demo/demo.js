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

const DAY = 86_400;

/** How alike two lists of hidden branches are: 1 for the same, 0 for nothing in common. */
function alike(a, b) {
  const all = new Set([...a, ...b]);
  if (all.size === 0) {
    return 1;
  }
  return a.filter((name) => b.includes(name)).length / all.size;
}

/**
 * What a date interval shows of the demo's history: `any` with none, else the days with commits
 * it covers, or `before`/`after` the history when it covers none.
 */
function daysKey(since, until) {
  if (since === undefined && until === undefined) {
    return "any";
  }
  const from = since ?? 0;
  const to = until ?? Number.MAX_SAFE_INTEGER;
  const covered = demo.days.filter((day) => day <= to && day + DAY - 1 >= from);
  if (covered.length > 0) {
    return covered.join(",");
  }
  return to < demo.days[0] ? "before" : "after";
}

function viewKey(v) {
  return v.dates === undefined ? "any" : daysKey(v.dates[0], v.dates[1]);
}

/** The canned view for `view`: an exact one if the demo has it, else the closest. */
function closest(view) {
  const days = daysKey(view.since, view.until);
  const same = demo.graphs.filter((v) => modeOf(v) === view.show);
  const pool = same.length > 0 ? same : demo.graphs;
  const isSameHides = (v) =>
    v.hides.length === view.hides.length && alike(v.hides, view.hides) === 1;
  // With a first day, the page is the days; else it is `commits` back.
  const isSamePage = (v) => view.since !== undefined || v.commits === view.commits;
  const exact = pool.find(
    (v) => same.length > 0 && viewKey(v) === days && isSameHides(v) && isSamePage(v),
  );
  if (exact !== undefined) {
    return { graph: exact.graph, isExact: true };
  }
  const nearest = (views) =>
    views.toSorted(
      (a, b) => Math.abs(a.commits - view.commits) - Math.abs(b.commits - view.commits),
    )[0];
  const score = (v) => 2 * (viewKey(v) === days ? 1 : 0) + alike(v.hides, view.hides);
  const best = Math.max(...pool.map(score));
  return { graph: nearest(pool.filter((v) => score(v) === best)).graph, isExact: false };
}

/** A note when the demo shows the closest view it has, not the one asked for. */
function note(isExact) {
  let node = document.querySelector(".demo-note");
  if (isExact) {
    node?.remove();
    return;
  }
  if (node === null) {
    node = document.createElement("div");
    node.className = "demo-note";
    node.style.cssText =
      "position:fixed;z-index:15;right:12px;bottom:12px;max-width:340px;padding:6px 10px;" +
      "border:1px solid var(--border);border-radius:6px;background:var(--bg);" +
      "color:var(--muted);font-size:0.85em;box-shadow:0 2px 8px rgb(0 0 0 / 20%)";
    node.textContent =
      "The demo has views made in advance, and none for exactly this: it shows the closest. " +
      "In VS Code, sundgren draws what you ask for.";
    document.body.append(node);
  }
}

function send(message) {
  window.postMessage(message, window.origin);
}

/** Run `view`, as the extension does: say it is running, then post its graph. */
function run(view) {
  current = view;
  send({ type: "running", view });
  const { graph, isExact } = closest(view);
  setTimeout(() => {
    send({ type: "graph", graph });
    note(isExact);
  }, 150);
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
      send({ type: "graph", graph: closest(current).graph });
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
