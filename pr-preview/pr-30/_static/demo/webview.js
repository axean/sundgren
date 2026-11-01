"use strict";
(() => {
  // src/graph.ts
  var UP = 1;
  var RIGHT = 2;
  var DOWN = 4;
  var LEFT = 8;
  function isObject(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function isCount(value) {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
  }
  function isNumber(value) {
    return typeof value === "number";
  }
  function isString(value) {
    return typeof value === "string";
  }
  function isOid(value) {
    return typeof value === "string" && (value.length === 40 || value.length === 64) && /^[\da-f]+$/u.test(value);
  }
  function isArrayOf(value, isItem) {
    return Array.isArray(value) && value.every((item) => isItem(item));
  }
  var Optional = class {
    isValid;
    value;
    constructor(value, isItem) {
      const isMissing = value === null || value === void 0;
      this.isValid = isMissing || isItem(value);
      this.value = !isMissing && isItem(value) ? value : void 0;
    }
  };
  function field(key, value) {
    return value === void 0 ? {} : { [key]: value };
  }
  function parseNamed(value) {
    return isObject(value) && isString(value["name"]) && isOid(value["oid"]) ? { name: value["name"], oid: value["oid"] } : void 0;
  }
  function parseFrame(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { offset, columns } = value;
    const older = new Optional(value["older"], isCount);
    const newer = new Optional(value["newer"], isCount);
    return !isCount(offset) || !isCount(columns) || columns < 1 || !older.isValid || !newer.isValid ? void 0 : { offset, columns, ...field("older", older.value), ...field("newer", newer.value) };
  }
  var LANE_KINDS = /* @__PURE__ */ new Set(["reference", "local", "remote"]);
  var SIDES_ONLY = /* @__PURE__ */ new Set(["local", "remote"]);
  var NODES = /* @__PURE__ */ new Set(["commit", "connector"]);
  function isLaneKind(value) {
    return LANE_KINDS.has(value);
  }
  function isOnly(value) {
    return SIDES_ONLY.has(value);
  }
  function isNode(value) {
    return NODES.has(value);
  }
  function laneBasics(value) {
    const { name, branch, kind, row, ahead, behind, tip, after } = value;
    const flags = [value["protected"], value["pinned"], value["merged"]];
    if (typeof after !== "boolean" || !isString(name) || !isString(branch) || !isLaneKind(kind) || !isCount(row) || !isCount(ahead) || !isCount(behind) || !isOid(tip) || flags.some((flag) => typeof flag !== "boolean")) {
      return void 0;
    }
    return {
      name,
      branch,
      kind,
      row,
      protected: value["protected"] === true,
      pinned: value["pinned"] === true,
      merged: value["merged"] === true,
      ahead,
      behind,
      tip,
      after
    };
  }
  function parseLane(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const basics = laneBasics(value);
    const family = new Optional(value["family"], isCount);
    const twin = new Optional(value["twin"], isCount);
    const only = new Optional(value["only"], isOnly);
    const score = new Optional(value["score"], isNumber);
    const fork = new Optional(value["fork"], isOid);
    const before = new Optional(value["before"], isCount);
    const options = [family, twin, only, score, fork, before];
    if (basics === void 0 || options.some((option) => !option.isValid)) {
      return void 0;
    }
    return {
      ...basics,
      ...field("family", family.value),
      ...field("twin", twin.value),
      ...field("only", only.value),
      ...field("score", score.value),
      ...field("fork", fork.value),
      ...field("before", before.value)
    };
  }
  function parseCommit(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { oid, parents, author, committed, subject, refs, column, lanes } = value;
    return isOid(oid) && isArrayOf(parents, isOid) && isString(author) && isNumber(committed) && Number.isSafeInteger(committed) && isString(subject) && isArrayOf(refs, isString) && isCount(column) && isArrayOf(lanes, isCount) ? { oid, parents, author, committed, subject, refs, column, lanes } : void 0;
  }
  function parseCell(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { row, col, sides, lane } = value;
    const node = new Optional(value["node"], isNode);
    const commit = new Optional(value["commit"], isCount);
    return isCount(row) && isCount(col) && isCount(sides) && sides <= 15 && isCount(lane) && node.isValid && commit.isValid ? { row, col, sides, lane, ...field("node", node.value), ...field("commit", commit.value) } : void 0;
  }
  function parseAll(value, parse) {
    if (!Array.isArray(value)) {
      return void 0;
    }
    const items = [];
    for (const item of value) {
      const parsed = parse(item);
      if (parsed === void 0) {
        return void 0;
      }
      items.push(parsed);
    }
    return items;
  }
  function parseGrid(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { width, height } = value;
    const cells = parseAll(value["cells"], parseCell);
    return cells !== void 0 && isCount(width) && isCount(height) ? { width, height, cells } : void 0;
  }
  function isConsistent(graph) {
    const { lanes, commits, grid } = graph;
    const isLaneIndex = (index) => index < lanes.length;
    return lanes.every(
      (lane) => lane.row < grid.height && (lane.twin === void 0 || isLaneIndex(lane.twin))
    ) && commits.every((commit) => commit.lanes.every((lane) => isLaneIndex(lane))) && grid.cells.every(
      (cell) => cell.row < grid.height && cell.col < grid.width && isLaneIndex(cell.lane) && (cell.commit === void 0 || cell.commit < commits.length)
    );
  }
  function parseGraph(value) {
    if (!isObject(value) || value["schema"] !== "sundgren.graph/2") {
      return void 0;
    }
    const { repository, threshold, candidates } = value;
    const base = parseNamed(value["base"]);
    const target = new Optional(
      value["target"],
      (item) => parseNamed(item) !== void 0
    );
    const frame = parseFrame(value["frame"]);
    const lanes = parseAll(value["lanes"], parseLane);
    const commits = parseAll(value["commits"], parseCommit);
    const grid = parseGrid(value["grid"]);
    if (base === void 0 || frame === void 0 || lanes === void 0 || commits === void 0 || grid === void 0 || !target.isValid || !isString(repository) || !isNumber(threshold) || !isCount(candidates)) {
      return void 0;
    }
    const graph = {
      schema: "sundgren.graph/2",
      repository,
      base,
      ...field("target", target.value === void 0 ? void 0 : parseNamed(target.value)),
      threshold,
      frame,
      candidates,
      lanes,
      commits,
      grid
    };
    return isConsistent(graph) ? graph : void 0;
  }

  // src/webview/common.ts
  var REPOSITORY_URL = "https://github.com/axean/sundgren";
  var AUTHOR = "Axel E. Andersson";
  function element(tag, className, text) {
    const node = document.createElement(tag);
    node.className = className;
    if (text !== void 0) {
      node.textContent = text;
    }
    return node;
  }
  function percent(value) {
    return value === void 0 ? "\u2013" : `${String(Math.round(value * 100))}%`;
  }
  function short(oid) {
    return oid.slice(0, 7);
  }
  function info(text, help) {
    const node = element("span", "info");
    node.append(text, element("span", "info-mark", "\u24D8"));
    node.title = help;
    return node;
  }
  function footer(meta) {
    const node = element("footer", "footer");
    const repository = element("a", "", "github.com/axean/sundgren");
    repository.href = REPOSITORY_URL;
    const parts = [element("span", "footer-name", "sundgren")];
    if (meta !== void 0) {
      parts.push(`extension ${meta.version} (${meta.commit})`, meta.binary);
    }
    parts.push(repository, `by ${AUTHOR}`);
    for (const [index, part] of parts.entries()) {
      if (index > 0) {
        node.append(element("span", "footer-dot", "\xB7"));
      }
      node.append(part);
    }
    return node;
  }
  function controlButton(label, value, help, onClick) {
    const button = element("button", "control");
    button.type = "button";
    button.title = help;
    const shown = element("span", "control-value", value);
    shown.title = value;
    button.append(element("span", "control-label", label), shown);
    button.append(element("span", "control-caret", "\u25BE"));
    button.addEventListener("click", onClick);
    return button;
  }
  function badge(tone, text, help) {
    const node = element("span", `badge ${tone}`, text);
    node.title = help;
    return node;
  }
  function syncBadge(branch, where) {
    if (where.isRemote) {
      return badge("muted", "remote", `Only on the remote: there is no local ${branch}.`);
    }
    if (where.upstream === void 0) {
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
    const arrows = [ahead > 0 ? `\u2191${String(ahead)}` : "", behind > 0 ? `\u2193${String(behind)}` : ""];
    return badge(
      "warn",
      arrows.filter((arrow) => arrow !== "").join(" "),
      `${String(ahead)} commits not on ${where.upstream}; ${String(behind)} on ${where.upstream} not here.`
    );
  }
  function worktreeBadges(where) {
    if (where.worktree === void 0) {
      return [];
    }
    const badges = [badge("muted", "worktree", `Checked out in ${where.worktree}`)];
    const changes = where.changes;
    if (changes !== void 0) {
      const total = changes.changed + changes.untracked;
      badges.push(
        total === 0 ? badge("good dot", "\u25CF", "No uncommitted changes in its work tree.") : badge(
          "bad dot",
          "\u25CF",
          `Uncommitted: ${String(changes.changed)} changed, ${String(changes.untracked)} untracked files. Not part of the comparison: only commits are compared.`
        )
      );
    }
    return badges;
  }

  // src/webview/format.ts
  function pad(n2) {
    return String(n2).padStart(2, "0");
  }
  function formatTime(seconds) {
    const d = new Date(seconds * 1e3);
    const date = `${String(d.getFullYear())}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    return `${date} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  // src/webview/geometry.ts
  var CELL_WIDTH = 9;
  var CELL_HEIGHT = 22;
  var NODE_RADIUS = 4.5;
  var GUTTER = 36;
  var CELLS_PER_COLUMN = 3;
  var EXTRA_CELLS = 3;
  var MAX_LABEL_ROWS = 3;
  var MAX_LABEL_LENGTH = 24;
  var LABEL_CHARACTER_WIDTH = 6.6;
  function columnsFor(pixels) {
    const cells = Math.floor(pixels / CELL_WIDTH);
    return Math.max(1, Math.floor((cells - EXTRA_CELLS) / CELLS_PER_COLUMN));
  }
  function cellX(col) {
    return GUTTER + col * CELL_WIDTH;
  }
  function n(value) {
    return String(Math.round(value * 100) / 100);
  }
  function markers(graph) {
    const reference = graph.lanes.findIndex((lane) => lane.kind === "reference");
    const drawn = new Set(graph.lanes.map((lane) => lane.name));
    const found = graph.commits.filter((commit) => commit.lanes.includes(reference)).map((commit) => ({
      col: commit.column,
      names: commit.refs.filter((name) => !drawn.has(name))
    })).filter((marker) => marker.names.length > 0).toSorted((a, b) => a.col - b.col);
    const ends = /* @__PURE__ */ new Map();
    return found.map((marker) => {
      const joined = marker.names.join(", ");
      const label = joined.length > MAX_LABEL_LENGTH ? `${joined.slice(0, MAX_LABEL_LENGTH - 1)}\u2026` : joined;
      const start = cellX(marker.col);
      const end = start + (label.length + 1) * LABEL_CHARACTER_WIDTH + CELL_WIDTH;
      let labelRow = 0;
      while (labelRow < MAX_LABEL_ROWS - 1 && (ends.get(labelRow) ?? 0) > start) {
        labelRow += 1;
      }
      ends.set(labelRow, Math.max(ends.get(labelRow) ?? 0, end));
      return { ...marker, label, labelRow };
    });
  }
  function rows(graph, found) {
    const inserted = Math.max(0, ...found.map((marker) => marker.labelRow + 1));
    return {
      y: (row) => (row + inserted) * CELL_HEIGHT,
      labelY: (labelRow) => (inserted - 1 - labelRow) * CELL_HEIGHT,
      height: (graph.grid.height + inserted) * CELL_HEIGHT,
      inserted
    };
  }
  function opposite(side) {
    return (side << 2 | side >> 2) & 15;
  }
  var SIDES = [UP, RIGHT, DOWN, LEFT];
  function sideCount(sides) {
    return SIDES.filter((side) => (sides & side) !== 0).length;
  }
  function step(side) {
    switch (side) {
      case UP: {
        return [-1, 0];
      }
      case DOWN: {
        return [1, 0];
      }
      case LEFT: {
        return [0, -1];
      }
      default: {
        return [0, 1];
      }
    }
  }
  function centre(cell, placement) {
    return {
      x: cellX(cell.col) + CELL_WIDTH / 2,
      y: placement.y(cell.row) + CELL_HEIGHT / 2
    };
  }
  function toward(cell, placement, side, distance) {
    const c = centre(cell, placement);
    const [down, right] = step(side);
    const d = distance ?? (down === 0 ? CELL_WIDTH : CELL_HEIGHT) / 2;
    return { x: c.x + right * d, y: c.y + down * d };
  }
  function point(p) {
    return `${n(p.x)} ${n(p.y)}`;
  }
  function isRun(cell) {
    return sideCount(cell.sides) === 2;
  }
  function spokes(cell, placement) {
    return SIDES.filter((side) => (cell.sides & side) !== 0).map((side) => `M${point(centre(cell, placement))}L${point(toward(cell, placement, side))}`).join("");
  }
  function lanePaths(cells, placement) {
    const at = /* @__PURE__ */ new Map();
    for (const cell of cells) {
      at.set(`${String(cell.row)},${String(cell.col)}`, cell);
    }
    const next = (cell, side) => {
      const [down, right] = step(side);
      const neighbour = at.get(`${String(cell.row + down)},${String(cell.col + right)}`);
      return neighbour?.lane === cell.lane && (neighbour.sides & opposite(side)) !== 0 && isRun(cell) && isRun(neighbour) ? neighbour : void 0;
    };
    const parts = /* @__PURE__ */ new Map();
    const add = (lane, part) => {
      parts.set(lane, (parts.get(lane) ?? "") + part);
    };
    const seen = /* @__PURE__ */ new Set();
    const walk = (start, entry) => {
      let cell = start;
      let from = entry;
      let d = `M${point(toward(start, placement, entry))}`;
      while (cell !== void 0) {
        seen.add(cell);
        const exit = cell.sides & ~from;
        if (exit !== opposite(from)) {
          const r = CELL_WIDTH / 2;
          d += `L${point(toward(cell, placement, from, r))}`;
          d += `Q${point(centre(cell, placement))} ${point(toward(cell, placement, exit, r))}`;
        }
        const following = next(cell, exit);
        if (following === void 0 || seen.has(following)) {
          d += `L${point(toward(cell, placement, exit))}`;
        } else {
          from = opposite(exit);
        }
        cell = following !== void 0 && seen.has(following) ? void 0 : following;
      }
      add(start.lane, d);
    };
    for (const cell of cells) {
      if (!isRun(cell)) {
        add(cell.lane, spokes(cell, placement));
      }
    }
    for (const cell of cells) {
      const open = SIDES.find((side) => (cell.sides & side) !== 0 && next(cell, side) === void 0);
      if (open !== void 0 && isRun(cell) && !seen.has(cell)) {
        walk(cell, open);
      }
    }
    for (const cell of cells) {
      if (isRun(cell) && !seen.has(cell)) {
        walk(cell, SIDES.find((side) => (cell.sides & side) !== 0) ?? UP);
      }
    }
    return parts;
  }

  // src/webview/main.ts
  var SVG = "http://www.w3.org/2000/svg";
  var FAMILIES = ["orange", "forest", "wine", "sage", "rose", "brown", "pink"];
  var MAX_BRANCHES = 7;
  var RESIZE_DELAY_MS = 250;
  var PAGE_PADDING = 24;
  var LABEL_GAP = 12;
  var LABEL_SHARE = 0.38;
  var LABEL_MIN = 200;
  var LABEL_MAX = 360;
  var vscode = acquireVsCodeApi();
  function send(message) {
    vscode.postMessage(message);
  }
  var state = {
    base: "",
    target: void 0,
    branches: 3,
    meta: void 0,
    states: /* @__PURE__ */ new Map(),
    graph: void 0,
    view: void 0,
    isRunning: false,
    progress: "",
    error: void 0,
    selected: void 0
  };
  function svgElement(tag, attributes) {
    const node = document.createElementNS(SVG, tag);
    for (const [name, value] of Object.entries(attributes)) {
      node.setAttribute(name, String(value));
    }
    return node;
  }
  function svgTitle(text) {
    const title = svgElement("title", {});
    title.textContent = text;
    return title;
  }
  function labelWidth(width) {
    return Math.round(Math.min(LABEL_MAX, Math.max(LABEL_MIN, width * LABEL_SHARE)));
  }
  function columnsNow() {
    const width = document.documentElement.clientWidth;
    return columnsFor(width - PAGE_PADDING - GUTTER - LABEL_GAP - labelWidth(width));
  }
  function ask(view) {
    const last = state.view;
    if (last?.offset === view.offset && last.columns === view.columns && last.branches === view.branches) {
      return;
    }
    state.view = view;
    send({ type: "view", ...view });
  }
  function refit() {
    ask({
      offset: state.view?.offset ?? 0,
      columns: columnsNow(),
      branches: state.branches
    });
  }
  function laneClasses(lane) {
    const classes = [lane.kind === "remote" ? "remote" : "local"];
    if (lane.protected || lane.family === void 0) {
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
  function compact(count) {
    return count < 1e3 ? String(count) : `${String(Math.round(count / 100) / 10)}k`;
  }
  function selectionKey(selection) {
    return selection.kind === "commit" ? `commit:${selection.oid}` : `lane:${selection.name}`;
  }
  function laneSummary(lane, graph) {
    if (lane.kind === "reference") {
      return `${lane.name}: the base, the line every other lane is placed against.`;
    }
    const parts = [
      `${lane.name}: ${String(lane.ahead)} commits not in ${graph.base.name}, ${String(lane.behind)} of ${graph.base.name} not in it.`
    ];
    if (lane.merged) {
      parts.push("Merged: nothing ahead of the base.");
    }
    if (lane.only === "remote") {
      parts.push("Only on the remote.");
    }
    if (lane.score !== void 0 && graph.target !== void 0) {
      parts.push(`${percent(lane.score)} of its work is in ${graph.target.name}.`);
    }
    if (lane.before !== void 0) {
      parts.push(`It forked before the frame: ${String(lane.before)} of its commits are earlier.`);
    }
    if (lane.after) {
      parts.push("It goes on after the frame.");
    }
    return parts.join(" ");
  }
  function commitSummary(commit) {
    return [
      commit.subject,
      `${short(commit.oid)} \xB7 ${commit.author} \xB7 ${formatTime(commit.committed)}`,
      commit.parents.length > 1 ? `merge of ${String(commit.parents.length)} parents` : "",
      commit.refs.length > 0 ? commit.refs.join(", ") : ""
    ].filter((line) => line !== "").join("\n");
  }
  function centre2(cell, placement) {
    return {
      x: cellX(cell.col) + CELL_WIDTH / 2,
      y: placement.y(cell.row) + CELL_HEIGHT / 2
    };
  }
  function commitNode(graph, cell, placement) {
    const commit = cell.commit === void 0 ? void 0 : graph.commits[cell.commit];
    const lane = graph.lanes[cell.lane];
    if (commit === void 0 || lane === void 0) {
      return void 0;
    }
    const { x, y } = centre2(cell, placement);
    const node = svgElement("g", {
      class: ["commit", ...laneClasses(lane)].join(" "),
      tabindex: 0,
      role: "button",
      "aria-label": `Commit ${short(commit.oid)} on ${lane.name}: ${commit.subject}`
    });
    node.dataset["oid"] = commit.oid;
    node.dataset["lane"] = String(cell.lane);
    node.append(
      svgElement("circle", { class: "focus", cx: x, cy: y, r: NODE_RADIUS + 3 }),
      svgElement("circle", {
        class: lane.kind === "reference" ? "node filled" : "node",
        cx: x,
        cy: y,
        r: NODE_RADIUS
      }),
      svgTitle(commitSummary(commit))
    );
    return node;
  }
  function connectorNode(graph, cell, placement) {
    const lane = graph.lanes[cell.lane];
    if (lane === void 0) {
      return void 0;
    }
    const { x, y } = centre2(cell, placement);
    const isLeft = cell.col === 0;
    const node = svgElement("g", { class: ["connector", ...laneClasses(lane)].join(" ") });
    node.dataset["lane"] = String(cell.lane);
    node.append(
      svgElement("circle", { class: "pad", cx: x, cy: y, r: NODE_RADIUS }),
      svgElement("circle", { class: "pad-hole", cx: x, cy: y, r: 1.5 })
    );
    let help = `${lane.name} goes on after the frame.`;
    if (isLeft) {
      const before = lane.before ?? 0;
      help = before > 0 ? `${lane.name} goes on before the frame: ${String(before)} earlier commits of its own.` : `${lane.name} goes on before the frame.`;
      if (before > 0) {
        const count = svgElement("text", {
          class: "count",
          x: x - NODE_RADIUS - 3,
          y: y + 4,
          "text-anchor": "end"
        });
        count.textContent = compact(before);
        node.append(count);
      }
    }
    node.append(svgTitle(help));
    return node;
  }
  function markerNodes(graph, placement) {
    const group = svgElement("g", { class: "markers" });
    for (const marker of markers(graph)) {
      const x = cellX(marker.col) + CELL_WIDTH / 2;
      const y = placement.labelY(marker.labelRow) + CELL_HEIGHT * 0.7;
      const arrow = svgElement("text", { class: "arrow", x, y, "text-anchor": "middle" });
      arrow.textContent = "\u25BC";
      const label = svgElement("text", { class: "marker-label", x: x + LABEL_CHARACTER_WIDTH, y });
      label.textContent = marker.label;
      const item = svgElement("g", { class: "marker" });
      item.append(arrow, label, svgTitle(marker.names.join("\n")));
      group.append(item);
    }
    return group;
  }
  function laneLabel(graph, lane, index, top) {
    const label = element("div", ["lane-label", ...laneClasses(lane)].join(" "));
    label.dataset["lane"] = String(index);
    label.tabIndex = 0;
    label.setAttribute("role", "button");
    label.style.top = `${String(top)}px`;
    label.title = laneSummary(lane, graph);
    label.append(element("span", "lane-name", lane.name));
    const facts = [
      lane.kind === "reference" ? "base" : `\u2191${String(lane.ahead)} \u2193${String(lane.behind)}`
    ];
    if (lane.merged) {
      facts.push("merged");
    }
    if (lane.name === graph.target?.name) {
      facts.push("target");
    } else if (lane.pinned && lane.kind !== "reference") {
      facts.push("pinned");
    }
    if (lane.score !== void 0 && graph.target !== void 0) {
      facts.push(`${percent(lane.score)} in ${graph.target.name}`);
    }
    label.append(element("span", "lane-facts", facts.join(" \xB7 ")));
    const where = state.states.get(lane.name);
    if (where !== void 0) {
      const badges = element("span", "badges");
      badges.append(syncBadge(lane.name, where), ...worktreeBadges(where));
      label.append(badges);
    }
    return label;
  }
  function drawMap(graph) {
    const placement = rows(graph, markers(graph));
    const width = cellX(graph.grid.width);
    const map = element("div", "map");
    map.style.setProperty("--grid-width", `${String(width + LABEL_GAP)}px`);
    map.style.setProperty(
      "--label-width",
      `${String(labelWidth(document.documentElement.clientWidth))}px`
    );
    map.style.setProperty("--cell-height", `${String(CELL_HEIGHT)}px`);
    map.style.height = `${String(placement.height)}px`;
    const svg = svgElement("svg", {
      class: "map-svg",
      width,
      height: placement.height,
      role: "group",
      "aria-label": `Branches around ${graph.base.name}`
    });
    const traces = lanePaths(graph.grid.cells, placement);
    for (const [index, lane] of graph.lanes.entries()) {
      const d = traces.get(index);
      if (d === void 0) {
        continue;
      }
      const group = svgElement("g", { class: ["lane", ...laneClasses(lane)].join(" ") });
      group.dataset["lane"] = String(index);
      group.append(
        svgElement("path", { class: "hit", d }),
        svgElement("path", { class: "trace", d }),
        svgTitle(laneSummary(lane, graph))
      );
      svg.append(group);
    }
    const nodes = svgElement("g", { class: "nodes" });
    for (const cell of graph.grid.cells) {
      let node;
      if (cell.node === "commit") {
        node = commitNode(graph, cell, placement);
      } else if (cell.node === "connector") {
        node = connectorNode(graph, cell, placement);
      }
      if (node !== void 0) {
        nodes.append(node);
      }
    }
    svg.append(nodes, markerNodes(graph, placement));
    const labels = element("div", "lane-labels");
    labels.append(
      ...graph.lanes.map((lane, index) => laneLabel(graph, lane, index, placement.y(lane.row)))
    );
    map.append(svg, labels);
    return map;
  }
  function showSelection(root) {
    for (const node of root.querySelectorAll(".selected")) {
      node.classList.remove("selected");
    }
    const graph = state.graph;
    const selected = state.selected;
    if (graph === void 0 || selected === void 0) {
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
  function selectionAt(target) {
    if (!(target instanceof Element) || state.graph === void 0) {
      return void 0;
    }
    const commit = target.closest(".commit");
    const oid = commit instanceof SVGElement ? commit.dataset["oid"] : void 0;
    if (oid !== void 0) {
      return { kind: "commit", oid };
    }
    const owner = target.closest("[data-lane]");
    const index = owner instanceof HTMLElement || owner instanceof SVGElement ? owner.dataset["lane"] : void 0;
    const lane = index === void 0 ? void 0 : state.graph.lanes[Number(index)];
    return lane === void 0 ? void 0 : { kind: "lane", name: lane.name };
  }
  function select(selection, root) {
    state.selected = selectionKey(selection);
    showSelection(root);
    send({ type: "select", selection });
  }
  function highlight(root, target) {
    const owner = target instanceof Element ? target.closest("[data-lane]") : void 0;
    const index = owner instanceof HTMLElement || owner instanceof SVGElement ? owner.dataset["lane"] : void 0;
    for (const node of root.querySelectorAll(".hot")) {
      node.classList.remove("hot");
    }
    if (index !== void 0) {
      for (const node of root.querySelectorAll(`[data-lane="${CSS.escape(index)}"]`)) {
        node.classList.add("hot");
      }
    }
  }
  function listen(map) {
    map.addEventListener("click", (event) => {
      const selection = selectionAt(event.target);
      if (selection !== void 0) {
        select(selection, map);
      }
    });
    map.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") {
        return;
      }
      const selection = selectionAt(event.target);
      if (selection === void 0) {
        return;
      }
      event.preventDefault();
      select(selection, map);
    });
    for (const type of ["pointerover", "focusin"]) {
      map.addEventListener(type, (event) => {
        highlight(map, event.target);
      });
    }
    for (const type of ["pointerleave", "focusout"]) {
      map.addEventListener(type, () => {
        highlight(map, void 0);
      });
    }
  }
  function swatch(classes, draw) {
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
          r: NODE_RADIUS
        })
      );
    }
    svg.append(group);
    return svg;
  }
  function viaSwatch() {
    const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
    const under = svgElement("g", { class: "lane protected" });
    under.append(svgElement("path", { class: "trace", d: "M2 7H9.5M18.5 7H26" }));
    const over = svgElement("g", { class: "lane family-0" });
    over.append(svgElement("path", { class: "trace", d: "M14 0V14" }));
    svg.append(under, over);
    return svg;
  }
  function connectorSwatch() {
    const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
    const group = svgElement("g", { class: "connector protected" });
    group.append(
      svgElement("path", { class: "trace", d: "M14 7H27" }),
      svgElement("circle", { class: "pad", cx: 14, cy: 7, r: NODE_RADIUS }),
      svgElement("circle", { class: "pad-hole", cx: 14, cy: 7, r: 1.5 })
    );
    svg.append(group);
    return svg;
  }
  function legendItem(symbol, text) {
    const item = element("span", "legend-item");
    item.append(symbol, element("span", "", text));
    return item;
  }
  function hasVia(graph) {
    const at = new Map(
      graph.grid.cells.map((cell) => [`${String(cell.row)},${String(cell.col)}`, cell])
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
  function legend(graph) {
    const node = element("div", "map-legend");
    const branches = /* @__PURE__ */ new Map();
    for (const lane of graph.lanes) {
      if (!lane.protected && lane.family !== void 0) {
        branches.set(lane.branch, [...branches.get(lane.branch) ?? [], lane]);
      }
    }
    for (const [branch, lanes] of branches) {
      const item = element("span", "legend-item");
      for (const lane of lanes) {
        const classes = laneClasses(lane).filter((name) => name !== "merged" && name !== "dashed");
        item.append(swatch(classes, "line"));
      }
      const kinds = lanes.map((lane) => lane.kind === "remote" ? "remote" : "local").join(", ");
      item.append(element("span", "", `${branch} (${kinds})`));
      node.append(item);
    }
    const isUsed = (hasIt) => graph.lanes.some((lane) => hasIt(lane));
    if (isUsed((lane) => lane.twin !== void 0 && lane.family !== void 0)) {
      node.append(element("span", "legend-note", "A remote twin: the same colour, turned in hue."));
    }
    const protectedNames = graph.lanes.filter((lane) => lane.protected || lane.family === void 0);
    if (protectedNames.length > 0) {
      node.append(
        legendItem(
          swatch(["protected"], "line"),
          `protected: ${protectedNames.map((lane) => lane.name).join(", ")}`
        )
      );
    }
    if (isUsed((lane) => lane.merged)) {
      node.append(
        legendItem(swatch(["protected", "merged"], "line"), "faded: merged, nothing ahead")
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
      legendItem(swatch(["family-0"], "ring"), "commit of another lane")
    );
    if (hasVia(graph)) {
      node.append(legendItem(viaSwatch(), "via: a lane passing under another, not joined"));
    }
    if (graph.grid.cells.some((cell) => cell.node === "connector")) {
      node.append(
        legendItem(connectorSwatch(), "goes on outside the frame; the number: earlier commits")
      );
    }
    if (markers(graph).length > 0) {
      node.append(
        legendItem(element("span", "legend-arrow", "\u25BC"), `branches and tags on ${graph.base.name}`)
      );
    }
    return node;
  }
  function branchesSelect() {
    const label = element("label", "control");
    label.title = "How many branches to show besides the base and the target.";
    const select2 = element("select", "control-select");
    for (let count = 1; count <= MAX_BRANCHES; count++) {
      const option = element("option", "", String(count));
      option.value = String(count);
      option.selected = count === state.branches;
      select2.append(option);
    }
    select2.addEventListener("change", () => {
      state.branches = Number(select2.value);
      refit();
    });
    label.append(element("span", "control-label", "Branches"), select2);
    return label;
  }
  function stepButton(text, help, offset) {
    const button = element("button", "control step", text);
    button.type = "button";
    button.title = help;
    button.disabled = offset === void 0;
    button.addEventListener("click", () => {
      if (offset !== void 0) {
        ask({ offset, columns: columnsNow(), branches: state.branches });
      }
    });
    return button;
  }
  function header() {
    const graph = state.graph;
    const node = element("header", "header stacked");
    const heading = element("div", "heading");
    heading.append(element("span", "title", `History around ${state.base}`));
    if (graph !== void 0) {
      const others = graph.lanes.filter((lane) => lane.kind !== "reference").length;
      heading.append(
        element(
          "span",
          "count",
          `${String(others)} lanes \xB7 ${String(graph.candidates)} branches in the frame`
        )
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
        }
      ),
      controlButton(
        "Target",
        state.target ?? "none",
        "The branch the others are ranked against: the branches whose work is most in it are shown. Click to pick another.",
        () => {
          send({ type: "choose", control: "target" });
        }
      ),
      branchesSelect(),
      stepButton("\u25C0", "Older: one step back in time", graph?.frame.older),
      stepButton("\u25B6", "Newer: one step forward in time", graph?.frame.newer),
      info(
        state.target === void 0 ? "newest branches" : `most in ${state.target}`,
        state.target === void 0 ? "Without a target, the branches with the newest tips in the frame are shown." : `Of the branches visible in the frame, those whose work is most found in ${state.target} are shown (containment, as in the Containment panel).`
      )
    );
    node.append(heading, toolbar);
    return node;
  }
  function render() {
    const root = document.querySelector("#root");
    if (root === null) {
      return;
    }
    const parts = [header()];
    const graph = state.graph;
    if (state.error !== void 0) {
      parts.push(element("p", "error", state.error));
    } else if (graph === void 0) {
      parts.push(element("p", "empty", state.isRunning ? "Reading the history\u2026" : ""));
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
  function receive(message) {
    switch (message.type) {
      case "setup": {
        state.base = message.base;
        state.target = message.target;
        state.branches = message.branches;
        state.meta = message.meta;
        state.states = new Map(message.states.map((branch) => [branch.name, branch]));
        state.graph = void 0;
        state.view = message.view;
        state.isRunning = false;
        state.error = void 0;
        state.selected = void 0;
        render();
        if (message.view === void 0) {
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
        const graph = parseGraph(message.graph);
        state.isRunning = false;
        state.progress = "";
        state.graph = graph;
        state.error = graph === void 0 ? "This isn't a sundgren.graph/2 document." : void 0;
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
    if (message.type !== "graph" || state.graph === void 0) {
      return;
    }
    send({ type: "rendered", oid: state.graph.base.oid, rows: state.graph.lanes.length });
    refit();
  }
  var resizing = {};
  new ResizeObserver(() => {
    clearTimeout(resizing.timer);
    resizing.timer = setTimeout(() => {
      if (state.view !== void 0) {
        refit();
      }
    }, RESIZE_DELAY_MS);
  }).observe(document.documentElement);
  addEventListener("message", (event) => {
    if (event.origin !== origin) {
      return;
    }
    receive(event.data);
  });
  send({ type: "ready" });
})();
//# sourceMappingURL=webview.js.map
