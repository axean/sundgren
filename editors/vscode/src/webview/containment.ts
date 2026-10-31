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
import type { Control, FromPage, Revision } from "../messages.js";

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

/** A toolbar button: `label` and the current `value`; a click asks the extension to change it. */
function control(name: Control, label: string, value: string, help: string): HTMLButtonElement {
  const button = element("button", "control");
  button.type = "button";
  button.title = help;
  button.append(element("span", "control-label", label), element("span", "control-value", value));
  button.append(element("span", "control-caret", "▾"));
  button.addEventListener("click", () => {
    send({ type: "choose", control: name });
  });
  return button;
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

/** A commit id as shown: its first 7 characters. */
function short(oid: string): string {
  return oid.slice(0, 7);
}

/** The commit a branch left the base at, marked as a commit of the base. */
function fork(oid: string, branch: string): HTMLSpanElement {
  const node = element("span", "fork", short(oid));
  node.title = `${branch} left ${state.base} at ${oid}`;
  return node;
}

/** Text that opens something in VS Code when clicked. */
function link(className: string, text: string, open: () => FromPage): HTMLSpanElement {
  const node = element("span", `${className} link`, text);
  node.addEventListener("click", () => {
    send(open());
  });
  return node;
}

/** `path`'s last part, for a file named again next to its full path. */
function fileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

/** What a branch changed in `path`: from where it left the base to its tip. */
function fileDiff(branch: string, result: Containment, path: string, line?: number): FromPage {
  const title = `${fileName(path)} (${branch} since ${short(result.a.fork)})`;
  const left: Revision = { oid: result.a.fork, path };
  const right: Revision = { oid: result.a.oid, path };
  return line === undefined
    ? { type: "diff", left, right, title }
    : { type: "diff", left, right, title, line };
}

/**
A row for one of a file's items: its name, and where its lines are in the target. The name opens
the file's diff at the item; the arrow opens the item next to its counterpart in the target.
*/
function itemRow(branch: string, result: Containment, item: ItemShare): HTMLTableRowElement {
  const row = element("tr", "item");
  const name = element("td", "path");
  const label = link("item-name", item.item ?? "outside items", () =>
    fileDiff(branch, result, item.path, item.start),
  );
  label.title = `${item.kind ?? "lines outside items"}: show what ${branch} changed here`;
  name.append(label);
  const there = item.counterpart;
  if (there !== undefined) {
    const elsewhere = there.path === item.path ? "" : `${fileName(there.path)} `;
    const isSame = elsewhere === "" && there.item === item.item;
    const target = link(
      isSame ? "counterpart same" : "counterpart",
      `→ ${elsewhere}${there.item ?? "outside items"}`,
      () => ({
        type: "diff",
        left: { oid: result.a.oid, path: item.path },
        right: { oid: result.b.oid, path: there.path },
        title: `${item.item ?? fileName(item.path)} (${branch}) ↔ ${there.item ?? fileName(there.path)} (${state.target})`,
        ...(there.start !== undefined && { line: there.start }),
      }),
    );
    const where = there.item === undefined ? there.path : `${there.path}, ${there.item}`;
    target.title = `${String(there.lines)} of its found lines are in ${where}. Click to see them side by side.`;
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

function filesTable(branch: string, result: Containment): HTMLElement {
  const container = element("div", "detail");
  const ahead = result.a_in_b;
  const actions = element("div", "actions");
  const all = element("button", "control", "Show all changes");
  all.type = "button";
  all.title = `Every file ${branch} changed since it left ${state.base}, in one diff`;
  all.addEventListener("click", () => {
    send({
      type: "changes",
      title: `${branch} since ${short(result.a.fork)}`,
      changes: ahead.files.map((file) => ({
        path: file.path,
        from: result.a.fork,
        to: result.a.oid,
      })),
    });
  });
  actions.append(all);
  if (ahead.comment_lines !== undefined) {
    actions.append(
      element(
        "span",
        "note",
        `Without its ${String(ahead.comment_lines)} comment lines: ${percent(ahead.code_containment)} in ${state.target}.`,
      ),
    );
  }
  container.append(actions);
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
    );
    const path = element("td", "path");
    path.append(link("file-name", file.path, () => fileDiff(branch, result, file.path)));
    row.append(path);
    table.append(row);
    const own = items.filter((item) => item.path === file.path);
    table.append(...own.map((item) => itemRow(branch, result, item)));
  }
  container.append(table);
  return container;
}

/** "N lines since <commit>": the work, and where it starts. */
function lines(count: number, from: string, branch: string): HTMLSpanElement {
  const node = element("span", "lines", `${String(count)} lines since `);
  node.append(fork(from, branch));
  return node;
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
    lines(result.a.lines, result.a.fork, branch),
    bars(result),
    element("span", "percent", percent(result.a_in_b.containment)),
    element("span", "percent reverse-percent", percent(result.b_in_a.containment)),
  );
  block.append(row);
  if (state.expanded.has(branch)) {
    block.classList.add("open");
    block.append(filesTable(branch, result));
  }
  return block;
}

/** The median in the header: what it is of, and how it is found. */
function medianNote(middle: number | undefined, count: number): HTMLSpanElement {
  if (middle === undefined) {
    return element("span", "median-note", "");
  }
  const node = element("span", "median-note");
  node.append(
    element("span", "median-mark", "┆"),
    ` median ${percent(middle)} of ${String(count)} branches`,
  );
  node.title =
    `The dashed line: the middle of the ${String(count)} branches' bars (how much of each is in ` +
    `${state.target}); half are above it, half below. Unrelated branches still share some lines ` +
    `(license headers, common imports), so it is about the level that alone reaches.`;
  return node;
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
  const targetSide = entries.find(([, entry]) => entry.result)?.[1].result?.b;

  const header = element("header", "header stacked");
  const title = element("div", "heading");
  title.append(
    element("span", "title", `Containment in ${state.target}`),
    targetSide === undefined
      ? element("span", "detail", "")
      : lines(targetSide.lines, targetSide.fork, state.target),
    medianNote(middle, scores.length),
    element(
      "span",
      "count",
      state.isDone
        ? `${String(entries.length)} branches`
        : `${String(finished.length)} / ${String(entries.length)} compared`,
    ),
  );
  const toolbar = element("div", "toolbar");
  toolbar.append(
    control(
      "base",
      "Base",
      state.base,
      "The canonical branch: a branch's work is what it added since it left this one. Click to pick another.",
    ),
    control(
      "target",
      "Target",
      state.target,
      "The branch every other branch is compared with. Click to pick another.",
    ),
  );
  header.append(title, toolbar);
  const legend = element(
    "p",
    "legend",
    `Lines: what each branch added since it left ${state.base}, at the commit shown. ` +
      `Bar: how much of them is in ${state.target} (solid exact, light edited); ` +
      `thin bar: how much of ${state.target} is in the branch. Click a branch to inspect it.`,
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
