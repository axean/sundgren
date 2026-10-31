// The containment page: for each branch, how much of its work is in the target branch and how much
// of the target's is in it, filled in as results arrive. Runs in the webview; bundled to
// dist/containment.js.

import {
  compareResults,
  median,
  type Containment,
  type ItemShare,
  type Relation,
  type ToContainmentPage,
} from "../containment.js";
import type { FromPage } from "../messages.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

const RELATION_LABELS: Readonly<Record<Relation, string>> = {
  same: "same work",
  "a-in-b": "in target",
  "b-in-a": "contains target",
  overlap: "overlap",
  unrelated: "unrelated",
  empty: "nothing ahead",
};

interface Entry {
  result?: Containment;
  error?: string;
}

const state = {
  target: "",
  base: "",
  entries: new Map<string, Entry>(),
  expanded: new Set<string>(),
  isDone: false,
};

function element<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

function percent(value: number | undefined): string {
  return value === undefined ? "–" : `${String(Math.round(value * 100))}%`;
}

function bar(className: string, fraction: number): HTMLDivElement {
  const node = element("div", className);
  node.style.width = `${String(Math.max(0, Math.min(1, fraction)) * 100)}%`;
  return node;
}

function bars(result: Containment): HTMLDivElement {
  const container = element("div", "bars");
  const ahead = result.a_in_b;
  const total = Math.max(ahead.lines, 1);
  const main = element("div", "track");
  main.append(bar("exact", ahead.exact / total), bar("fuzzy", ahead.fuzzy / total));
  const reverse = element("div", "track thin");
  reverse.append(bar("reverse", result.b_in_a.containment ?? 0));
  container.append(main, reverse, element("div", "median"));
  return container;
}

/** `path`'s last part, for a file named again next to its full path. */
function fileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

/** A row for one of a file's items: its name, and where its lines are in the target. */
function itemRow(item: ItemShare): HTMLTableRowElement {
  const row = element("tr", "item");
  const name = element("td", "path");
  name.append(element("span", "item-name", item.item ?? "outside items"));
  if (item.kind !== undefined) {
    name.title = item.kind;
  }
  const there = item.counterpart;
  if (there !== undefined) {
    const elsewhere = there.path === item.path ? "" : `${fileName(there.path)} `;
    const isSame = elsewhere === "" && there.item === item.item;
    const target = element(
      "span",
      isSame ? "counterpart same" : "counterpart",
      `→ ${elsewhere}${there.item ?? "outside items"}`,
    );
    const where = there.item === undefined ? there.path : `${there.path}, ${there.item}`;
    target.title = `${String(there.lines)} of its found lines are in ${where}`;
    name.append(target);
  }
  row.append(
    element("td", "number", String(item.exact)),
    element("td", "number", String(item.fuzzy)),
    element("td", "number", String(item.lines)),
    name,
  );
  return row;
}

function filesTable(result: Containment): HTMLElement {
  const container = element("div", "detail");
  const ahead = result.a_in_b;
  if (ahead.comment_lines !== undefined) {
    container.append(
      element(
        "p",
        "note",
        `Without its ${String(ahead.comment_lines)} comment lines: ${percent(ahead.code_containment)} in ${state.target}.`,
      ),
    );
  }
  const table = element("table", "files");
  const head = element("tr", "");
  for (const title of ["exact", "fuzzy", "lines", "file, and its items → where in the target"]) {
    head.append(element("th", "", title));
  }
  table.append(head);
  const files = ahead.files.toSorted(
    (a, b) => b.exact + b.fuzzy - (a.exact + a.fuzzy) || b.lines - a.lines,
  );
  const items = ahead.items ?? [];
  for (const file of files) {
    const row = element("tr", "file");
    row.append(
      element("td", "number", String(file.exact)),
      element("td", "number", String(file.fuzzy)),
      element("td", "number", String(file.lines)),
      element("td", "path", file.path),
    );
    table.append(row);
    const own = items.filter((item) => item.path === file.path);
    table.append(...own.map((item) => itemRow(item)));
  }
  container.append(table);
  return container;
}

