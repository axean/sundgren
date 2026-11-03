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
import { type FromPage, type Selection, type ToPage, type View } from "../messages.js";
import {
  controlButton,
  element,
  footer,
  percent,
  repositoryControl,
  short,
  syncBadge,
  worktreeBadges,
} from "./common.js";
import { authorsOf, calendar, groups, lastDays, wholeDays } from "./filters.js";
import { clock, dayKey, dayLabel, formatTime } from "./format.js";
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
const EXTRA_CH = 26;
const LABEL_GAPS = 3 * 8;
/** Width of one character of the time axis' font (11px JetBrains Mono). */
const AXIS_CH = 6.6;
/** The time axis: room for the tick and the gaps. */
const AXIS_EXTRA = 22;
/** How long the branch list waits for more ticks before it asks for the map. */
const TICK_WAIT_MS = 350;

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
  repository: string | undefined;
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
  axisWidth: number;
  /** The list open over the page, and what each list is filtered by. */
  popover: Popover | undefined;
  queries: Map<Popover, string>;
  /** Authors (emails) whose commits are thinned out. */
  hiddenAuthors: Set<string>;
  /** The date filter's calendar: the month shown, and the first day picked. */
  calendar: { month: number; pending: number | undefined };
} = {
  repository: undefined,
  base: "",
  view: { show: "local", branches: 12, commits: 50, pins: [], hides: [] },
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
  axisWidth: 0,
  popover: undefined,
  queries: new Map<Popover, string>(),
  hiddenAuthors: new Set<string>(),
  calendar: { month: 0, pending: undefined },
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

/** Hover text of a commit: its whole message, then who, when and what. */
function commitSummary(commit: Commit): string {
  return [
    commit.subject,
    commit.body === "" ? "" : `\n${commit.body}\n`,
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

/** The commit node of `cell`, if it has one: with its fold, thin if filtered out. */
function commitAt(
  graph: Graph,
  cell: Cell,
  placed: Layout,
  rows: {
    folded: ReadonlySet<number>;
    starts: ReadonlyMap<number, number>;
    thin: ReadonlySet<number>;
  },
): SVGGElement | undefined {
  if (cell.node !== "commit") {
    return undefined;
  }
  const index = rows.starts.get(cell.row);
  const fold =
    index === undefined || graph.folds.at(index)?.lane !== cell.lane
      ? undefined
      : { index, isFolded: rows.folded.has(index) };
  const node = commitNode(graph, cell, placed, fold);
  if (rows.thin.has(cell.row) && !placed.labels.has(cell.row)) {
    node?.classList.add("thin");
  }
  return node;
}

function drawGraph(
  graph: Graph,
  placed: Layout,
  width: number,
  thin: ReadonlySet<number>,
): SVGSVGElement {
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
    const node =
      cell.node === "connector"
        ? connectorNode(graph, cell, placed)
        : commitAt(graph, cell, placed, { folded, starts, thin });
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

/**
A table row: the time axis (`day` labels the first row of a day), the refs and labels on `row`,
and its commit's hash, subject and author.
*/
function tableRow(
  graph: Graph,
  placed: Layout,
  row: number,
  commitAt: ReadonlyMap<number, Commit>,
  day: string | undefined,
): HTMLDivElement {
  const node = element("div", day === undefined ? "hrow" : "hrow day-start");
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
  const axis = element("span", "axis");
  const hash = element("span", "hash");
  const subject = element("span", "subject");
  const author = element("span", "author");
  const commit = commitAt.get(row);
  if (commit !== undefined) {
    node.dataset["oid"] = commit.oid;
    // Kept tall for a label, but by an author filtered out.
    node.classList.toggle("filtered", state.hiddenAuthors.has(commit.email));
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
    hash.textContent = short(commit.oid);
    axisTick(axis, commit, day, isFolded ? runs.at(-1) : undefined);
  }
  node.append(axis, element("span", "graph-cell"), refs, hash, subject, author);
  return node;
}

/** The time axis at a commit: a tick and the time, with the day where one starts. */
function axisTick(
  axis: HTMLSpanElement,
  commit: Commit,
  day: string | undefined,
  oldest: Commit | undefined,
): void {
  axis.classList.add(day === undefined ? "tick" : "day");
  if (day !== undefined) {
    axis.append(element("span", "axis-day", day));
  }
  axis.append(element("span", "axis-time", clock(commit.committed)));
  const time = formatTime(commit.committed);
  axis.title = oldest === undefined ? time : `${formatTime(oldest.committed)} to ${time}`;
}

/** The rows that start a day, top down, with the day's label. */
function dayStarts(placed: Layout, commitAt: ReadonlyMap<number, Commit>): Map<number, string> {
  const year = new Date().getFullYear();
  const starts = new Map<number, string>();
  let last = "";
  for (const row of placed.rows) {
    const commit = commitAt.get(row);
    if (commit === undefined) {
      continue;
    }
    const key = dayKey(commit.committed);
    if (key === last) {
      continue;
    }
    starts.set(row, dayLabel(commit.committed, year));
    last = key;
  }
  return starts;
}

/** The map: the SVG graph, and a table row per commit or label row beside it. */
/** Rows of commits by hidden authors (for a folded run, all of it by them). */
function thinRows(graph: Graph): Set<number> {
  const thin = new Set<number>();
  if (state.hiddenAuthors.size === 0) {
    return thin;
  }
  const folded = foldedIndices(graph);
  const starts = foldStarts(graph);
  for (const commit of graph.commits) {
    const index = starts.get(commit.row);
    const fold = index === undefined || !folded.has(index) ? undefined : graph.folds.at(index);
    const run = fold?.commits.map((at) => graph.commits.at(at)) ?? [commit];
    if (run.every((each) => each !== undefined && state.hiddenAuthors.has(each.email))) {
      thin.add(commit.row);
    }
  }
  return thin;
}

function drawMap(graph: Graph): HTMLElement {
  const thin = thinRows(graph);
  const placed = layout(graph, foldedIndices(graph), thin);
  const longest = Math.max(0, ...graph.lanes.map((lane) => lane.name.length));
  state.nameChars = Math.max(state.nameChars, Math.min(NAME_MAX, longest));
  state.graphWidth = Math.max(state.graphWidth, cellX(graph.grid.width) + GUTTER);
  const refsWidth = (state.nameChars + COUNTS_CH + WHERE_CH + EXTRA_CH) * CH + LABEL_GAPS;
  const commitAt = new Map(graph.commits.map((commit) => [commit.row, commit]));
  const days = dayStarts(placed, commitAt);
  const dayChars = Math.max(0, ...Array.from(days.values(), (day) => day.length));
  // The day, a space, the time.
  state.axisWidth = Math.max(state.axisWidth, (dayChars + 6) * AXIS_CH + AXIS_EXTRA);
  const map = element("div", "map");
  map.style.setProperty("--axis-width", `${String(Math.round(state.axisWidth))}px`);
  map.style.setProperty("--graph-width", `${String(state.graphWidth)}px`);
  map.style.setProperty("--refs-width", `${String(Math.round(refsWidth))}px`);
  // Narrow panels: the labels keep their name and counts only.
  const narrowWidth = (state.nameChars + COUNTS_CH) * CH + LABEL_GAPS / 3 + 16;
  map.style.setProperty("--refs-width-narrow", `${String(Math.round(narrowWidth))}px`);
  map.style.setProperty("--name-width", `${String(Math.round(state.nameChars * CH))}px`);
  map.style.height = `${String(placed.height)}px`;
  map.append(
    drawGraph(graph, placed, state.graphWidth, thin),
    ...placed.rows.map((row) => tableRow(graph, placed, row, commitAt, days.get(row))),
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
      const { branches, commits, pins, hides } = state.view;
      ask({ show: value, branches, commits, pins, hides });
    } else if (state.view.like !== undefined) {
      ask({ ...state.view, show: "like" });
    }
  });
  label.append(element("span", "control-label", "Show"), select);
  return label;
}

/** The open list (branches, authors or dates), drawn over the page; kept between renders. */
const popover = element("div", "popover");
popover.setAttribute("role", "dialog");
/** Ticks wait here for more before the map is asked for. */
const ticking: { timer: ReturnType<typeof setTimeout> | undefined } = { timer: undefined };

type Popover = "branches" | "authors" | "dates";

/** The id of the toolbar button that opens each list. */
function anchorId(which: Popover): string {
  return `${which}-button`;
}

function anchor(which: Popover): HTMLElement | null {
  return document.querySelector<HTMLElement>(`#${anchorId(which)}`);
}

/** Whether `name`, `rank`th in show order, is ticked in the view. */
function isTicked(view: View, name: string, rank: number): boolean {
  return !view.hides.includes(name) && (rank < view.branches || view.pins.includes(name));
}

/** `view` with `name` ticked on or off: a pin past the first ones, a hide among them. */
function ticked(view: View, name: string, rank: number, isOn: boolean): View {
  const pins = view.pins.filter((pin) => pin !== name);
  const hides = view.hides.filter((hide) => hide !== name);
  if (isOn && rank >= view.branches) {
    pins.push(name);
  }
  if (!isOn && rank < view.branches) {
    hides.push(name);
  }
  return { ...view, pins, hides };
}

/** Ask for `view` once the ticking stops for a moment. */
function askSoon(view: View): void {
  state.view = view;
  clearTimeout(ticking.timer);
  ticking.timer = setTimeout(() => {
    ticking.timer = undefined;
    ask(state.view);
  }, TICK_WAIT_MS);
  render();
}

/** How many of the graph's branches are ticked: drawn as lanes, or as a ref on another's commit. */
function shownCount(graph: Graph): number {
  return graph.branches.filter((branch, rank) => isTicked(state.view, branch.name, rank)).length;
}

/** A row of a list: its tick, a mark, its text; `onTick` gets the new state. */
function listItem(
  key: string,
  isOn: boolean,
  mark: HTMLElement | SVGSVGElement,
  texts: readonly HTMLElement[],
  onTick: (isOn: boolean) => void,
): HTMLLabelElement {
  const item = element("label", "list-item");
  const box = element("input", "");
  box.type = "checkbox";
  box.checked = isOn;
  box.dataset["key"] = key;
  box.addEventListener("change", () => {
    onTick(box.checked);
  });
  item.append(box, mark, ...texts);
  return item;
}

/** A branch's mark: its lane's colour if it is drawn, `ref` if ticked but only a ref. */
function branchMark(graph: Graph, name: string, isOn: boolean): HTMLElement | SVGSVGElement {
  const lanes = graph.lanes.filter((lane) => lane.branch === name);
  const lane = lanes.find((each) => each.kind !== "remote") ?? lanes[0];
  if (lane !== undefined) {
    return swatch(laneClasses(lane), lane.kind === "remote" ? "line" : "ring");
  }
  const mark = element("span", "swatch-blank", isOn ? "ref" : "");
  if (isOn) {
    mark.title = `${name}: no commits of its own here; shown as a ref on another lane.`;
  }
  return mark;
}

/** A button of a list: `text`, explained by `help`; `isPressed` marks a group all ticked. */
function listButton(
  text: string,
  help: string,
  onClick: () => void,
  isPressed?: boolean,
): HTMLButtonElement {
  const button = element("button", "list-button", text);
  button.type = "button";
  button.title = help;
  if (isPressed !== undefined) {
    button.setAttribute("aria-pressed", String(isPressed));
  }
  button.addEventListener("click", onClick);
  return button;
}

/** A search box over a list, its text kept in `state.queries`. */
function searchBox(which: Popover, placeholder: string): HTMLInputElement {
  const search = element("input", "list-search");
  search.type = "search";
  search.placeholder = placeholder;
  search.value = state.queries.get(which) ?? "";
  search.dataset["key"] = "search";
  search.addEventListener("input", () => {
    state.queries.set(which, search.value);
    drawPopover();
  });
  return search;
}

/** The branch list: search, group toggles, then every branch with its tick. */
function branchList(graph: Graph): HTMLElement[] {
  const query = (state.queries.get("branches") ?? "").trim().toLowerCase();
  const matches = graph.branches
    .map((branch, rank) => ({ branch, rank }))
    .filter(({ branch }) => branch.name.toLowerCase().includes(query));
  const first = state.view.branches;
  const buttons = element("div", "list-buttons");
  for (const group of groups(first, state.hiddenAuthors)) {
    const members = matches.filter(({ branch, rank }) => group.has(branch, rank));
    const isAll =
      members.length > 0 &&
      members.every(({ branch, rank }) => isTicked(state.view, branch.name, rank));
    const button = listButton(
      group.label,
      group.help,
      () => {
        let view = state.view;
        for (const { branch, rank } of members) {
          view = ticked(view, branch.name, rank, !isAll);
        }
        askSoon(view);
      },
      isAll,
    );
    button.disabled = members.length === 0;
    buttons.append(button);
  }
  buttons.append(
    listButton(
      "Reset",
      `Back to the first ${String(first)} by Show, nothing ticked by hand.`,
      () => {
        askSoon({ ...state.view, pins: [], hides: [] });
      },
    ),
  );
  const items = element("div", "list-items");
  for (const { branch, rank } of matches) {
    if (rank === first && query === "") {
      items.append(element("div", "list-divider", `past the first ${String(first)} by Show`));
    }
    const isOn = isTicked(state.view, branch.name, rank);
    const item = listItem(
      branch.name,
      isOn,
      branchMark(graph, branch.name, isOn),
      [element("span", "list-name", branch.name)],
      (isTicking) => {
        askSoon(ticked(state.view, branch.name, rank, isTicking));
      },
    );
    item.title = branch.local ? branch.name : `${branch.name}: only on a remote`;
    items.append(item);
  }
  if (matches.length === 0) {
    items.append(element("div", "list-divider", "No branch matches."));
  }
  return [
    searchBox("branches", `Filter ${String(graph.branches.length)} branches`),
    buttons,
    items,
  ];
}

/** The authors list: tick the authors whose commits are shown in full. */
function authorList(graph: Graph): HTMLElement[] {
  const query = (state.queries.get("authors") ?? "").trim().toLowerCase();
  const all = authorsOf(graph);
  const matches = all.filter(
    (author) =>
      author.name.toLowerCase().includes(query) || author.email.toLowerCase().includes(query),
  );
  const setHidden = (emails: Iterable<string>, isHidden: boolean): void => {
    for (const email of emails) {
      if (isHidden) {
        state.hiddenAuthors.add(email);
      } else {
        state.hiddenAuthors.delete(email);
      }
    }
    render();
  };
  const buttons = element("div", "list-buttons");
  buttons.append(
    listButton("All", "Show every author's commits (as filtered).", () => {
      setHidden(
        matches.map((author) => author.email),
        false,
      );
    }),
    listButton("None", "Thin out every author's commits (as filtered).", () => {
      setHidden(
        matches.map((author) => author.email),
        true,
      );
    }),
  );
  const items = element("div", "list-items");
  for (const author of matches) {
    const count = element("span", "list-count", String(author.commits));
    count.title = `${plural(author.commits, "commit")} on the page`;
    const item = listItem(
      author.email,
      !state.hiddenAuthors.has(author.email),
      element("span", "swatch-blank"),
      [
        element("span", "list-name", author.name),
        element("span", "list-email", author.name === author.email ? "" : author.email),
        count,
      ],
      (isOn) => {
        setHidden([author.email], !isOn);
      },
    );
    item.title = `${author.name} <${author.email}>`;
    items.append(item);
  }
  if (matches.length === 0) {
    items.append(element("div", "list-divider", "No author matches."));
  }
  return [searchBox("authors", `Filter ${plural(all.length, "author")}`), buttons, items];
}

/** `view` without its dates. */
function withoutDates(view: View): View {
  const { show, like, branches, commits, pins, hides } = view;
  return { show, ...(like !== undefined && { like }), branches, commits, pins, hides };
}

/** The date filter: presets, and a calendar to pick the first and last day. */
function dateList(graph: Graph): HTMLElement[] {
  const { since, until } = state.view;
  const pick = (bounds: { since: number; until: number } | undefined): void => {
    state.calendar.pending = undefined;
    ask({ ...withoutDates(state.view), ...bounds });
    render();
  };
  const now = new Date();
  const buttons = element("div", "list-buttons");
  buttons.append(
    listButton("7 days", "The last 7 days.", () => {
      pick(lastDays(7, now));
    }),
    listButton("30 days", "The last 30 days.", () => {
      pick(lastDays(30, now));
    }),
    listButton("90 days", "The last 90 days.", () => {
      pick(lastDays(90, now));
    }),
    listButton("Any time", "No dates: the newest commits, and Load older for more.", () => {
      pick(undefined);
    }),
  );
  const pending = state.calendar.pending;
  const hint = element(
    "div",
    "list-divider",
    pending === undefined
      ? "Click the first day, then the last."
      : `From ${formatTime(pending).slice(0, 10)}: now click the last day.`,
  );
  const busy = new Set(graph.commits.map((commit) => dayKey(commit.committed)));
  const month = calendar({
    month: state.calendar.month,
    since,
    until,
    pending,
    busy,
    onMonth: (next) => {
      state.calendar.month = next;
      drawPopover();
    },
    onDay: (day) => {
      if (pending === undefined) {
        state.calendar.pending = day;
        drawPopover();
      } else {
        pick(wholeDays(pending, day));
      }
    },
  });
  return [buttons, month, hint];
}

/** What has the focus in the popover: `search`, a row's tick, or nothing. */
function focusedInPopover(): string | undefined {
  const active = document.activeElement;
  return active instanceof HTMLInputElement && popover.contains(active)
    ? active.dataset["key"]
    : undefined;
}

/** Draw the open list again, keeping its scroll and focus, under its button. */
function drawPopover(): void {
  const graph = state.graph;
  const which = state.popover;
  if (graph === undefined || which === undefined) {
    popover.remove();
    return;
  }
  const scroll = popover.querySelector(".list-items")?.scrollTop ?? 0;
  const focused = focusedInPopover();
  const builders = new Map<Popover, (graph: Graph) => HTMLElement[]>([
    ["branches", branchList],
    ["authors", authorList],
    ["dates", dateList],
  ]);
  popover.replaceChildren(...(builders.get(which)?.(graph) ?? []));
  popover.className = `popover ${which}`;
  popover.setAttribute("aria-label", which);
  if (!popover.isConnected) {
    document.body.append(popover);
  }
  const box = anchor(which)?.getBoundingClientRect();
  if (box !== undefined) {
    popover.style.top = `${String(Math.round(box.bottom + 4))}px`;
    // Inside the window, even under a button near its right edge.
    const left = Math.min(box.left, innerWidth - popover.offsetWidth - 8);
    popover.style.left = `${String(Math.round(Math.max(8, left)))}px`;
  }
  const items = popover.querySelector(".list-items");
  if (items !== null) {
    items.scrollTop = scroll;
  }
  const again = [...popover.querySelectorAll("input")].find(
    (input) => input.dataset["key"] === focused,
  );
  again?.focus({ preventScroll: true });
}

function openPopover(which: Popover | undefined): void {
  state.popover = which;
  if (which === "dates") {
    state.calendar.pending = undefined;
    const graph = state.graph;
    state.calendar.month =
      state.view.until ?? graph?.commits[0]?.committed ?? Math.round(Date.now() / 1000);
  }
  render();
  if (which === undefined) {
    return;
  }
  popover.querySelector<HTMLInputElement>(".list-search")?.focus({ preventScroll: true });
}

popover.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") {
    return;
  }
  const which = state.popover;
  openPopover(undefined);
  if (which !== undefined) {
    anchor(which)?.focus();
  }
});
addEventListener("pointerdown", (event) => {
  const target = event.target;
  const which = state.popover;
  if (
    which !== undefined &&
    target instanceof Node &&
    !popover.contains(target) &&
    anchor(which)?.contains(target) !== true
  ) {
    openPopover(undefined);
  }
});

