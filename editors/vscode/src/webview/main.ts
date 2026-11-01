// The history page: a map of branches around the tip of a base branch, drawn from the grid of a
// sundgren.graph/2 document as SVG, with a label per lane. Runs in the webview (a browser);
// bundled to dist/webview.js.

import type { Meta } from "../containment.js";
import type { BranchState } from "../git.js";
import {
  DOWN,
  LEFT,
  parseGraph,
  RIGHT,
  UP,
  type Cell,
  type Commit,
  type Graph,
  type Lane,
} from "../graph.js";
import type { FromPage, Selection, ToPage, View } from "../messages.js";
import {
  controlButton,
  element,
  footer,
  info,
  percent,
  short,
  syncBadge,
  worktreeBadges,
} from "./common.js";
import { formatTime } from "./format.js";
import {
  CELL_HEIGHT,
  CELL_WIDTH,
  cellX,
  columnsFor,
  GUTTER,
  LABEL_CHARACTER_WIDTH,
  lanePaths,
  markers,
  NODE_RADIUS,
  rows,
  type Rows,
} from "./geometry.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const SVG = "http://www.w3.org/2000/svg";
/** Lane colour families in brand.css, in the order `family` counts them. */
const FAMILIES = ["orange", "forest", "wine", "sage", "rose", "brown", "pink"] as const;
const MAX_BRANCHES = 7;
const RESIZE_DELAY_MS = 250;
/** Page padding (both sides), the space between grid and labels, and the labels' width. */
const PAGE_PADDING = 24;
const LABEL_GAP = 12;
const LABEL_SHARE = 0.38;
const LABEL_MIN = 200;
const LABEL_MAX = 360;

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

