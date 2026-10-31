"use strict";
(() => {
  // src/webview/geometry.ts
  var ROW_HEIGHT = 24;
  var COLUMN_WIDTH = 16;
  var NODE_RADIUS = 4;
  function x(column) {
    return column * COLUMN_WIDTH + COLUMN_WIDTH / 2;
  }
  function y(row) {
    return row * ROW_HEIGHT + ROW_HEIGHT / 2;
  }
  function graphWidth(columns) {
    return Math.max(columns, 1) * COLUMN_WIDTH;
  }
  function path(from, to, y1, y2) {
    const x1 = x(from);
    const x2 = x(to);
    if (from === to) {
      return `M${String(x1)} ${String(y1)}V${String(y2)}`;
    }
    const middle = (y1 + y2) / 2;
    return `M${String(x1)} ${String(y1)}C${String(x1)} ${String(middle)} ${String(x2)} ${String(middle)} ${String(x2)} ${String(y2)}`;
  }
  function linePath(line, row) {
    return path(line.from, line.to, y(row - 1), y(row));
  }
  function tailPath(line, lastRow) {
    return path(line.from, line.to, y(lastRow), y(lastRow) + ROW_HEIGHT);
  }

  // src/webview/format.ts
  function pad(n) {
    return String(n).padStart(2, "0");
  }
  function formatTime(seconds) {
    const d = new Date(seconds * 1e3);
    const date = `${String(d.getFullYear())}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    return `${date} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  // src/webview/main.ts
  var PALETTE_SIZE = 8;
  var SVG = "http://www.w3.org/2000/svg";
  var vscode = acquireVsCodeApi();
  function send(message) {
    vscode.postMessage(message);
  }
  function element(tag, className, text) {
    const node = document.createElement(tag);
    node.className = className;
    if (text !== void 0) {
      node.textContent = text;
    }
    return node;
  }
  function svgElement(tag, attributes) {
    const node = document.createElementNS(SVG, tag);
    for (const [name, value] of Object.entries(attributes)) {
      node.setAttribute(name, String(value));
    }
    return node;
  }
  function colorClass(color) {
    return `c${String(color % PALETTE_SIZE)}`;
  }
  function drawGraph(graph) {
    const rows = graph.commits.length;
    const height = rows * ROW_HEIGHT + (graph.tail.length > 0 ? ROW_HEIGHT / 2 : 0);
    const svg = svgElement("svg", {
      class: "graph",
      width: graphWidth(graph.columns),
      height,
      "aria-hidden": "true"
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
          class: `${colorClass(commit.color)}${isMerge ? " merge" : ""}`
        })
      );
    }
    for (const line of graph.tail) {
      lines.append(
        svgElement("path", { d: tailPath(line, rows - 1), class: colorClass(line.color) })
      );
    }
    svg.append(lines, nodes);
    return svg;
  }
  function refBadge(ref) {
    return element("span", `ref ${ref.kind}`, ref.name);
  }
  function commitRow(commit, isTip) {
    const row = element("div", isTip ? "row tip" : "row");
    row.dataset["oid"] = commit.oid;
    row.title = [
      commit.oid,
      `${commit.author.name} <${commit.author.email}>`,
      `authored ${formatTime(commit.authored)}, committed ${formatTime(commit.committed)}`,
      commit.parents.length > 1 ? `merge of ${String(commit.parents.length)} parents` : ""
    ].filter((part) => part !== "").join("\n");
    const message = element("span", "message");
    const hasHeadBranch = commit.refs.some((ref) => ref.kind === "head");
    for (const [index, ref] of commit.refs.entries()) {
      if (ref.kind === "head") {
        continue;
      }
      const badge = refBadge(ref);
      const previous = commit.refs[index - 1];
      if (hasHeadBranch && previous?.kind === "head" && ref.kind === "branch") {
        badge.classList.add("current");
      }
      message.append(badge);
    }
    if (hasHeadBranch && commit.refs.every((ref) => ref.kind !== "branch")) {
      message.append(element("span", "ref head", "HEAD"));
    }
    message.append(element("span", "subject", commit.subject));
    row.append(
      message,
      element("span", "author", commit.author.name),
      element("span", "date", formatTime(commit.authored)),
      element("span", "sha", commit.oid.slice(0, 8))
    );
    return row;
  }
  function render(graph) {
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
        `${String(graph.commits.length)} commits${graph.truncated ? " (more not shown)" : ""}`
      )
    );
    const history = element("div", "history");
    const width = graphWidth(graph.columns) + COLUMN_WIDTH / 2;
    history.style.setProperty("--graph-width", `${String(width)}px`);
    history.style.setProperty("--row-height", `${String(ROW_HEIGHT)}px`);
    const list = element("div", "rows");
    list.append(...graph.commits.map((commit, row) => commitRow(commit, row === 0)));
    list.addEventListener("click", (event) => {
      const target = event.target instanceof Element ? event.target.closest(".row") : void 0;
      for (const selected of list.querySelectorAll(".row.selected")) {
        selected.classList.remove("selected");
      }
      target?.classList.add("selected");
    });
    history.append(drawGraph(graph), list);
    root.replaceChildren(header, history);
    send({ type: "rendered", oid: graph.oid, rows: list.childElementCount });
  }
  function showError(message) {
    document.querySelector("#root")?.replaceChildren(element("p", "error", message));
  }
  window.addEventListener("message", (event) => {
    if (event.origin !== origin) {
      return;
    }
    const message = event.data;
    if (message.type === "graph") {
      render(message.graph);
    } else {
      showError(message.message);
    }
  });
  send({ type: "ready" });
})();
//# sourceMappingURL=webview.js.map
