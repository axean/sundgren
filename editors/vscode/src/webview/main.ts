// The history page: a vertical map of branches around the tip of a base branch, newest on top,
// drawn from the grid of a sundgren.graph/3 document as SVG, beside a table of the commits.
// Runs in the webview (a browser); bundled to dist/webview.js.

import type { Meta } from "../containment.js";
import type { BranchState } from "../git.js";
import {
  isShowMode,
  parseGraph,
  type Cell,
  type Commit,
  type Graph,
  type Lane,
  type ShowMode,
} from "../graph.js";
import {
  MAX_BRANCHES,
  type FromPage,
  type Selection,
  type ToPage,
  type View,
} from "../messages.js";
import {
  controlButton,
  element,
  footer,
  percent,
  short,
  syncBadge,
  worktreeBadges,
} from "./common.js";
import { formatTime } from "./format.js";
import {
  cellX,
  centre,
  GUTTER,
  lanePaths,
  layout,
  NODE_RADIUS,
  vias,
  type Layout,
} from "./geometry.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const SVG = "http://www.w3.org/2000/svg";
/** Lane colours in history.css; families past these are drawn dotted. */
const FAMILIES = 12;
/** Width of one character of the refs column's font (12px JetBrains Mono). */
const CH = 7.2;
/** Lane names get this many characters at least, and at most. */
const NAME_MIN = 8;
const NAME_MAX = 24;
/** The label's other columns, in characters: ↑↓ counts, where it is, the rest. */
const COUNTS_CH = 9;
const WHERE_CH = 8;
const EXTRA_CH = 19;
const LABEL_GAPS = 3 * 8;

const SHOW_TEXT: Readonly<Record<Exclude<ShowMode, "like">, string>> = {
  local: "Local first",
  recent: "Recent activity",
  mine: "Mine",
};

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

const state: {
  base: string;
  view: View;
  /** Base commits each "Load older" adds. */
  step: number;
  meta: Meta | undefined;
  states: Map<string, BranchState>;
  graph: Graph | undefined;
  isRunning: boolean;
  progress: string;
  error: string | undefined;
  /** What was clicked last: `commit:<oid>` or `lane:<name>`. */
  selected: string | undefined;
  /** Folds shown unfolded, by their newest commit. */
  unfolded: Set<string>;
  /** Widths only grow for one map, so the columns don't move when more is loaded. */
  graphWidth: number;
  nameChars: number;
} = {
  base: "",
  view: { show: "local", branches: 12, commits: 50 },
  step: 50,
  meta: undefined,
  states: new Map<string, BranchState>(),
  graph: undefined,
  isRunning: false,
  progress: "",
  error: undefined,
  selected: undefined,
  unfolded: new Set<string>(),
  graphWidth: 0,
  nameChars: NAME_MIN,
};

function svgElement<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attributes: Readonly<Record<string, string | number>>,
): SVGElementTagNameMap[K] {
  const node = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attributes)) {
    node.setAttribute(name, String(value));
  }
  return node;
}

function svgTitle(text: string): SVGTitleElement {
  const title = svgElement("title", {});
  title.textContent = text;
  return title;
}

/** CSS classes for a lane's colour and line style. */
function laneClasses(lane: Lane): string[] {
  const classes = [lane.kind === "remote" ? "remote" : "local"];
  if (lane.protected || lane.family === undefined) {
    classes.push("protected");
  } else {
    classes.push(`family-${String(lane.family % FAMILIES)}`);
    if (lane.family >= FAMILIES) {
      classes.push("dotted");
    }
  }
  if (lane.merged) {
    classes.push("merged");
  }
  return classes;
}

function selectionKey(selection: Selection): string {
  return selection.kind === "commit" ? `commit:${selection.oid}` : `lane:${selection.name}`;
}

function plural(count: number, word: string): string {
  return `${String(count)} ${word}${count === 1 ? "" : "s"}`;
}