const state: {
  base: string;
  target: string | undefined;
  branches: number;
  meta: Meta | undefined;
  states: Map<string, BranchState>;
  graph: Graph | undefined;
  /** The view last asked for. */
  view: View | undefined;
  isRunning: boolean;
  progress: string;
  error: string | undefined;
  /** What was clicked last: `commit:<oid>` or `lane:<name>`. */
  selected: string | undefined;
} = {
  base: "",
  target: undefined,
  branches: 3,
  meta: undefined,
  states: new Map<string, BranchState>(),
  graph: undefined,
  view: undefined,
  isRunning: false,
  progress: "",
  error: undefined,
  selected: undefined,
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

/** Width of the lane labels for a page `width` wide. */
function labelWidth(width: number): number {
  return Math.round(Math.min(LABEL_MAX, Math.max(LABEL_MIN, width * LABEL_SHARE)));
}

/** The commit columns that fit the page as it is now. */
function columnsNow(): number {
  const width = document.documentElement.clientWidth;
  return columnsFor(width - PAGE_PADDING - GUTTER - LABEL_GAP - labelWidth(width));
}

/** Ask the extension for `view`, unless that is what was asked for last. */
function ask(view: View): void {
  const last = state.view;
  if (
    last?.offset === view.offset &&
    last.columns === view.columns &&
    last.branches === view.branches
  ) {
    return;
  }
  state.view = view;
  send({ type: "view", ...view });
}

/** Ask again with as many columns as fit now, at the same frame. */
function refit(): void {
  ask({
    offset: state.view?.offset ?? 0,
    columns: columnsNow(),
    branches: state.branches,
  });
}

/** CSS classes for a lane's colour and line style. */
function laneClasses(lane: Lane): string[] {
  const classes = [lane.kind === "remote" ? "remote" : "local"];
  if (lane.protected || lane.family === undefined) {
    classes.push("protected");
  } else {
    classes.push(`family-${String(lane.family % FAMILIES.length)}`);
    if (lane.family >= FAMILIES.length) {
      classes.push("dotted");
    }
  }
  if (lane.merged) {
    classes.push("merged");
  }
  if (lane.only === "remote") {
    classes.push("dashed");
  }
  return classes;
}

/** A count in little space: 140, 1.2k. */
function compact(count: number): string {
  return count < 1000 ? String(count) : `${String(Math.round(count / 100) / 10)}k`;
}

function selectionKey(selection: Selection): string {
  return selection.kind === "commit" ? `commit:${selection.oid}` : `lane:${selection.name}`;
}

/** One line of what a lane is, for hover text. */
function laneSummary(lane: Lane, graph: Graph): string {
  if (lane.kind === "reference") {
    return `${lane.name}: the base, the line every other lane is placed against.`;
  }
  const parts = [
    `${lane.name}: ${String(lane.ahead)} commits not in ${graph.base.name}, ${String(lane.behind)} of ${graph.base.name} not in it.`,
  ];
  if (lane.merged) {
    parts.push("Merged: nothing ahead of the base.");
  }
  if (lane.only === "remote") {
    parts.push("Only on the remote.");
  }
  if (lane.score !== undefined && graph.target !== undefined) {
    parts.push(`${percent(lane.score)} of its work is in ${graph.target.name}.`);
  }
  if (lane.before !== undefined) {
    parts.push(`It forked before the frame: ${String(lane.before)} of its commits are earlier.`);
  }
  if (lane.after) {
    parts.push("It goes on after the frame.");
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

function centre(cell: Cell, placement: Rows): { x: number; y: number } {
  return {
    x: cellX(cell.col) + CELL_WIDTH / 2,
    y: placement.y(cell.row) + CELL_HEIGHT / 2,
  };
}

/** A commit: a dot on the reference lane, a ring elsewhere; focusable and clickable. */
function commitNode(graph: Graph, cell: Cell, placement: Rows): SVGGElement | undefined {
  const commit = cell.commit === undefined ? undefined : graph.commits[cell.commit];
  const lane = graph.lanes[cell.lane];
  if (commit === undefined || lane === undefined) {
    return undefined;
  }
  const { x, y } = centre(cell, placement);
  const node = svgElement("g", {
    class: ["commit", ...laneClasses(lane)].join(" "),
    tabindex: 0,
    role: "button",
    "aria-label": `Commit ${short(commit.oid)} on ${lane.name}: ${commit.subject}`,
  });
  node.dataset["oid"] = commit.oid;
  node.dataset["lane"] = String(cell.lane);
  node.append(
    svgElement("circle", { class: "focus", cx: x, cy: y, r: NODE_RADIUS + 3 }),
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

/** An off-board connector: the lane goes on outside the frame. */
function connectorNode(graph: Graph, cell: Cell, placement: Rows): SVGGElement | undefined {
  const lane = graph.lanes[cell.lane];
  if (lane === undefined) {
    return undefined;
  }
  const { x, y } = centre(cell, placement);
  const isLeft = cell.col === 0;
  const node = svgElement("g", { class: ["connector", ...laneClasses(lane)].join(" ") });
  node.dataset["lane"] = String(cell.lane);
  node.append(
    svgElement("circle", { class: "pad", cx: x, cy: y, r: NODE_RADIUS }),
    svgElement("circle", { class: "pad-hole", cx: x, cy: y, r: 1.5 }),
  );
  let help = `${lane.name} goes on after the frame.`;
  if (isLeft) {
    const before = lane.before ?? 0;
    help =
      before > 0
        ? `${lane.name} goes on before the frame: ${String(before)} earlier commits of its own.`
        : `${lane.name} goes on before the frame.`;
    if (before > 0) {
      const count = svgElement("text", {
        class: "count",
        x: x - NODE_RADIUS - 3,
        y: y + 4,
        "text-anchor": "end",
      });
      count.textContent = compact(before);
      node.append(count);
    }
  }
  node.append(svgTitle(help));
  return node;
}

/** The `▼` markers above the grid, with their labels. */
function markerNodes(graph: Graph, placement: Rows): SVGGElement {
  const group = svgElement("g", { class: "markers" });
  for (const marker of markers(graph)) {
    const x = cellX(marker.col) + CELL_WIDTH / 2;
    const y = placement.labelY(marker.labelRow) + CELL_HEIGHT * 0.7;
    const arrow = svgElement("text", { class: "arrow", x, y, "text-anchor": "middle" });
    arrow.textContent = "▼";
    const label = svgElement("text", { class: "marker-label", x: x + LABEL_CHARACTER_WIDTH, y });
    label.textContent = marker.label;
    const item = svgElement("g", { class: "marker" });
    item.append(arrow, label, svgTitle(marker.names.join("\n")));
    group.append(item);
  }
  return group;
}

/** The lane's label: name, ahead and behind, what else is true of it, and its badges. */
function laneLabel(graph: Graph, lane: Lane, index: number, top: number): HTMLDivElement {
  const label = element("div", ["lane-label", ...laneClasses(lane)].join(" "));
  label.dataset["lane"] = String(index);
  label.tabIndex = 0;
  label.setAttribute("role", "button");
  label.style.top = `${String(top)}px`;
  label.title = laneSummary(lane, graph);
  label.append(element("span", "lane-name", lane.name));
  const facts = [
    lane.kind === "reference" ? "base" : `↑${String(lane.ahead)} ↓${String(lane.behind)}`,
  ];
  if (lane.merged) {
    facts.push("merged");
  }
  if (lane.name === graph.target?.name) {
    facts.push("target");
  } else if (lane.pinned && lane.kind !== "reference") {
    facts.push("pinned");
  }
  if (lane.score !== undefined && graph.target !== undefined) {
    facts.push(`${percent(lane.score)} in ${graph.target.name}`);
  }
  label.append(element("span", "lane-facts", facts.join(" · ")));
  const where = state.states.get(lane.name);
  if (where !== undefined) {
    const badges = element("span", "badges");
    badges.append(syncBadge(lane.name, where), ...worktreeBadges(where));
    label.append(badges);
  }
  return label;
}

function drawMap(graph: Graph): HTMLElement {
  const placement = rows(graph, markers(graph));
  const width = cellX(graph.grid.width);
  const map = element("div", "map");
  map.style.setProperty("--grid-width", `${String(width + LABEL_GAP)}px`);
  map.style.setProperty(
    "--label-width",
    `${String(labelWidth(document.documentElement.clientWidth))}px`,
  );
  map.style.setProperty("--cell-height", `${String(CELL_HEIGHT)}px`);
  map.style.height = `${String(placement.height)}px`;
  const svg = svgElement("svg", {
    class: "map-svg",
    width,
    height: placement.height,
    role: "group",
    "aria-label": `Branches around ${graph.base.name}`,
  });
  const traces = lanePaths(graph.grid.cells, placement);
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
  const nodes = svgElement("g", { class: "nodes" });
  for (const cell of graph.grid.cells) {
    let node: SVGGElement | undefined;
    if (cell.node === "commit") {
      node = commitNode(graph, cell, placement);
    } else if (cell.node === "connector") {
      node = connectorNode(graph, cell, placement);
    }
    if (node !== undefined) {
      nodes.append(node);
    }
  }
  svg.append(nodes, markerNodes(graph, placement));
  const labels = element("div", "lane-labels");
  labels.append(
    ...graph.lanes.map((lane, index) => laneLabel(graph, lane, index, placement.y(lane.row))),
  );
  map.append(svg, labels);
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
    for (const node of root.querySelectorAll(".commit")) {
      if (node instanceof SVGElement && node.dataset["oid"] === oid) {
        node.classList.add("selected");
      }
    }
    return;
  }
  const index = graph.lanes.findIndex((lane) => `lane:${lane.name}` === selected);
  for (const node of root.querySelectorAll(`[data-lane="${CSS.escape(String(index))}"]`)) {
    node.classList.add("selected");
  }
}

/** What a click or key on `target` selects, if anything. */
function selectionAt(target: EventTarget | null | undefined): Selection | undefined {
  if (!(target instanceof Element) || state.graph === undefined) {
    return undefined;
  }
  const commit = target.closest(".commit");
  const oid = commit instanceof SVGElement ? commit.dataset["oid"] : undefined;
  if (oid !== undefined) {
    return { kind: "commit", oid };
  }
  const owner = target.closest("[data-lane]");
  const index =
    owner instanceof HTMLElement || owner instanceof SVGElement ? owner.dataset["lane"] : undefined;
  const lane = index === undefined ? undefined : state.graph.lanes[Number(index)];
  return lane === undefined ? undefined : { kind: "lane", name: lane.name };
}

function select(selection: Selection, root: Element): void {
  state.selected = selectionKey(selection);
  showSelection(root);
  send({ type: "select", selection });
}

/** Light up every element of the lane under `target`, or none. */
function highlight(root: Element, target: EventTarget | null | undefined): void {
  const owner = target instanceof Element ? target.closest("[data-lane]") : undefined;
  const index =
    owner instanceof HTMLElement || owner instanceof SVGElement ? owner.dataset["lane"] : undefined;
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
    const selection = selectionAt(event.target);
    if (selection !== undefined) {
      select(selection, map);
    }
  });
  map.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") {
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
function swatch(classes: readonly string[], draw: "line" | "dot" | "ring"): SVGSVGElement {
  const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
  const group = svgElement("g", { class: ["lane", ...classes].join(" ") });
  if (draw === "line") {
    group.append(svgElement("path", { class: "trace", d: "M3 7H25" }));
  } else {
    group.append(svgElement("path", { class: "trace", d: "M3 7H25" }));
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

/** The via symbol: a lane broken under a vertical one. */
function viaSwatch(): SVGSVGElement {
  const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
  const under = svgElement("g", { class: "lane protected" });
  under.append(svgElement("path", { class: "trace", d: "M2 7H9.5M18.5 7H26" }));
  const over = svgElement("g", { class: "lane family-0" });
  over.append(svgElement("path", { class: "trace", d: "M14 0V14" }));
  svg.append(under, over);
  return svg;
}

function connectorSwatch(): SVGSVGElement {
  const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
  const group = svgElement("g", { class: "connector protected" });
  group.append(
    svgElement("path", { class: "trace", d: "M14 7H27" }),
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

/** Whether any lane is broken under another's vertical. */
function hasVia(graph: Graph): boolean {
  const at = new Map(
    graph.grid.cells.map((cell) => [`${String(cell.row)},${String(cell.col)}`, cell]),
  );
  return graph.grid.cells.some((cell) => {
    if (cell.sides !== (UP | DOWN)) {
      return false;
    }
    const left = at.get(`${String(cell.row)},${String(cell.col - 1)}`);
    const right = at.get(`${String(cell.row)},${String(cell.col + 1)}`);
    return left?.sides === LEFT && right?.sides === RIGHT && left.lane !== cell.lane;
  });
}

/** What every colour and marking on the map means: only those it uses. */
function legend(graph: Graph): HTMLElement {
  const node = element("div", "map-legend");
  const branches = new Map<string, Lane[]>();
  for (const lane of graph.lanes) {
    if (!lane.protected && lane.family !== undefined) {
      branches.set(lane.branch, [...(branches.get(lane.branch) ?? []), lane]);
    }
  }
  for (const [branch, lanes] of branches) {
    const item = element("span", "legend-item");
    for (const lane of lanes) {
      const classes = laneClasses(lane).filter((name) => name !== "merged" && name !== "dashed");
      item.append(swatch(classes, "line"));
    }
    const kinds = lanes.map((lane) => (lane.kind === "remote" ? "remote" : "local")).join(", ");
    item.append(element("span", "", `${branch} (${kinds})`));
    node.append(item);
  }
  const isUsed = (hasIt: (lane: Lane) => boolean): boolean =>
    graph.lanes.some((lane) => hasIt(lane));
  if (isUsed((lane) => lane.twin !== undefined && lane.family !== undefined)) {
    node.append(element("span", "legend-note", "A remote twin: the same colour, turned in hue."));
  }
  const protectedNames = graph.lanes.filter((lane) => lane.protected || lane.family === undefined);
  if (protectedNames.length > 0) {
    node.append(
      legendItem(
        swatch(["protected"], "line"),
        `protected: ${protectedNames.map((lane) => lane.name).join(", ")}`,
      ),
    );
  }
  if (isUsed((lane) => lane.merged)) {
    node.append(
      legendItem(swatch(["protected", "merged"], "line"), "faded: merged, nothing ahead"),
    );
  }
  if (isUsed((lane) => lane.only === "remote")) {
    node.append(legendItem(swatch(["protected", "dashed"], "line"), "dashed: only on the remote"));
  }
  if (isUsed((lane) => (lane.family ?? 0) >= FAMILIES.length)) {
    node.append(legendItem(swatch(["protected", "dotted"], "line"), "dotted: colours used again"));
  }
  node.append(
    legendItem(swatch(["protected"], "dot"), `commit of ${graph.base.name}`),
    legendItem(swatch(["family-0"], "ring"), "commit of another lane"),
  );
  if (hasVia(graph)) {
    node.append(legendItem(viaSwatch(), "via: a lane passing under another, not joined"));
  }
  if (graph.grid.cells.some((cell) => cell.node === "connector")) {
    node.append(
      legendItem(connectorSwatch(), "goes on outside the frame; the number: earlier commits"),
    );
  }
  if (markers(graph).length > 0) {
    node.append(
      legendItem(element("span", "legend-arrow", "▼"), `branches and tags on ${graph.base.name}`),
    );
  }
  return node;
}

function branchesSelect(): HTMLLabelElement {
  const label = element("label", "control");
  label.title = "How many branches to show besides the base and the target.";
  const select = element("select", "control-select");
  for (let count = 1; count <= MAX_BRANCHES; count++) {
    const option = element("option", "", String(count));
    option.value = String(count);
    option.selected = count === state.branches;
    select.append(option);
  }
  select.addEventListener("change", () => {
    state.branches = Number(select.value);
    refit();
  });
  label.append(element("span", "control-label", "Branches"), select);
  return label;
}

function stepButton(text: string, help: string, offset: number | undefined): HTMLButtonElement {
  const button = element("button", "control step", text);
  button.type = "button";
  button.title = help;
  button.disabled = offset === undefined;
  button.addEventListener("click", () => {
    if (offset !== undefined) {
      ask({ offset, columns: columnsNow(), branches: state.branches });
    }
  });
  return button;
}

function header(): HTMLElement {
  const graph = state.graph;
  const node = element("header", "header stacked");
  const heading = element("div", "heading");
  heading.append(element("span", "title", `History around ${state.base}`));
  if (graph !== undefined) {
    const others = graph.lanes.filter((lane) => lane.kind !== "reference").length;
    heading.append(
      element(
        "span",
        "count",
        `${String(others)} lanes · ${String(graph.candidates)} branches in the frame`,
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
    controlButton(
      "Target",
      state.target ?? "none",
      "The branch the others are ranked against: the branches whose work is most in it are shown. Click to pick another.",
      () => {
        send({ type: "choose", control: "target" });
      },
    ),
    branchesSelect(),
    stepButton("◀", "Older: one step back in time", graph?.frame.older),
    stepButton("▶", "Newer: one step forward in time", graph?.frame.newer),
    info(
      state.target === undefined ? "newest branches" : `most in ${state.target}`,
      state.target === undefined
        ? "Without a target, the branches with the newest tips in the frame are shown."
        : `Of the branches visible in the frame, those whose work is most found in ${state.target} are shown (containment, as in the Containment panel).`,
    ),
  );
  node.append(heading, toolbar);
  return node;
}

function render(): void {
  const root = document.querySelector("#root");
  if (root === null) {
    return;
  }
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
    parts.push(map, legend(graph));
  }
  parts.push(footer(state.meta));
  root.replaceChildren(...parts);
}

function receive(message: ToPage): void {
  switch (message.type) {
    case "setup": {
      state.base = message.base;
      state.target = message.target;
      state.branches = message.branches;
      state.meta = message.meta;
      state.states = new Map(message.states.map((branch) => [branch.name, branch]));
      state.graph = undefined;
      state.view = message.view;
      state.isRunning = false;
      state.error = undefined;
      state.selected = undefined;
      render();
      if (message.view === undefined) {
        refit();
      }
      return;
    }
    case "running": {
      state.isRunning = true;
      state.progress = message.progress ?? "";
      break;
    }
    case "graph": {
      // Parsed here too: the docs demo posts sundgren's JSON as it is.
      const graph = parseGraph(message.graph);
      state.isRunning = false;
      state.progress = "";
      state.graph = graph;
      state.error = graph === undefined ? "This isn't a sundgren.graph/2 document." : undefined;
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
  if (message.type !== "graph" || state.graph === undefined) {
    return;
  }

  send({ type: "rendered", oid: state.graph.base.oid, rows: state.graph.lanes.length });
  // The panel may have changed size while sundgren ran.
  refit();
}

// Fit the frame to the panel again once it has stopped changing size.
const resizing: { timer?: ReturnType<typeof setTimeout> } = {};
new ResizeObserver(() => {
  clearTimeout(resizing.timer);
  resizing.timer = setTimeout(() => {
    if (state.view !== undefined) {
      refit();
    }
  }, RESIZE_DELAY_MS);
}).observe(document.documentElement);

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToPage>) => {
  if (event.origin !== origin) {
    return;
  }
  receive(event.data);
});

send({ type: "ready" });
