// The history panel's page: draws a sundgren.graph/1 document as an SVG graph beside a commit list.
// Runs in the webview (a browser); bundled to dist/webview.js.

import type { Commit, Graph, Ref as Reference } from "../graph.js";
import {
  COLUMN_WIDTH,
  graphWidth,
  linePath,
  NODE_RADIUS,
  ROW_HEIGHT,
  tailPath,
  x,
  y,
} from "./geometry.js";
import { formatTime } from "./format.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

/** Messages from the extension. */
type ToWebview =
  | { readonly type: "graph"; readonly graph: Graph }
  | { readonly type: "error"; readonly message: string };

const PALETTE_SIZE = 8;
const SVG = "http://www.w3.org/2000/svg";

const vscode = acquireVsCodeApi();

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

function colorClass(color: number): string {
  return `c${String(color % PALETTE_SIZE)}`;
}

function drawGraph(graph: Graph): SVGSVGElement {
  const rows = graph.commits.length;
  const height = rows * ROW_HEIGHT + (graph.tail.length > 0 ? ROW_HEIGHT / 2 : 0);
  const svg = svgElement("svg", {
    class: "graph",
    width: graphWidth(graph.columns),
    height,
    "aria-hidden": "true",
  });
  const lines = svgElement("g", { class: "lines" });
  const nodes = svgElement("g", { class: "nodes" });
  for (const [row, commit] of graph.commits.entries()) {
    for (const line of commit.lines) {
      lines.append(svgElement("path", { d: linePath(line, row), class: colorClass(line.color) }));
    }
    const isMerge = commit.parents.length > 1;
    nodes.append(
      svgElement("circle", {
        cx: x(commit.column),
        cy: y(row),
        r: NODE_RADIUS,
        class: `${colorClass(commit.color)}${isMerge ? " merge" : ""}`,
      }),
    );
  }
  for (const line of graph.tail) {
    lines.append(
      svgElement("path", { d: tailPath(line, rows - 1), class: colorClass(line.color) }),
    );
  }
  svg.append(lines, nodes);
  return svg;
}

function referenceBadge(reference: Reference): HTMLSpanElement {
  return element("span", `ref ${reference.kind}`, reference.name);
}

function commitRow(commit: Commit, isTip: boolean): HTMLDivElement {
  const row = element("div", isTip ? "row tip" : "row");
  row.dataset["oid"] = commit.oid;
  row.title = [
    commit.oid,
    `${commit.author.name} <${commit.author.email}>`,
    `authored ${formatTime(commit.authored)}, committed ${formatTime(commit.committed)}`,
    commit.parents.length > 1 ? `merge of ${String(commit.parents.length)} parents` : "",
  ]
    .filter((part) => part !== "")
    .join("\n");
  const message = element("span", "message");
  // HEAD is shown on its branch's badge instead of on its own.
  const hasHeadBranch = commit.refs.some((reference) => reference.kind === "head");
  for (const [index, reference] of commit.refs.entries()) {
    if (reference.kind === "head") {
      continue;
    }
    const badge = referenceBadge(reference);
    const previous = commit.refs[index - 1];
    if (hasHeadBranch && previous?.kind === "head" && reference.kind === "branch") {
      badge.classList.add("current");
    }
    message.append(badge);
  }
  if (hasHeadBranch && commit.refs.every((reference) => reference.kind !== "branch")) {
    message.append(element("span", "ref head", "HEAD"));
  }
  message.append(element("span", "subject", commit.subject));
  row.append(
    message,
    element("span", "author", commit.author.name),
    element("span", "date", formatTime(commit.authored)),
    element("span", "sha", commit.oid.slice(0, 8)),
  );
  return row;
}

function render(graph: Graph): void {
  const root = document.querySelector("#root");
  if (root === null) {
    return;
  }
  const header = element("header", "header");
  header.append(
    element("span", "rev", graph.rev),
    element("span", "sha", graph.oid.slice(0, 8)),
    element(
      "span",
      "count",
      `${String(graph.commits.length)} commits${graph.truncated ? " (more not shown)" : ""}`,
    ),
  );
  const history = element("div", "history");
  const width = graphWidth(graph.columns) + COLUMN_WIDTH / 2;
  history.style.setProperty("--graph-width", `${String(width)}px`);
  history.style.setProperty("--row-height", `${String(ROW_HEIGHT)}px`);
  const list = element("div", "rows");
  list.append(...graph.commits.map((commit, row) => commitRow(commit, row === 0)));
  list.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target.closest(".row") : undefined;
    for (const selected of list.querySelectorAll(".row.selected")) {
      selected.classList.remove("selected");
    }
    target?.classList.add("selected");
  });
  history.append(drawGraph(graph), list);
  root.replaceChildren(header, history);
}

function showError(message: string): void {
  document.querySelector("#root")?.replaceChildren(element("p", "error", message));
}

// Only VS Code can post into a webview's frame: these are the extension's messages.
// eslint-disable-next-line sonarjs/post-message -- see above
window.addEventListener("message", (event: MessageEvent<ToWebview>) => {
  const message = event.data;
  if (message.type === "graph") {
    render(message.graph);
  } else {
    showError(message.message);
  }
});

vscode.postMessage({ type: "ready" });