/** One line of what a lane is, for hover text. */
function laneSummary(lane: Lane, graph: Graph): string {
  const base = graph.base.name;
  const what =
    lane.kind === "reference"
      ? "the base, the line every other lane is placed against."
      : `${plural(lane.ahead, "commit")} not in ${base}, ${String(lane.behind)} of ${base} not in it.`;
  const parts = [`${lane.name}: ${what}`];
  if (lane.remote !== undefined) {
    parts.push(`${lane.remote} is at the same commit.`);
  }
  if (lane.merged) {
    parts.push("Merged: nothing ahead of the base.");
  }
  if (lane.only === "remote") {
    parts.push("Only on the remote.");
  }
  const like = graph.show.like?.name;
  if (like !== undefined && lane.score !== undefined) {
    parts.push(`${percent(lane.score)} of its work is in ${like}.`);
  }
  if (lane.before !== undefined) {
    parts.push(`It goes on below the page: ${plural(lane.before, "older commit")} of its own.`);
  }
  return parts.join(" ");
}

function commitSummary(commit: Commit): string {
  return [
    commit.subject,
    `${short(commit.oid)} · ${commit.author} · ${formatTime(commit.committed)}`,
    commit.parents.length > 1 ? `merge of ${String(commit.parents.length)} parents` : "",
    commit.refs.length > 0 ? commit.refs.join(", ") : "",
  ]
    .filter((line) => line !== "")
    .join("\n");
}

/** The folds drawn folded, as indices into `graph.folds`. */
function foldedIndices(graph: Graph): Set<number> {
  const folded = new Set<number>();
  for (const [index, fold] of graph.folds.entries()) {
    const head = graph.commits[fold.commits[0] ?? -1];
    if (head !== undefined && !state.unfolded.has(head.oid)) {
      folded.add(index);
    }
  }
  return folded;
}

/** The fold starting at each row. */
function foldStarts(graph: Graph): Map<number, number> {
  return new Map(graph.folds.map((fold, index) => [fold.rows[0], index]));
}

/** A commit: a dot on the reference lane, a ring elsewhere, two rings for a folded run. */
function commitNode(
  graph: Graph,
  cell: Cell,
  placed: Layout,
  fold: { index: number; isFolded: boolean } | undefined,
): SVGGElement | undefined {
  const commit = cell.commit === undefined ? undefined : graph.commits[cell.commit];
  const lane = graph.lanes[cell.lane];
  if (commit === undefined || lane === undefined) {
    return undefined;
  }
  const { x, y } = centre(cell.row, cell.col, placed);
  const node = svgElement("g", {
    class: ["commit", ...laneClasses(lane)].join(" "),
    tabindex: 0,
    role: "button",
    "aria-label": `Commit ${short(commit.oid)} on ${lane.name}: ${commit.subject}`,
  });
  node.dataset["oid"] = commit.oid;
  node.dataset["lane"] = String(cell.lane);
  node.append(svgElement("circle", { class: "focus", cx: x, cy: y, r: NODE_RADIUS + 3 }));
  if (fold?.isFolded === true) {
    node.dataset["fold"] = String(fold.index);
    node.append(svgElement("circle", { class: "node stack", cx: x, cy: y, r: NODE_RADIUS + 2.5 }));
  }
  node.append(
    svgElement("circle", {
      class: lane.kind === "reference" ? "node filled" : "node",
      cx: x,
      cy: y,
      r: NODE_RADIUS,
    }),
    svgTitle(commitSummary(commit)),
  );
  return node;
}

/** An off-board connector: the lane goes on in older history. */
function connectorNode(graph: Graph, cell: Cell, placed: Layout): SVGGElement | undefined {
  const lane = graph.lanes[cell.lane];
  if (lane === undefined) {
    return undefined;
  }
  const { x, y } = centre(cell.row, cell.col, placed);
  const node = svgElement("g", { class: ["connector", ...laneClasses(lane)].join(" ") });
  node.dataset["lane"] = String(cell.lane);
  node.append(
    svgElement("circle", { class: "pad", cx: x, cy: y, r: NODE_RADIUS }),
    svgElement("circle", { class: "pad-hole", cx: x, cy: y, r: 1.5 }),
    svgTitle(
      lane.before === undefined
        ? `${lane.name} goes on in older history.`
        : `${lane.name} goes on in older history: ${plural(lane.before, "commit")} of its own.`,
    ),
  );
  return node;
}

