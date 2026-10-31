// The containment page: for each branch, how much of its work is in the target branch and how much
// of the target's is in it, filled in as results arrive. Runs in the webview; bundled to
// dist/containment.js.

import {
  compareResults,
  median,
  type Comments,
  type Containment,
  type ItemShare,
  type Meta,
  type Relation,
  type ToContainmentPage,
} from "../containment.js";
import type { BranchState } from "../git.js";
import type { Control, FromPage } from "../messages.js";
import { element, fileName, footer, info, movement, percent, work } from "./common.js";

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

const state: {
  target: string;
  base: string;
  comments: Comments;
  threshold: number;
  meta: Meta | undefined;
  states: Map<string, BranchState>;
  entries: Map<string, Entry>;
  expanded: Set<string>;
  isDone: boolean;
} = {
  target: "",
  base: "",
  comments: "ignored",
  threshold: 0.9,
  meta: undefined,
  states: new Map<string, BranchState>(),
  entries: new Map<string, Entry>(),
  expanded: new Set<string>(),
  isDone: false,
};

/** A toolbar button: `label` and the current `value`; a click asks the extension to change it. */
function control(name: Control, label: string, value: string, help: string): HTMLButtonElement {
  const button = element("button", "control");
  button.type = "button";
  button.title = help;
  const shown = element("span", "control-value", value);
  shown.title = value;
  button.append(element("span", "control-label", label), shown);
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

/** Text that asks the extension to inspect a branch when clicked. */
function link(className: string, text: string, open: () => FromPage): HTMLSpanElement {
  const node = element("span", `${className} link`, text);
  node.addEventListener("click", () => {
    send(open());
  });
  return node;
}

/** Inspect `branch`, at `path` and `line` if given. */
function inspect(branch: string, path?: string, line?: number): FromPage {
  return {
    type: "inspect",
    branch,
    ...(path !== undefined && { path }),
    ...(line !== undefined && { line }),
  };
}

/**
A row for one of a file's items: its name, and where its lines are in the target, marked if the
item was moved or renamed there. Both inspect the branch at the item.
*/
function itemRow(branch: string, item: ItemShare): HTMLTableRowElement {
  const row = element("tr", "item");
  const name = element("td", "path");
  const open = (): FromPage => inspect(branch, item.path, item.start);
  const label = link("item-name", item.item ?? "outside items", open);
  label.title = `${item.kind ?? "lines outside items"}: inspect its lines`;
  name.append(label);
  const there = item.counterpart;
  if (there !== undefined) {
    const elsewhere = there.path === item.path ? "" : `${fileName(there.path)} `;
    const moved = movement(item.path, item.item, there);
    const isSame = moved === undefined && there.item === item.item;
    const target = link(
      isSame ? "counterpart same" : "counterpart",
      `→ ${elsewhere}${there.item ?? "outside items"}`,
      open,
    );
    const where = there.item === undefined ? there.path : `${there.path}, ${there.item}`;
    target.title = `${String(there.lines)} of its found lines are in ${where} in ${state.target}. Click to inspect them.`;
    name.append(target);
    if (moved !== undefined) {
      name.append(element("span", "movement", moved));
    }
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
  const all = element("button", "control", "Inspect");
  all.type = "button";
  all.title = `Every line ${branch} added since it left ${state.base}, next to where it is in ${state.target}`;
  all.addEventListener("click", () => {
    send(inspect(branch));
  });
  actions.append(all);
  if (result.a.comment_lines > 0) {
    actions.append(
      element(
        "span",
        "note",
        state.comments === "ignored"
          ? `${String(result.a.comment_lines)} comment lines left out.`
          : `${String(result.a.comment_lines)} of its lines are comments, counted.`,
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
    path.append(link("file-name", file.path, () => inspect(branch, file.path)));
    row.append(path);
    table.append(row);
    const own = items.filter((item) => item.path === file.path);
    table.append(...own.map((item) => itemRow(branch, item)));
  }
  container.append(table);
  return container;
}

/** A small badge: `text`, coloured by `tone`, explained by `help`. */
function badge(tone: string, text: string, help: string): HTMLSpanElement {
  const node = element("span", `badge ${tone}`, text);
  node.title = help;
  return node;
}

/** The sync badge: remote only, local only, upstream gone, in sync, or ahead and behind. */
function syncBadge(branch: string, where: BranchState): HTMLSpanElement {
  if (where.isRemote) {
    return badge("muted", "remote", `Only on the remote: there is no local ${branch}.`);
  }
  if (where.upstream === undefined) {
    return badge("muted", "local", "Only local: not on any remote.");
  }
  if (where.isGone === true) {
    return badge("bad", "upstream gone", `${where.upstream} no longer exists.`);
  }
  const ahead = where.ahead ?? 0;
  const behind = where.behind ?? 0;
  if (ahead === 0 && behind === 0) {
    return badge("good", "in sync", `The same as ${where.upstream}.`);
  }
  const arrows = [ahead > 0 ? `↑${String(ahead)}` : "", behind > 0 ? `↓${String(behind)}` : ""];
  return badge(
    "warn",
    arrows.filter((arrow) => arrow !== "").join(" "),
    `${String(ahead)} commits not on ${where.upstream}; ${String(behind)} on ${where.upstream} not here.`,
  );
}

/** The work tree badges: checked out somewhere, and whether there are uncommitted changes. */
function worktreeBadges(where: BranchState): HTMLSpanElement[] {
  if (where.worktree === undefined) {
    return [];
  }
  const badges = [badge("muted", "worktree", `Checked out in ${where.worktree}`)];
  const changes = where.changes;
  if (changes !== undefined) {
    const total = changes.changed + changes.untracked;
    badges.push(
      total === 0
        ? badge("good dot", "●", "No uncommitted changes in its work tree.")
        : badge(
            "bad dot",
            "●",
            `Uncommitted: ${String(changes.changed)} changed, ${String(changes.untracked)} untracked files. Not part of the comparison: only commits are compared.`,
          ),
    );
  }
  return badges;
}

/** The branch's name with its badges. */
function nameCell(branch: string): HTMLSpanElement {
  const node = element("span", "name-cell");
  node.append(element("span", "name", branch));
  const where = state.states.get(branch);
  if (where !== undefined) {
    const badges = element("span", "badges");
    badges.append(syncBadge(branch, where), ...worktreeBadges(where));
    node.append(badges);
  }
  return node;
}

/** How many of the listed branches are local, and how many only on a remote. */
function branchCount(total: number): string {
  let remote = 0;
  for (const branch of state.states.values()) {
    if (branch.isRemote) {
      remote += 1;
    }
  }
  return state.states.size === 0
    ? `${String(total)} branches`
    : `${String(total)} branches: ${String(total - remote)} local, ${String(remote)} remote only`;
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
      nameCell(branch),
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
    nameCell(branch),
    work(result.a, branch, state.base),
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
      : work(targetSide, state.target, state.base),
    medianNote(middle, scores.length),
    element(
      "span",
      "count",
      state.isDone
        ? branchCount(entries.length)
        : `${String(finished.length)} / ${String(entries.length)} compared`,
    ),
  );
  const similar = percent(state.threshold);
  const oneIn = String(Math.round(1 / (1 - state.threshold)));
  const fuzzyHelp =
    `A line of a branch counts as found if the target has the same line (exact, solid bar), or ` +
    `a line at least ${similar} similar (edited, light bar): about one character in ${oneIn} ` +
    `may differ (normalized Levenshtein distance). Whitespace is ignored.`;
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
    control(
      "comments",
      "Comments",
      state.comments,
      state.comments === "ignored"
        ? "Lines that are only a comment (license headers, doc comments) in Python, Rust and C files are not counted as work. Click to count them, and compare again."
        : "Lines that are only a comment are counted as work. Click to leave them out, and compare again.",
    ),
    info(`edited lines: ≥ ${similar} similar`, fuzzyHelp),
    info(
      "commits only",
      "Only committed work is compared: uncommitted changes in a work tree are not checked.",
    ),
  );
  header.append(title, toolbar);
  const legend = element(
    "p",
    "legend",
    `Lines: what each branch added since it left ${state.base}, at the commit shown` +
      (state.comments === "ignored" ? ", comments left out. " : ". ") +
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
  root.replaceChildren(header, legend, list, footer(state.meta));
}

function receive(message: ToContainmentPage): void {
  switch (message.type) {
    case "start": {
      state.target = message.target;
      state.base = message.base;
      state.comments = message.comments;
      state.threshold = message.threshold;
      state.meta = message.meta;
      state.states = new Map((message.states ?? []).map((branch) => [branch.name, branch]));
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