function branchRow(branch: string, entry: Entry): HTMLElement {
  const block = element("section", "branch");
  const row = element("div", "row");
  row.dataset["branch"] = branch;
  const { result, error } = entry;
  if (result === undefined) {
    row.append(
      element(
        "span",
        error === undefined ? "relation pending" : "relation failed",
        error ? "failed" : "…",
      ),
      element("span", "name", branch),
      element("span", "lines", ""),
      element("span", "message", error ?? "comparing"),
    );
    block.append(row);
    return block;
  }
  row.classList.add("clickable");
  row.title = `${String(result.a_in_b.exact)} exact + ${String(result.a_in_b.fuzzy)} fuzzy of ${String(result.a.lines)} lines are in ${state.target}; ${percent(result.b_in_a.containment)} of ${state.target}'s ${String(result.b.lines)} lines are in ${branch}. In order: ${percent(result.a_in_b.ordered_containment)}.`;
  row.append(
    element("span", `relation ${result.relation}`, RELATION_LABELS[result.relation]),
    element("span", "name", branch),
    element("span", "lines", `${String(result.a.lines)} lines since ${state.base}`),
    bars(result),
    element("span", "percent", percent(result.a_in_b.containment)),
    element("span", "percent reverse-percent", percent(result.b_in_a.containment)),
  );
  block.append(row);
  if (state.expanded.has(branch)) {
    block.classList.add("open");
    block.append(filesTable(result));
  }
  return block;
}

function render(): void {
  const root = document.querySelector("#root");
  if (root === null) {
    return;
  }
  const entries = [...state.entries];
  const finished = entries.filter(
    ([, entry]) => entry.result !== undefined || entry.error !== undefined,
  );
  const scores = entries
    .map(([, entry]) => entry.result?.a_in_b.containment)
    .filter((score): score is number => typeof score === "number");
  const middle = median(scores);
  const targetLines = entries.find(([, entry]) => entry.result)?.[1].result?.b.lines;

  const header = element("header", "header");
  header.append(
    element("span", "title", `Containment in ${state.target}`),
    element(
      "span",
      "detail",
      targetLines === undefined
        ? `work since ${state.base}`
        : `${String(targetLines)} lines added since ${state.base}`,
    ),
    element(
      "span",
      "count",
      state.isDone
        ? `${String(entries.length)} branches`
        : `${String(finished.length)} / ${String(entries.length)} compared`,
    ),
  );
  const level = middle === undefined ? "" : ` (${percent(middle)})`;
  const legend = element(
    "p",
    "legend",
    `Work: what a branch added since it left ${state.base}, the canonical branch ` +
      `(the setting sundgren.base changes it). ` +
      `Bar: how much of each branch's work is in ${state.target} (solid exact, light fuzzy). ` +
      `Thin bar: how much of ${state.target} is in the branch. Dashed line: the median, ` +
      `about the level that boilerplate alone reaches${level}. Click a branch for its files.`,
  );
  const list = element("div", "rows");
  list.style.setProperty("--median", `${String((middle ?? 0) * 100)}%`);
  list.classList.toggle("has-median", middle !== undefined);
  entries.sort(([, a], [, b]) => compareResults(a.result, b.result));
  list.append(...entries.map(([branch, entry]) => branchRow(branch, entry)));
  list.addEventListener("click", (event) => {
    const row =
      event.target instanceof Element ? event.target.closest(".row.clickable") : undefined;
    const branch = row instanceof HTMLElement ? row.dataset["branch"] : undefined;
    if (branch === undefined) {
      return;
    }
    if (state.expanded.has(branch)) {
      state.expanded.delete(branch);
    } else {
      state.expanded.add(branch);
    }
    render();
  });
  root.replaceChildren(header, legend, list);
}

function receive(message: ToContainmentPage): void {
  switch (message.type) {
    case "start": {
      state.target = message.target;
      state.base = message.base;
      state.entries = new Map(message.branches.map((branch) => [branch, {}]));
      state.expanded.clear();
      state.isDone = false;
      break;
    }
    case "result": {
      state.entries.set(message.branch, { result: message.result });
      break;
    }
    case "failed": {
      state.entries.set(message.branch, { error: message.message });
      break;
    }
    case "done": {
      state.isDone = true;
      break;
    }
  }
  render();
  if (message.type !== "done") {
    return;
  }
  let rows = 0;
  for (const entry of state.entries.values()) {
    if (entry.result !== undefined) {
      rows += 1;
    }
  }
  send({ type: "rendered", oid: "containment", rows });
}

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToContainmentPage>) => {
  if (event.origin !== origin) {
    return;
  }
  receive(event.data);
});

send({ type: "ready" });