function drawGraph(graph: Graph, placed: Layout, width: number): SVGSVGElement {
  const svg = svgElement("svg", {
    class: "map-svg",
    width,
    height: placed.height,
    role: "group",
    "aria-label": `Branches around ${graph.base.name}`,
  });
  const traces = lanePaths(graph.grid.cells, placed);
  for (const [index, lane] of graph.lanes.entries()) {
    const d = traces.get(index);
    if (d === undefined) {
      continue;
    }
    const group = svgElement("g", { class: ["lane", ...laneClasses(lane)].join(" ") });
    group.dataset["lane"] = String(index);
    group.append(
      svgElement("path", { class: "hit", d }),
      svgElement("path", { class: "trace", d }),
      svgTitle(laneSummary(lane, graph)),
    );
    svg.append(group);
  }
  const folded = foldedIndices(graph);
  const starts = foldStarts(graph);
  const nodes = svgElement("g", { class: "nodes" });
  for (const cell of graph.grid.cells) {
    if (placed.h(cell.row) === 0) {
      continue;
    }
    let node: SVGGElement | undefined;
    if (cell.node === "commit") {
      const index = starts.get(cell.row);
      const fold =
        index === undefined || graph.folds.at(index)?.lane !== cell.lane
          ? undefined
          : { index, isFolded: folded.has(index) };
      node = commitNode(graph, cell, placed, fold);
    } else if (cell.node === "connector") {
      node = connectorNode(graph, cell, placed);
    }
    if (node !== undefined) {
      nodes.append(node);
    }
  }
  svg.append(nodes);
  return svg;
}

/** Where a lane is: base, local, remote, or in sync with its remote (`= origin`). */
function where(lane: Lane): { text: string; help: string } {
  if (lane.remote !== undefined) {
    const remote = lane.remote.split("/", 1)[0] ?? lane.remote;
    return {
      text: `= ${remote}`,
      help: `${lane.remote} is at the same commit: one lane for both.`,
    };
  }
  if (lane.kind === "reference") {
    return { text: "base", help: "The base: every other lane is placed against it." };
  }
  if (lane.kind === "remote") {
    const help =
      lane.only === "remote"
        ? "Only on the remote: there is no local branch."
        : "The remote branch: its commits the local one doesn't have.";
    return { text: "remote", help };
  }
  const help = lane.only === "local" ? "Only local: not on any remote." : "The local branch.";
  return { text: "local", help };
}

/** The lane's label: name, ↑ahead ↓behind, where it is, and the rest; in aligned columns. */
function laneLabel(graph: Graph, lane: Lane, index: number): HTMLDivElement {
  const label = element("div", ["lane-label", ...laneClasses(lane)].join(" "));
  label.dataset["lane"] = String(index);
  label.tabIndex = 0;
  label.setAttribute("role", "button");
  label.title = laneSummary(lane, graph);
  const name = element("span", "lane-name", lane.name);
  const counts = element(
    "span",
    "lane-counts",
    lane.kind === "reference" ? "" : `↑${String(lane.ahead)} ↓${String(lane.behind)}`,
  );
  const place = where(lane);
  const kind = element("span", "lane-where", place.text);
  kind.title = place.help;
  const extra = element("span", "lane-extra");
  const facts: string[] = [];
  if (lane.merged) {
    facts.push("merged");
  }
  if (lane.score !== undefined && graph.show.like !== undefined) {
    facts.push(percent(lane.score));
  }
  if (lane.before !== undefined) {
    facts.push(`${String(lane.before)} older`);
  }
  if (facts.length > 0) {
    extra.append(element("span", "lane-facts", facts.join(" · ")));
  }
  const branch = state.states.get(lane.name);
  if (branch !== undefined) {
    // Upstream drift only: in sync, local and remote are said by the `where` column.
    const isDrift = lane.remote === undefined && !branch.isRemote && branch.upstream !== undefined;
    extra.append(...(isDrift ? [syncBadge(lane.name, branch)] : []), ...worktreeBadges(branch));
  }
  label.append(name, counts, kind, extra);
  return label;
}

/** Refs on a commit that aren't drawn as lanes: tags, a remote that is only behind. */
function refChips(graph: Graph, commit: Commit): HTMLSpanElement[] {
  const drawn = new Set(graph.lanes.flatMap((lane) => [lane.name, lane.remote ?? lane.name]));
  return commit.refs
    .filter((name) => !drawn.has(name))
    .map((name) => {
      const chip = element("span", "ref-chip", name);
      chip.title = name;
      return chip;
    });
}