/** A toolbar button that opens list `which`. */
function popoverButton(
  which: Popover,
  label: string,
  value: string,
  help: string,
): HTMLButtonElement {
  const button = controlButton(label, value, help, () => {
    openPopover(state.popover === which ? undefined : which);
  });
  button.id = anchorId(which);
  button.setAttribute("aria-expanded", String(state.popover === which));
  return button;
}

/** The date filter's value: `any`, or the first and last day. */
function datesValue(): string {
  const { since, until } = state.view;
  if (since === undefined && until === undefined) {
    return "any";
  }
  const day = (seconds: number | undefined): string =>
    seconds === undefined ? "…" : dayLabel(seconds, new Date().getFullYear());
  return `${day(since)} – ${day(until)}`;
}

function filterButtons(graph: Graph | undefined): HTMLButtonElement[] {
  const branches =
    graph === undefined ? "…" : `${String(shownCount(graph))} of ${String(graph.branches.length)}`;
  const authors =
    graph === undefined || state.hiddenAuthors.size === 0
      ? "all"
      : `${String(authorsOf(graph).filter((author) => !state.hiddenAuthors.has(author.email)).length)} of ${String(authorsOf(graph).length)}`;
  return [
    popoverButton(
      "branches",
      "Branches",
      branches,
      "Which branches to show besides the base: tick or untick them.",
    ),
    popoverButton(
      "authors",
      "Authors",
      authors,
      "Whose commits to show in full; the others are thinned out.",
    ),
    popoverButton("dates", "Dates", datesValue(), "Which days of history to show."),
  ];
}