/** Authors of `commits`, the first and how many others. */
function authors(commits: readonly Commit[]): string {
  const names = [...new Set(commits.map((commit) => commit.author))];
  return names.length <= 1 ? (names[0] ?? "") : `${names[0] ?? ""} +${String(names.length - 1)}`;
}

/** The fold toggle on a fold's first row: `×7 ▸` folded, `×7 ▾` open. */
function foldButton(count: number, index: number, isFolded: boolean): HTMLButtonElement {
  const button = element("button", "fold-toggle", `×${String(count)} ${isFolded ? "▸" : "▾"}`);
  button.type = "button";
  button.dataset["fold"] = String(index);
  button.title = isFolded
    ? `${plural(count, "commit")} on one lane, folded. Click to show them.`
    : "Click to fold these commits into one row.";
  return button;
}

/** A table row: the refs and labels on `row`, and its commit's subject, author and date. */
function tableRow(
  graph: Graph,
  placed: Layout,
  row: number,
  commitAt: ReadonlyMap<number, Commit>,
): HTMLDivElement {
  const node = element("div", "hrow");
  node.style.top = `${String(placed.y(row))}px`;
  node.style.height = `${String(placed.h(row))}px`;
  const refs = element("span", "refs");
  const labelled = placed.labels.get(row) ?? [];
  for (const index of labelled) {
    const lane = graph.lanes.at(index);
    if (lane !== undefined) {
      refs.append(laneLabel(graph, lane, index));
    }
  }
  const subject = element("span", "subject");
  const author = element("span", "author");
  const date = element("span", "date");
  const commit = commitAt.get(row);
  if (commit !== undefined) {
    node.dataset["oid"] = commit.oid;
    node.title = commitSummary(commit);
    const index = foldStarts(graph).get(row);
    const fold = index === undefined ? undefined : graph.folds.at(index);
    const runs =
      fold?.commits.map((at) => graph.commits.at(at)).filter((item) => item !== undefined) ?? [];
    const isFolded = index !== undefined && foldedIndices(graph).has(index);
    if (index !== undefined && runs.length > 0) {
      subject.append(foldButton(runs.length, index, isFolded));
    }
    subject.append(...refChips(graph, commit), element("span", "subject-text", commit.subject));
    author.textContent = isFolded ? authors(runs) : commit.author;
    date.textContent = formatTime(commit.committed);
    const oldest = runs.at(-1);
    if (isFolded && oldest !== undefined) {
      date.title = `${formatTime(oldest.committed)} to ${formatTime(commit.committed)}`;
    }
  }
  node.append(element("span", "graph-cell"), refs, subject, author, date);
  return node;
}

/** The map: the SVG graph, and a table row per commit or label row beside it. */
function drawMap(graph: Graph): HTMLElement {
  const placed = layout(graph, foldedIndices(graph));
  const longest = Math.max(0, ...graph.lanes.map((lane) => lane.name.length));
  state.nameChars = Math.max(state.nameChars, Math.min(NAME_MAX, longest));
  state.graphWidth = Math.max(state.graphWidth, cellX(graph.grid.width) + GUTTER);
  const refsWidth = (state.nameChars + COUNTS_CH + WHERE_CH + EXTRA_CH) * CH + LABEL_GAPS;
  const map = element("div", "map");
  map.style.setProperty("--graph-width", `${String(state.graphWidth)}px`);
  map.style.setProperty("--refs-width", `${String(Math.round(refsWidth))}px`);
  map.style.setProperty("--name-width", `${String(Math.round(state.nameChars * CH))}px`);
  map.style.height = `${String(placed.height)}px`;
  const commitAt = new Map(graph.commits.map((commit) => [commit.row, commit]));
  map.append(
    drawGraph(graph, placed, state.graphWidth),
    ...placed.rows.map((row) => tableRow(graph, placed, row, commitAt)),
  );
  return map;
}