function header(): HTMLElement {
  const graph = state.graph;
  const node = element("header", "header stacked");
  const heading = element("div", "heading");
  heading.append(element("span", "title", `History around ${state.base}`));
  if (graph !== undefined) {
    const shown = `${String(shownCount(graph))} of ${String(graph.branches.length)} branches`;
    const full = graph.commits.filter((commit) => !state.hiddenAuthors.has(commit.email)).length;
    const commits =
      full === graph.commits.length
        ? plural(full, "commit")
        : `${String(full)} of ${plural(graph.commits.length, "commit")}`;
    heading.append(element("span", "count", `${shown} · ${commits}`));
  }
  if (state.isRunning) {
    const progress = state.progress === "" ? "" : `: ${state.progress}`;
    heading.append(element("span", "status", `reading${progress}`));
  }
  const toolbar = element("div", "toolbar");
  toolbar.append(
    ...repositoryControl(state.repository, () => {
      send({ type: "choose", control: "repository" });
    }),
    controlButton(
      "Base",
      state.base,
      "The reference line: every branch is placed against it. Click to pick another.",
      () => {
        send({ type: "choose", control: "base" });
      },
    ),
    showSelect(),
    ...filterButtons(graph),
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
    if (graph.commits.length === 0) {
      parts.push(element("p", "empty", emptyText()));
    }
    const map = drawMap(graph);
    map.classList.toggle("updating", state.isRunning);
    listen(map);
    showSelection(map);
    parts.push(map);
    const below = belowNote(graph);
    if (below !== undefined) {
      parts.push(below);
    }
    // With a first day, the page is the days picked: Dates changes it, not Load older.
    if (graph.page.more && state.view.since === undefined) {
      parts.push(olderButton(graph));
    }
    parts.push(legend(graph));
  }
  parts.push(footer(state.meta));
  root.replaceChildren(...parts);
  scrollTo(0, scroll);
  drawPopover();
}

/** Why the page has no commits. */
function emptyText(): string {
  const { since, until } = state.view;
  if (since === undefined && until === undefined) {
    return "No commits here.";
  }
  const year = new Date().getFullYear();
  const day = (seconds: number | undefined, edge: string): string =>
    seconds === undefined ? edge : dayLabel(seconds, year);
  return `No commits from ${day(since, "the start")} to ${day(until, "now")}: pick other days in Dates.`;
}

/** Which lanes go on below the page, and how far: in place of a count on every label. */
function belowNote(graph: Graph): HTMLElement | undefined {
  const lanes = graph.lanes.filter((lane) => lane.before !== undefined);
  if (lanes.length === 0) {
    return undefined;
  }
  const note = element("div", "below-note");
  note.append(element("span", "below-title", "Goes on below this page:"));
  for (const lane of lanes) {
    const before = lane.before ?? 0;
    const item = element("span", ["below-lane", ...laneClasses(lane)].join(" "));
    item.append(
      element("span", "lane-name", lane.name),
      element("span", "below-count", before === 0 ? " (forks)" : ` ${String(before)}`),
    );
    item.title =
      before === 0
        ? `${lane.name} forks off below this page.`
        : `${plural(before, "commit")} of ${lane.name} are older than this page.`;
    note.append(item);
  }
  return note;
}

function receive(message: ToPage): void {
  switch (message.type) {
    case "setup": {
      state.repository = message.repository;
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
      state.axisWidth = 0;
      state.popover = undefined;
      state.queries = new Map<Popover, string>();
      state.hiddenAuthors = new Set<string>();
      break;
    }
    case "running": {
      state.isRunning = true;
      state.progress = message.progress ?? "";
      // Ticks waiting to be sent are newer than the view running.
      state.view = ticking.timer === undefined ? (message.view ?? state.view) : state.view;
      break;
    }
    case "graph": {
      // Parsed here too: the docs demo posts sundgren's JSON as it is.
      const graph = parseGraph(message.graph);
      state.isRunning = false;
      state.progress = "";
      state.graph = graph;
      state.error = graph === undefined ? "This isn't a sundgren.graph/3 document." : undefined;
      if (graph !== undefined && ticking.timer === undefined) {
        // Forget ticks of branches that are gone from the list.
        const listed = new Set(graph.branches.map((branch) => branch.name));
        state.view = {
          ...state.view,
          pins: state.view.pins.filter((name) => listed.has(name)),
          hides: state.view.hides.filter((name) => listed.has(name)),
        };
      }
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