/** Mark the elements of what was selected. */
function showSelection(root: Element): void {
  for (const node of root.querySelectorAll(".selected")) {
    node.classList.remove("selected");
  }
  const graph = state.graph;
  const selected = state.selected;
  if (graph === undefined || selected === undefined) {
    return;
  }
  if (selected.startsWith("commit:")) {
    const oid = selected.slice("commit:".length);
    for (const node of root.querySelectorAll(`[data-oid="${CSS.escape(oid)}"]`)) {
      node.classList.add("selected");
    }
    return;
  }
  const index = graph.lanes.findIndex((lane) => `lane:${lane.name}` === selected);
  for (const node of root.querySelectorAll(`[data-lane="${CSS.escape(String(index))}"]`)) {
    node.classList.add("selected");
  }
}

function datasetOf(target: Element | null): DOMStringMap | undefined {
  return target instanceof HTMLElement || target instanceof SVGElement ? target.dataset : undefined;
}

/** What a click or key on `target` selects, if anything: a lane label, else a commit. */
function selectionAt(target: EventTarget | null | undefined): Selection | undefined {
  if (!(target instanceof Element) || state.graph === undefined) {
    return undefined;
  }
  const index = datasetOf(target.closest(".lane-label, .lane"))?.["lane"];
  const lane = index === undefined ? undefined : state.graph.lanes[Number(index)];
  if (lane !== undefined) {
    return { kind: "lane", name: lane.name };
  }
  const oid = datasetOf(target.closest("[data-oid]"))?.["oid"];
  return oid === undefined ? undefined : { kind: "commit", oid };
}

function select(selection: Selection, root: Element): void {
  state.selected = selectionKey(selection);
  showSelection(root);
  send({ type: "select", selection });
}

/** Fold or unfold the fold under `target`; whether there was one. */
function didToggleFold(target: EventTarget | null): boolean {
  const graph = state.graph;
  const index =
    target instanceof Element ? datasetOf(target.closest("[data-fold]"))?.["fold"] : undefined;
  const fold = index === undefined ? undefined : graph?.folds[Number(index)];
  const head = fold === undefined ? undefined : graph?.commits[fold.commits[0] ?? -1];
  if (head === undefined) {
    return false;
  }
  if (state.unfolded.has(head.oid)) {
    state.unfolded.delete(head.oid);
  } else {
    state.unfolded.add(head.oid);
  }
  render();
  return true;
}

/** Light up every element of the lane under `target`, or none. */
function highlight(root: Element, target: EventTarget | null | undefined): void {
  const index =
    target instanceof Element ? datasetOf(target.closest("[data-lane]"))?.["lane"] : undefined;
  for (const node of root.querySelectorAll(".hot")) {
    node.classList.remove("hot");
  }
  if (index !== undefined) {
    for (const node of root.querySelectorAll(`[data-lane="${CSS.escape(index)}"]`)) {
      node.classList.add("hot");
    }
  }
}

function listen(map: HTMLElement): void {
  map.addEventListener("click", (event) => {
    if (didToggleFold(event.target)) {
      return;
    }
    const selection = selectionAt(event.target);
    if (selection !== undefined) {
      select(selection, map);
    }
  });
  map.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }
    if (event.target instanceof HTMLButtonElement) {
      return;
    }
    const selection = selectionAt(event.target);
    if (selection === undefined) {
      return;
    }
    event.preventDefault();
    select(selection, map);
  });
  for (const type of ["pointerover", "focusin"] as const) {
    map.addEventListener(type, (event) => {
      highlight(map, event.target);
    });
  }
  for (const type of ["pointerleave", "focusout"] as const) {
    map.addEventListener(type, () => {
      highlight(map, undefined);
    });
  }
}

/** A short line in the legend's own symbol. */
function swatch(
  classes: readonly string[],
  draw: "line" | "dot" | "ring" | "stack",
): SVGSVGElement {
  const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
  const group = svgElement("g", { class: ["lane", ...classes].join(" ") });
  group.append(svgElement("path", { class: "trace", d: "M3 7H25" }));
  if (draw === "stack") {
    group.append(
      svgElement("circle", { class: "node stack", cx: 14, cy: 7, r: NODE_RADIUS + 2.5 }),
    );
  }
  if (draw !== "line") {
    group.append(
      svgElement("circle", {
        class: draw === "dot" ? "node filled" : "node",
        cx: 14,
        cy: 7,
        r: NODE_RADIUS,
      }),
    );
  }
  svg.append(group);
  return svg;
}

/** The via symbol: a lane broken under a line passing over it. */
function viaSwatch(): SVGSVGElement {
  const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
  const under = svgElement("g", { class: "lane protected" });
  under.append(svgElement("path", { class: "trace", d: "M14 0V4M14 10V14" }));
  const over = svgElement("g", { class: "lane family-0" });
  over.append(svgElement("path", { class: "trace", d: "M2 7H26" }));
  svg.append(under, over);
  return svg;
}

function connectorSwatch(): SVGSVGElement {
  const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
  const group = svgElement("g", { class: "connector protected" });
  group.append(
    svgElement("path", { class: "trace", d: "M14 0V7" }),
    svgElement("circle", { class: "pad", cx: 14, cy: 7, r: NODE_RADIUS }),
    svgElement("circle", { class: "pad-hole", cx: 14, cy: 7, r: 1.5 }),
  );
  svg.append(group);
  return svg;
}

function legendItem(symbol: SVGSVGElement | HTMLElement, text: string): HTMLSpanElement {
  const item = element("span", "legend-item");
  item.append(symbol, element("span", "", text));
  return item;
}

/** What every marking on the map means: only those it uses. */
function legend(graph: Graph): HTMLElement {
  const node = element("div", "map-legend");
  const isUsed = (hasIt: (lane: Lane) => boolean): boolean =>
    graph.lanes.some((lane) => hasIt(lane));
  const protectedNames = graph.lanes.filter((lane) => lane.protected || lane.family === undefined);
  if (protectedNames.length > 0) {
    node.append(
      legendItem(
        swatch(["protected"], "line"),
        `protected: ${protectedNames.map((lane) => lane.name).join(", ")}`,
      ),
    );
  }
  node.append(legendItem(swatch(["family-4"], "line"), "local"));
  if (isUsed((lane) => lane.kind === "remote")) {
    node.append(legendItem(swatch(["family-4", "remote"], "line"), "remote"));
  }
  if (isUsed((lane) => lane.remote !== undefined)) {
    node.append(legendItem(element("span", "legend-mark", "="), "remote at the same commit"));
  }
  if (isUsed((lane) => lane.merged)) {
    node.append(legendItem(swatch(["family-4", "merged"], "line"), "merged"));
  }
  if (isUsed((lane) => (lane.family ?? 0) >= FAMILIES)) {
    node.append(legendItem(swatch(["family-4", "dotted"], "line"), "colour used again"));
  }
  node.append(
    legendItem(swatch(["protected"], "dot"), `commit of ${graph.base.name}`),
    legendItem(swatch(["family-4"], "ring"), "commit"),
  );
  if (graph.folds.length > 0) {
    node.append(legendItem(swatch(["family-4"], "stack"), "folded run: click ×N"));
  }
  if (vias(graph.grid.cells).length > 0) {
    node.append(legendItem(viaSwatch(), "passes under, not joined"));
  }
  if (graph.grid.cells.some((cell) => cell.node === "connector")) {
    node.append(legendItem(connectorSwatch(), "goes on in older history"));
  }
  return node;
}

/** Ask the extension for `view`. */
function ask(view: View): void {
  state.view = view;
  send({ type: "view", ...view });
}

function showSelect(): HTMLLabelElement {
  const label = element("label", "control");
  label.title = "Which branches to show besides the base.";
  const select = element("select", "control-select");
  const option = (value: string, text: string): void => {
    const item = element("option", "", text);
    item.value = value;
    item.selected = value === state.view.show;
    select.append(item);
  };
  for (const [value, text] of Object.entries(SHOW_TEXT)) {
    option(value, text);
  }
  if (state.view.like !== undefined) {
    option("like", `Like ${state.view.like}`);
  }
  option("pick", "Like a branch…");
  select.addEventListener("change", () => {
    const value = select.value;
    if (value === "pick") {
      // The extension asks which branch, then runs the map again.
      select.value = state.view.show;
      send({ type: "choose", control: "like" });
    } else if (value !== "like" && isShowMode(value)) {
      ask({ show: value, branches: state.view.branches, commits: state.view.commits });
    } else if (state.view.like !== undefined) {
      ask({ ...state.view, show: "like" });
    }
  });
  label.append(element("span", "control-label", "Show"), select);
  return label;
}

function branchesSelect(): HTMLLabelElement {
  const label = element("label", "control");
  label.title = "How many branches to show besides the base.";
  const select = element("select", "control-select");
  for (let count = 0; count <= MAX_BRANCHES; count++) {
    const option = element("option", "", String(count));
    option.value = String(count);
    option.selected = count === state.view.branches;
    select.append(option);
  }
  select.addEventListener("change", () => {
    ask({ ...state.view, branches: Number(select.value) });
  });
  label.append(element("span", "control-label", "Branches"), select);
  return label;
}

function header(): HTMLElement {
  const graph = state.graph;
  const node = element("header", "header stacked");
  const heading = element("div", "heading");
  heading.append(element("span", "title", `History around ${state.base}`));
  if (graph !== undefined) {
    const others = new Set(graph.lanes.filter((lane) => !lane.pinned).map((lane) => lane.branch))
      .size;
    heading.append(
      element(
        "span",
        "count",
        `${String(others)} of ${String(graph.candidates)} branches · ${plural(graph.commits.length, "commit")}`,
      ),
    );
  }
  if (state.isRunning) {
    const progress = state.progress === "" ? "" : `: ${state.progress}`;
    heading.append(element("span", "status", `reading${progress}`));
  }
  const toolbar = element("div", "toolbar");
  toolbar.append(
    controlButton(
      "Base",
      state.base,
      "The reference line: every branch is placed against it. Click to pick another.",
      () => {
        send({ type: "choose", control: "base" });
      },
    ),
    showSelect(),
    branchesSelect(),
  );
  node.append(heading, toolbar);
  return node;
}

/** "Load older": the same view with more of the base's history. */
function olderButton(graph: Graph): HTMLButtonElement {
  const button = element("button", "control older", `Load older (${String(state.step)} more)`);
  button.type = "button";
  button.title = `Read ${String(state.step)} more commits of ${graph.base.name}, and the branches' commits in that time.`;
  button.disabled = state.isRunning;
  button.addEventListener("click", () => {
    ask({ ...state.view, commits: graph.page.commits + state.step });
    render();
  });
  return button;
}

function render(): void {
  const root = document.querySelector("#root");
  if (root === null) {
    return;
  }
  // Older history comes in below, so the same scroll position shows the same commits.
  const scroll = scrollY;
  const parts: HTMLElement[] = [header()];
  const graph = state.graph;
  if (state.error !== undefined) {
    parts.push(element("p", "error", state.error));
  } else if (graph === undefined) {
    parts.push(element("p", "empty", state.isRunning ? "Reading the history…" : ""));
  } else {
    const map = drawMap(graph);
    map.classList.toggle("updating", state.isRunning);
    listen(map);
    showSelection(map);
    parts.push(map);
    if (graph.page.more) {
      parts.push(olderButton(graph));
    }
    parts.push(legend(graph));
  }
  parts.push(footer(state.meta));
  root.replaceChildren(...parts);
  scrollTo(0, scroll);
}

function receive(message: ToPage): void {
  switch (message.type) {
    case "setup": {
      state.base = message.base;
      state.view = message.view;
      state.step = message.step;
      state.meta = message.meta;
      state.states = new Map(message.states.map((branch) => [branch.name, branch]));
      state.graph = undefined;
      state.isRunning = false;
      state.error = undefined;
      state.selected = undefined;
      state.unfolded = new Set<string>();
      state.graphWidth = 0;
      state.nameChars = NAME_MIN;
      break;
    }
    case "running": {
      state.isRunning = true;
      state.progress = message.progress ?? "";
      state.view = message.view ?? state.view;
      break;
    }
    case "graph": {
      // Parsed here too: the docs demo posts sundgren's JSON as it is.
      const graph = parseGraph(message.graph);
      state.isRunning = false;
      state.progress = "";
      state.graph = graph;
      state.error = graph === undefined ? "This isn't a sundgren.graph/3 document." : undefined;
      break;
    }
    case "error": {
      state.isRunning = false;
      state.progress = "";
      state.error = message.message;
      break;
    }
  }
  render();
  if (message.type === "graph" && state.graph !== undefined) {
    send({ type: "rendered", oid: state.graph.base.oid, rows: state.graph.lanes.length });
  }
}

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToPage>) => {
  if (event.origin !== origin) {
    return;
  }
  receive(event.data);
});

send({ type: "ready" });
