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
  function isNamed(value) {
    return parseNamed(value) !== void 0;
  }
  var SHOW_MODES = /* @__PURE__ */ new Set(["local", "recent", "mine", "like"]);
  var LANE_KINDS = /* @__PURE__ */ new Set(["reference", "local", "remote"]);
  var SIDES_ONLY = /* @__PURE__ */ new Set(["local", "remote"]);
  var NODES = /* @__PURE__ */ new Set(["commit", "connector"]);
  function isShowMode(value) {
    return SHOW_MODES.has(value);
  }
  function isLaneKind(value) {
    return LANE_KINDS.has(value);
  }
  function isOnly(value) {
    return SIDES_ONLY.has(value);
  }
  function isNode(value) {
    return NODES.has(value);
  }
  function parseShow(value) {
    if (!isObject(value) || !isShowMode(value["mode"])) {
      return void 0;
    }
    const like = new Optional(value["like"], isNamed);
    const named = like.value === void 0 ? void 0 : parseNamed(like.value);
    return like.isValid ? { mode: value["mode"], ...field("like", named) } : void 0;
  }
  function parsePage(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { commits, more } = value;
    return typeof more === "boolean" && isCount(commits) && commits >= 1 ? { commits, more } : void 0;
  }
  function laneBasics(value) {
    const { name, branch, kind, column, label_row, ahead, behind, tip } = value;
    const flags = [value["protected"], value["pinned"], value["merged"]];
    if (!isString(name) || !isString(branch) || !isLaneKind(kind) || !isCount(column) || !isCount(label_row) || !isCount(ahead) || !isCount(behind) || !isOid(tip) || flags.some((flag) => typeof flag !== "boolean")) {
      return void 0;
    }
    return {
      name,
      branch,
      kind,
      column,
      label_row,
      protected: value["protected"] === true,
      pinned: value["pinned"] === true,
      merged: value["merged"] === true,
      ahead,
      behind,
      tip
    };
  }
  function parseLane(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const basics = laneBasics(value);
    const family = new Optional(value["family"], isCount);
    const twin = new Optional(value["twin"], isCount);
    const remote = new Optional(value["remote"], isString);
    const only = new Optional(value["only"], isOnly);
    const score = new Optional(value["score"], isNumber);
    const fork = new Optional(value["fork"], isOid);
    const before = new Optional(value["before"], isCount);
    const options = [family, twin, remote, only, score, fork, before];
    if (basics === void 0 || options.some((option) => !option.isValid)) {
      return void 0;
    }
    return {
      ...basics,
      ...field("family", family.value),
      ...field("twin", twin.value),
      ...field("remote", remote.value),
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
    const { oid, parents, author, committed, subject, body, refs, row, lanes } = value;
    return isOid(oid) && isArrayOf(parents, isOid) && isString(author) && isNumber(committed) && Number.isSafeInteger(committed) && isString(subject) && isString(body) && isArrayOf(refs, isString) && isCount(row) && isArrayOf(lanes, isCount) ? { oid, parents, author, committed, subject, body, refs, row, lanes } : void 0;
  }
  function parseFold(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { lane, rows, commits } = value;
    if (!isCount(lane) || !isArrayOf(rows, isCount) || !isArrayOf(commits, isCount)) {
      return void 0;
    }
    const [first, last] = rows;
    return first !== void 0 && last !== void 0 && rows.length === 2 && first <= last ? { lane, rows: [first, last], commits } : void 0;
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
    const { lanes, commits, folds, grid } = graph;
    const isLaneIndex = (index) => index < lanes.length;
    const isCommitIndex = (index) => index < commits.length;
    return lanes.every(
      (lane) => lane.column < grid.width && lane.label_row < grid.height && (lane.twin === void 0 || isLaneIndex(lane.twin))
    ) && commits.every(
      (commit) => commit.row < grid.height && commit.lanes.every((lane) => isLaneIndex(lane))
    ) && folds.every(
      (fold) => isLaneIndex(fold.lane) && fold.rows[1] < grid.height && fold.commits.every((commit) => isCommitIndex(commit))
    ) && grid.cells.every(
      (cell) => cell.row < grid.height && cell.col < grid.width && isLaneIndex(cell.lane) && (cell.commit === void 0 || isCommitIndex(cell.commit))
    );
  }
  function parseGraph(value) {
    if (!isObject(value) || value["schema"] !== "sundgren.graph/3") {
      return void 0;
    }
    const { repository, threshold, branches } = value;
    const base = parseNamed(value["base"]);
    const show = parseShow(value["show"]);
    const page = parsePage(value["page"]);
    const lanes = parseAll(value["lanes"], parseLane);
    const commits = parseAll(value["commits"], parseCommit);
    const folds = parseAll(value["folds"], parseFold);
    const grid = parseGrid(value["grid"]);
    if (base === void 0 || show === void 0 || page === void 0 || lanes === void 0 || commits === void 0 || folds === void 0 || grid === void 0 || !isString(repository) || !isNumber(threshold) || !isArrayOf(branches, isString)) {
      return void 0;
    }
    const graph = {
      schema: "sundgren.graph/3",
      repository,
      base,
      show,
      threshold,
      page,
      branches,
      lanes,
      commits,
      folds,
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
  function syncBadge(branch, where2) {
    if (where2.isRemote) {
      return badge("muted", "remote", `Only on the remote: there is no local ${branch}.`);
    }
    if (where2.upstream === void 0) {
      return badge("muted", "local", "Only local: not on any remote.");
    }
    if (where2.isGone === true) {
      return badge("bad", "upstream gone", `${where2.upstream} no longer exists.`);
    }
    const ahead = where2.ahead ?? 0;
    const behind = where2.behind ?? 0;
    if (ahead === 0 && behind === 0) {
      return badge("good", "in sync", `The same as ${where2.upstream}.`);
    }
    const arrows = [ahead > 0 ? `\u2191${String(ahead)}` : "", behind > 0 ? `\u2193${String(behind)}` : ""];
    return badge(
      "warn",
      arrows.filter((arrow) => arrow !== "").join(" "),
      `${String(ahead)} commits not on ${where2.upstream}; ${String(behind)} on ${where2.upstream} not here.`
    );
  }
  function worktreeBadges(where2) {
    if (where2.worktree === void 0) {
      return [];
    }
    const badges = [badge("muted", "worktree", `Checked out in ${where2.worktree}`)];
    const changes = where2.changes;
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
  var MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  function clock(seconds) {
    const d = new Date(seconds * 1e3);
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  function dayKey(seconds) {
    return formatTime(seconds).slice(0, 10);
  }
  function dayLabel(seconds, year) {
    const d = new Date(seconds * 1e3);
    const day = `${MONTHS[d.getMonth()] ?? ""} ${String(d.getDate())}`;
    return d.getFullYear() === year ? day : `${day} '${String(d.getFullYear() % 100).padStart(2, "0")}`;
  }

  // src/webview/geometry.ts
  var CELL_WIDTH = 8;
  var ROW_HEIGHT = 24;
  var SPACER_HEIGHT = 8;
  var NODE_RADIUS = 4;
  var GUTTER = 6;
  var VIA_GAP = 3.5;
  function cellX(col) {
    return GUTTER + col * CELL_WIDTH;
  }
  function n(value) {
    return String(Math.round(value * 100) / 100);
  }
  function hiddenRows(graph, folded) {
    const hidden = /* @__PURE__ */ new Set();
    for (const [index, fold] of graph.folds.entries()) {
      if (folded.has(index)) {
        for (let row = fold.rows[0] + 1; row <= fold.rows[1]; row++) {
          hidden.add(row);
        }
      }
    }
    return hidden;
  }
  function shownAt(row, hidden) {
    let shown = row;
    while (shown > 0 && hidden.has(shown)) {
      shown -= 1;
    }
    return shown;
  }
  function layout(graph, folded) {
    const hidden = hiddenRows(graph, folded);
    const labels = /* @__PURE__ */ new Map();
    const byColumn = graph.lanes.map((lane, index) => ({ lane, index })).toSorted((a, b) => a.lane.column - b.lane.column);
    for (const { lane, index } of byColumn) {
      const row = shownAt(lane.label_row, hidden);
      labels.set(row, [...labels.get(row) ?? [], index]);
    }
    const tall = new Set(labels.keys());
    for (const cell of graph.grid.cells) {
      if (cell.node !== void 0 && !hidden.has(cell.row)) {
        tall.add(cell.row);
      }
    }
    const tops = [];
    const heights = [];
    let top = 0;
    for (let row = 0; row < graph.grid.height; row++) {
      let height = tall.has(row) ? ROW_HEIGHT * Math.max(1, labels.get(row)?.length ?? 0) : 0;
      if (!tall.has(row) && !hidden.has(row)) {
        height = SPACER_HEIGHT;
      }
      tops.push(top);
      heights.push(height);
      top += height;
    }
    return {
      y: (row) => tops.at(row) ?? top,
      h: (row) => heights.at(row) ?? 0,
      height: top,
      rows: [...tall].toSorted((a, b) => a - b),
      labels
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
  function centre(row, col, placement) {
    return { x: cellX(col) + CELL_WIDTH / 2, y: placement.y(row) + placement.h(row) / 2 };
  }
  function toward(cell, placement, side, distance) {
    const c = centre(cell.row, cell.col, placement);
    const [down, right] = step(side);
    const d = distance ?? (down === 0 ? CELL_WIDTH : placement.h(cell.row)) / 2;
    return { x: c.x + right * d, y: c.y + down * d };
  }
  function point(p) {
    return `${n(p.x)} ${n(p.y)}`;
  }
  function isRun(cell) {
    return sideCount(cell.sides) === 2;
  }
  function spokes(cell, placement) {
    const c = point(centre(cell.row, cell.col, placement));
    return SIDES.filter((side) => (cell.sides & side) !== 0).map((side) => `M${c}L${point(toward(cell, placement, side))}`).join("");
  }
  function cellIndex(cells) {
    const at = /* @__PURE__ */ new Map();
    for (const cell of cells) {
      at.set(`${String(cell.row)},${String(cell.col)}`, cell);
    }
    return (row, col) => at.get(`${String(row)},${String(col)}`);
  }
  function isAcross(cell) {
    return cell?.node === void 0 && cell?.sides === (LEFT | RIGHT);
  }
  function brokenEnd(at, row, col, side) {
    const [down] = step(side);
    let next = row + down;
    for (; ; ) {
      const cell = at(next, col);
      if (cell !== void 0 && (cell.sides & side) !== 0 && (cell.sides & opposite(side)) === 0) {
        return cell;
      }
      const isBlank = cell === void 0 || cell.sides === 0;
      if (!isBlank || !isAcross(at(next + down, col))) {
        return void 0;
      }
      next += 2 * down;
    }
  }
  function vias(cells) {
    const at = cellIndex(cells);
    const found = [];
    for (const cell of cells) {
      if (!isAcross(cell)) {
        continue;
      }
      const above = brokenEnd(at, cell.row, cell.col, UP);
      const below = brokenEnd(at, cell.row, cell.col, DOWN);
      if (above !== void 0 && above.lane === below?.lane && above.lane !== cell.lane) {
        found.push({ row: cell.row, col: cell.col, lane: above.lane });
      }
    }
    return found;
  }
  function bridge(via, placement) {
    const top = centre(via.row - 1, via.col, placement);
    const middle = centre(via.row, via.col, placement);
    const bottom = centre(via.row + 1, via.col, placement);
    const gap = Math.min(VIA_GAP, placement.h(via.row) / 2);
    return `M${point(top)}L${point({ x: middle.x, y: middle.y - gap })}M${point({ x: middle.x, y: middle.y + gap })}L${point(bottom)}`;
  }
  function lanePaths(cells, placement) {
    const at = cellIndex(cells);
    const next = (cell, side) => {
      const [down, right] = step(side);
      const neighbour = at(cell.row + down, cell.col + right);
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
          const r = Math.min(CELL_WIDTH, placement.h(cell.row)) / 2;
          d += `L${point(toward(cell, placement, from, r))}`;
          d += `Q${point(centre(cell.row, cell.col, placement))} ${point(toward(cell, placement, exit, r))}`;
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
      if (!isRun(cell) && cell.sides !== 0) {
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
    for (const via of vias(cells)) {
      add(via.lane, bridge(via, placement));
    }
    return parts;
  }

  // src/webview/main.ts
  var SVG = "http://www.w3.org/2000/svg";
  var FAMILIES = 12;
  var CH = 7.2;
  var NAME_MIN = 8;
  var NAME_MAX = 24;
  var COUNTS_CH = 9;
  var WHERE_CH = 8;
  var EXTRA_CH = 19;
  var LABEL_GAPS = 3 * 8;
  var AXIS_CH = 6.6;
  var AXIS_EXTRA = 22;
  var TICK_WAIT_MS = 350;
  var SHOW_TEXT = {
    local: "Local first",
    recent: "Recent activity",
    mine: "Mine"
  };
  var vscode = acquireVsCodeApi();
  function send(message) {
    vscode.postMessage(message);
  }
  var state = {
    base: "",
    view: { show: "local", branches: 12, commits: 50, pins: [], hides: [] },
    step: 50,
    meta: void 0,
    states: /* @__PURE__ */ new Map(),
    graph: void 0,
    isRunning: false,
    progress: "",
    error: void 0,
    selected: void 0,
    unfolded: /* @__PURE__ */ new Set(),
    graphWidth: 0,
    nameChars: NAME_MIN,
    axisWidth: 0,
    isListOpen: false,
    listQuery: ""
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
  function laneClasses(lane) {
    const classes = [lane.kind === "remote" ? "remote" : "local"];
    if (lane.protected || lane.family === void 0) {
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
  function selectionKey(selection) {
    return selection.kind === "commit" ? `commit:${selection.oid}` : `lane:${selection.name}`;
  }
  function plural(count, word) {
    return `${String(count)} ${word}${count === 1 ? "" : "s"}`;
  }
  function laneSummary(lane, graph) {
    const base = graph.base.name;
    const what = lane.kind === "reference" ? "the base, the line every other lane is placed against." : `${plural(lane.ahead, "commit")} not in ${base}, ${String(lane.behind)} of ${base} not in it.`;
    const parts = [`${lane.name}: ${what}`];
    if (lane.remote !== void 0) {
      parts.push(`${lane.remote} is at the same commit.`);
    }
    if (lane.merged) {
      parts.push("Merged: nothing ahead of the base.");
    }
    if (lane.only === "remote") {
      parts.push("Only on the remote.");
    }
    const like = graph.show.like?.name;
    if (like !== void 0 && lane.score !== void 0) {
      parts.push(`${percent(lane.score)} of its work is in ${like}.`);
    }
    if (lane.before !== void 0) {
      parts.push(`It goes on below the page: ${plural(lane.before, "older commit")} of its own.`);
    }
    return parts.join(" ");
  }
  function commitSummary(commit) {
    return [
      commit.subject,
      commit.body === "" ? "" : `
${commit.body}
`,
      `${short(commit.oid)} \xB7 ${commit.author} \xB7 ${formatTime(commit.committed)}`,
      commit.parents.length > 1 ? `merge of ${String(commit.parents.length)} parents` : "",
      commit.refs.length > 0 ? commit.refs.join(", ") : ""
    ].filter((line) => line !== "").join("\n");
  }
  function foldedIndices(graph) {
    const folded = /* @__PURE__ */ new Set();
    for (const [index, fold] of graph.folds.entries()) {
      const head = graph.commits[fold.commits[0] ?? -1];
      if (head !== void 0 && !state.unfolded.has(head.oid)) {
        folded.add(index);
      }
    }
    return folded;
  }
  function foldStarts(graph) {
    return new Map(graph.folds.map((fold, index) => [fold.rows[0], index]));
  }
  function commitNode(graph, cell, placed, fold) {
    const commit = cell.commit === void 0 ? void 0 : graph.commits[cell.commit];
    const lane = graph.lanes[cell.lane];
    if (commit === void 0 || lane === void 0) {
      return void 0;
    }
    const { x, y } = centre(cell.row, cell.col, placed);
    const node = svgElement("g", {
      class: ["commit", ...laneClasses(lane)].join(" "),
      tabindex: 0,
      role: "button",
      "aria-label": `Commit ${short(commit.oid)} on ${lane.name}: ${commit.subject}`
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
        r: NODE_RADIUS
      }),
      svgTitle(commitSummary(commit))
    );
    return node;
  }
  function connectorNode(graph, cell, placed) {
    const lane = graph.lanes[cell.lane];
    if (lane === void 0) {
      return void 0;
    }
    const { x, y } = centre(cell.row, cell.col, placed);
    const node = svgElement("g", { class: ["connector", ...laneClasses(lane)].join(" ") });
    node.dataset["lane"] = String(cell.lane);
    node.append(
      svgElement("circle", { class: "pad", cx: x, cy: y, r: NODE_RADIUS }),
      svgElement("circle", { class: "pad-hole", cx: x, cy: y, r: 1.5 }),
      svgTitle(
        lane.before === void 0 ? `${lane.name} goes on in older history.` : `${lane.name} goes on in older history: ${plural(lane.before, "commit")} of its own.`
      )
    );
    return node;
  }
  function drawGraph(graph, placed, width) {
    const svg = svgElement("svg", {
      class: "map-svg",
      width,
      height: placed.height,
      role: "group",
      "aria-label": `Branches around ${graph.base.name}`
    });
    const traces = lanePaths(graph.grid.cells, placed);
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
    const folded = foldedIndices(graph);
    const starts = foldStarts(graph);
    const nodes = svgElement("g", { class: "nodes" });
    for (const cell of graph.grid.cells) {
      if (placed.h(cell.row) === 0) {
        continue;
      }
      let node;
      if (cell.node === "commit") {
        const index = starts.get(cell.row);
        const fold = index === void 0 || graph.folds.at(index)?.lane !== cell.lane ? void 0 : { index, isFolded: folded.has(index) };
        node = commitNode(graph, cell, placed, fold);
      } else if (cell.node === "connector") {
        node = connectorNode(graph, cell, placed);
      }
      if (node !== void 0) {
        nodes.append(node);
      }
    }
    svg.append(nodes);
    return svg;
  }
  function where(lane) {
    if (lane.remote !== void 0) {
      const remote = lane.remote.split("/", 1)[0] ?? lane.remote;
      return {
        text: `= ${remote}`,
        help: `${lane.remote} is at the same commit: one lane for both.`
      };
    }
    if (lane.kind === "reference") {
      return { text: "base", help: "The base: every other lane is placed against it." };
    }
    if (lane.kind === "remote") {
      const help2 = lane.only === "remote" ? "Only on the remote: there is no local branch." : "The remote branch: its commits the local one doesn't have.";
      return { text: "remote", help: help2 };
    }
    const help = lane.only === "local" ? "Only local: not on any remote." : "The local branch.";
    return { text: "local", help };
  }
  function laneLabel(graph, lane, index) {
    const label = element("div", ["lane-label", ...laneClasses(lane)].join(" "));
    label.dataset["lane"] = String(index);
    label.tabIndex = 0;
    label.setAttribute("role", "button");
    label.title = laneSummary(lane, graph);
    const name = element("span", "lane-name", lane.name);
    const counts = element(
      "span",
      "lane-counts",
      lane.kind === "reference" ? "" : `\u2191${String(lane.ahead)} \u2193${String(lane.behind)}`
    );
    const place = where(lane);
    const kind = element("span", "lane-where", place.text);
    kind.title = place.help;
    const extra = element("span", "lane-extra");
    const facts = [];
    if (lane.merged) {
      facts.push("merged");
    }
    if (lane.score !== void 0 && graph.show.like !== void 0) {
      facts.push(percent(lane.score));
    }
    if (lane.before !== void 0) {
      facts.push(`${String(lane.before)} older`);
    }
    if (facts.length > 0) {
      extra.append(element("span", "lane-facts", facts.join(" \xB7 ")));
    }
    const branch = state.states.get(lane.name);
    if (branch !== void 0) {
      const isDrift = lane.remote === void 0 && !branch.isRemote && branch.upstream !== void 0;
      extra.append(...isDrift ? [syncBadge(lane.name, branch)] : [], ...worktreeBadges(branch));
    }
    label.append(name, counts, kind, extra);
    return label;
  }
  function refChips(graph, commit) {
    const drawn = new Set(graph.lanes.flatMap((lane) => [lane.name, lane.remote ?? lane.name]));
    return commit.refs.filter((name) => !drawn.has(name)).map((name) => {
      const chip = element("span", "ref-chip", name);
      chip.title = name;
      return chip;
    });
  }
  function authors(commits) {
    const names = [...new Set(commits.map((commit) => commit.author))];
    return names.length <= 1 ? names[0] ?? "" : `${names[0] ?? ""} +${String(names.length - 1)}`;
  }
  function foldButton(count, index, isFolded) {
    const button = element("button", "fold-toggle", `\xD7${String(count)} ${isFolded ? "\u25B8" : "\u25BE"}`);
    button.type = "button";
    button.dataset["fold"] = String(index);
    button.title = isFolded ? `${plural(count, "commit")} on one lane, folded. Click to show them.` : "Click to fold these commits into one row.";
    return button;
  }
  function tableRow(graph, placed, row, commitAt, day) {
    const node = element("div", day === void 0 ? "hrow" : "hrow day-start");
    node.style.top = `${String(placed.y(row))}px`;
    node.style.height = `${String(placed.h(row))}px`;
    const refs = element("span", "refs");
    const labelled = placed.labels.get(row) ?? [];
    for (const index of labelled) {
      const lane = graph.lanes.at(index);
      if (lane !== void 0) {
        refs.append(laneLabel(graph, lane, index));
      }
    }
    const axis = element("span", "axis");
    const hash = element("span", "hash");
    const subject = element("span", "subject");
    const author = element("span", "author");
    const commit = commitAt.get(row);
    if (commit !== void 0) {
      node.dataset["oid"] = commit.oid;
      node.title = commitSummary(commit);
      const index = foldStarts(graph).get(row);
      const fold = index === void 0 ? void 0 : graph.folds.at(index);
      const runs = fold?.commits.map((at) => graph.commits.at(at)).filter((item) => item !== void 0) ?? [];
      const isFolded = index !== void 0 && foldedIndices(graph).has(index);
      if (index !== void 0 && runs.length > 0) {
        subject.append(foldButton(runs.length, index, isFolded));
      }
      subject.append(...refChips(graph, commit), element("span", "subject-text", commit.subject));
      author.textContent = isFolded ? authors(runs) : commit.author;
      hash.textContent = short(commit.oid);
      axisTick(axis, commit, day, isFolded ? runs.at(-1) : void 0);
    }
    node.append(axis, element("span", "graph-cell"), refs, hash, subject, author);
    return node;
  }
  function axisTick(axis, commit, day, oldest) {
    axis.classList.add(day === void 0 ? "tick" : "day");
    if (day !== void 0) {
      axis.append(element("span", "axis-day", day));
    }
    axis.append(element("span", "axis-time", clock(commit.committed)));
    const time = formatTime(commit.committed);
    axis.title = oldest === void 0 ? time : `${formatTime(oldest.committed)} to ${time}`;
  }
  function dayStarts(placed, commitAt) {
    const year = (/* @__PURE__ */ new Date()).getFullYear();
    const starts = /* @__PURE__ */ new Map();
    let last = "";
    for (const row of placed.rows) {
      const commit = commitAt.get(row);
      if (commit === void 0) {
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
  function drawMap(graph) {
    const placed = layout(graph, foldedIndices(graph));
    const longest = Math.max(0, ...graph.lanes.map((lane) => lane.name.length));
    state.nameChars = Math.max(state.nameChars, Math.min(NAME_MAX, longest));
    state.graphWidth = Math.max(state.graphWidth, cellX(graph.grid.width) + GUTTER);
    const refsWidth = (state.nameChars + COUNTS_CH + WHERE_CH + EXTRA_CH) * CH + LABEL_GAPS;
    const commitAt = new Map(graph.commits.map((commit) => [commit.row, commit]));
    const days = dayStarts(placed, commitAt);
    const dayChars = Math.max(0, ...Array.from(days.values(), (day) => day.length));
    state.axisWidth = Math.max(state.axisWidth, (dayChars + 6) * AXIS_CH + AXIS_EXTRA);
    const map = element("div", "map");
    map.style.setProperty("--axis-width", `${String(Math.round(state.axisWidth))}px`);
    map.style.setProperty("--graph-width", `${String(state.graphWidth)}px`);
    map.style.setProperty("--refs-width", `${String(Math.round(refsWidth))}px`);
    const narrowWidth = (state.nameChars + COUNTS_CH) * CH + LABEL_GAPS / 3 + 16;
    map.style.setProperty("--refs-width-narrow", `${String(Math.round(narrowWidth))}px`);
    map.style.setProperty("--name-width", `${String(Math.round(state.nameChars * CH))}px`);
    map.style.height = `${String(placed.height)}px`;
    map.append(
      drawGraph(graph, placed, state.graphWidth),
      ...placed.rows.map((row) => tableRow(graph, placed, row, commitAt, days.get(row)))
    );
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
  function datasetOf(target) {
    return target instanceof HTMLElement || target instanceof SVGElement ? target.dataset : void 0;
  }
  function selectionAt(target) {
    if (!(target instanceof Element) || state.graph === void 0) {
      return void 0;
    }
    const index = datasetOf(target.closest(".lane-label, .lane"))?.["lane"];
    const lane = index === void 0 ? void 0 : state.graph.lanes[Number(index)];
    if (lane !== void 0) {
      return { kind: "lane", name: lane.name };
    }
    const oid = datasetOf(target.closest("[data-oid]"))?.["oid"];
    return oid === void 0 ? void 0 : { kind: "commit", oid };
  }
  function select(selection, root) {
    state.selected = selectionKey(selection);
    showSelection(root);
    send({ type: "select", selection });
  }
  function didToggleFold(target) {
    const graph = state.graph;
    const index = target instanceof Element ? datasetOf(target.closest("[data-fold]"))?.["fold"] : void 0;
    const fold = index === void 0 ? void 0 : graph?.folds[Number(index)];
    const head = fold === void 0 ? void 0 : graph?.commits[fold.commits[0] ?? -1];
    if (head === void 0) {
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
  function highlight(root, target) {
    const index = target instanceof Element ? datasetOf(target.closest("[data-lane]"))?.["lane"] : void 0;
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
      if (didToggleFold(event.target)) {
        return;
      }
      const selection = selectionAt(event.target);
      if (selection !== void 0) {
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
    group.append(svgElement("path", { class: "trace", d: "M3 7H25" }));
    if (draw === "stack") {
      group.append(
        svgElement("circle", { class: "node stack", cx: 14, cy: 7, r: NODE_RADIUS + 2.5 })
      );
    }
    if (draw !== "line") {
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
    under.append(svgElement("path", { class: "trace", d: "M14 0V4M14 10V14" }));
    const over = svgElement("g", { class: "lane family-0" });
    over.append(svgElement("path", { class: "trace", d: "M2 7H26" }));
    svg.append(under, over);
    return svg;
  }
  function connectorSwatch() {
    const svg = svgElement("svg", { class: "swatch", width: 28, height: 14, "aria-hidden": "true" });
    const group = svgElement("g", { class: "connector protected" });
    group.append(
      svgElement("path", { class: "trace", d: "M14 0V7" }),
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
  function legend(graph) {
    const node = element("div", "map-legend");
    const isUsed = (hasIt) => graph.lanes.some((lane) => hasIt(lane));
    const protectedNames = graph.lanes.filter((lane) => lane.protected || lane.family === void 0);
    if (protectedNames.length > 0) {
      node.append(
        legendItem(
          swatch(["protected"], "line"),
          `protected: ${protectedNames.map((lane) => lane.name).join(", ")}`
        )
      );
    }
    node.append(legendItem(swatch(["family-4"], "line"), "local"));
    if (isUsed((lane) => lane.kind === "remote")) {
      node.append(legendItem(swatch(["family-4", "remote"], "line"), "remote"));
    }
    if (isUsed((lane) => lane.remote !== void 0)) {
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
      legendItem(swatch(["family-4"], "ring"), "commit")
    );
    if (graph.folds.length > 0) {
      node.append(legendItem(swatch(["family-4"], "stack"), "folded run: click \xD7N"));
    }
    if (vias(graph.grid.cells).length > 0) {
      node.append(legendItem(viaSwatch(), "passes under, not joined"));
    }
    if (graph.grid.cells.some((cell) => cell.node === "connector")) {
      node.append(legendItem(connectorSwatch(), "goes on in older history"));
    }
    return node;
  }
  function ask(view) {
    state.view = view;
    send({ type: "view", ...view });
  }
  function showSelect() {
    const label = element("label", "control");
    label.title = "Which branches to show besides the base.";
    const select2 = element("select", "control-select");
    const option = (value, text) => {
      const item = element("option", "", text);
      item.value = value;
      item.selected = value === state.view.show;
      select2.append(item);
    };
    for (const [value, text] of Object.entries(SHOW_TEXT)) {
      option(value, text);
    }
    if (state.view.like !== void 0) {
      option("like", `Like ${state.view.like}`);
    }
    option("pick", "Like a branch\u2026");
    select2.addEventListener("change", () => {
      const value = select2.value;
      if (value === "pick") {
        select2.value = state.view.show;
        send({ type: "choose", control: "like" });
      } else if (value !== "like" && isShowMode(value)) {
        const { branches, commits, pins, hides } = state.view;
        ask({ show: value, branches, commits, pins, hides });
      } else if (state.view.like !== void 0) {
        ask({ ...state.view, show: "like" });
      }
    });
    label.append(element("span", "control-label", "Show"), select2);
    return label;
  }
  var list = element("div", "branch-list");
  list.setAttribute("role", "dialog");
  list.setAttribute("aria-label", "Branches to show");
  var ticking = { timer: void 0 };
  function isTicked(view, name, rank) {
    return !view.hides.includes(name) && (rank < view.branches || view.pins.includes(name));
  }
  function ticked(view, name, rank, isOn) {
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
  function askSoon(view) {
    state.view = view;
    clearTimeout(ticking.timer);
    ticking.timer = setTimeout(() => {
      ticking.timer = void 0;
      ask(state.view);
    }, TICK_WAIT_MS);
    drawList();
  }
  function shownCount(graph) {
    return graph.branches.filter((name, rank) => isTicked(state.view, name, rank)).length;
  }
  function listItem(graph, name, rank) {
    const item = element("label", "branch-item");
    const box = element("input", "");
    box.type = "checkbox";
    box.checked = isTicked(state.view, name, rank);
    box.dataset["branch"] = name;
    box.addEventListener("change", () => {
      askSoon(ticked(state.view, name, rank, box.checked));
    });
    const lanes = graph.lanes.filter((lane2) => lane2.branch === name);
    const lane = lanes.find((each) => each.kind !== "remote") ?? lanes[0];
    let mark = element("span", "swatch-blank");
    item.title = name;
    if (lane !== void 0) {
      mark = swatch(laneClasses(lane), lane.kind === "remote" ? "line" : "ring");
    } else if (box.checked) {
      mark = element("span", "swatch-blank", "ref");
      item.title = `${name}: no commits of its own here; shown as a ref on another lane.`;
    }
    item.append(box, mark, element("span", "branch-name", name));
    return item;
  }
  function listButton(text, help, next) {
    const button = element("button", "list-button", text);
    button.type = "button";
    button.title = help;
    button.addEventListener("click", () => {
      askSoon(next());
    });
    return button;
  }
  function focusedInList() {
    const active = document.activeElement;
    if (!(active instanceof HTMLInputElement) || !list.contains(active)) {
      return void 0;
    }
    return active.type === "search" ? "search" : active.dataset["branch"];
  }
  function tickedAll(view, branches, isOn) {
    let next = view;
    for (const { name, rank } of branches) {
      next = ticked(next, name, rank, isOn);
    }
    return next;
  }
  function drawList() {
    const graph = state.graph;
    if (graph === void 0 || !state.isListOpen) {
      list.remove();
      return;
    }
    const scroller = list.querySelector(".branch-items");
    const scroll = scroller?.scrollTop ?? 0;
    const focused = focusedInList();
    const query = state.listQuery.trim().toLowerCase();
    const matches = graph.branches.map((name, rank) => ({ name, rank })).filter(({ name }) => name.toLowerCase().includes(query));
    const search = element("input", "branch-search");
    search.type = "search";
    search.placeholder = `Filter ${String(graph.branches.length)} branches`;
    search.value = state.listQuery;
    search.addEventListener("input", () => {
      state.listQuery = search.value;
      drawList();
    });
    const first = state.view.branches;
    const buttons = element("div", "list-buttons");
    buttons.append(
      listButton(
        "Reset",
        `Back to the first ${String(first)} by Show, nothing ticked by hand.`,
        () => ({ ...state.view, pins: [], hides: [] })
      ),
      listButton(
        "All",
        "Tick every branch in the list (as filtered).",
        () => tickedAll(state.view, matches, true)
      ),
      listButton(
        "None",
        "Untick every branch in the list (as filtered).",
        () => tickedAll(state.view, matches, false)
      )
    );
    const items = element("div", "branch-items");
    for (const { name, rank } of matches) {
      if (rank === first && query === "") {
        items.append(element("div", "branch-divider", `past the first ${String(first)} by Show`));
      }
      items.append(listItem(graph, name, rank));
    }
    if (matches.length === 0) {
      items.append(element("div", "branch-divider", "No branch matches."));
    }
    list.replaceChildren(search, buttons, items);
    if (!list.isConnected) {
      document.body.append(list);
    }
    const anchor = document.querySelector("#branches-button")?.getBoundingClientRect();
    if (anchor !== void 0) {
      list.style.top = `${String(Math.round(anchor.bottom + 4))}px`;
      list.style.left = `${String(Math.round(anchor.left))}px`;
    }
    items.scrollTop = scroll;
    const again = focused === "search" ? search : [...items.querySelectorAll("input")].find((box) => box.dataset["branch"] === focused);
    again?.focus({ preventScroll: true });
  }
  function openList(isOpen) {
    state.isListOpen = isOpen;
    drawList();
    if (!isOpen) {
      return;
    }
    list.querySelector(".branch-search")?.focus({ preventScroll: true });
  }
  list.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") {
      return;
    }
    openList(false);
    document.querySelector("#branches-button")?.focus();
  });
  addEventListener("pointerdown", (event) => {
    const target = event.target;
    if (target instanceof Node && state.isListOpen && !list.contains(target) && document.querySelector("#branches-button")?.contains(target) !== true) {
      openList(false);
    }
  });
  function branchesButton(graph) {
    const value = graph === void 0 ? "\u2026" : `${String(shownCount(graph))} of ${String(graph.branches.length)}`;
    const button = controlButton(
      "Branches",
      value,
      "Which branches to show besides the base: tick or untick them.",
      () => {
        openList(!state.isListOpen);
      }
    );
    button.id = "branches-button";
    button.setAttribute("aria-expanded", String(state.isListOpen));
    return button;
  }
  function header() {
    const graph = state.graph;
    const node = element("header", "header stacked");
    const heading = element("div", "heading");
    heading.append(element("span", "title", `History around ${state.base}`));
    if (graph !== void 0) {
      const shown = `${String(shownCount(graph))} of ${String(graph.branches.length)} branches`;
      heading.append(
        element("span", "count", `${shown} \xB7 ${plural(graph.commits.length, "commit")}`)
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
      showSelect(),
      branchesButton(graph)
    );
    node.append(heading, toolbar);
    return node;
  }
  function olderButton(graph) {
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
  function render() {
    const root = document.querySelector("#root");
    if (root === null) {
      return;
    }
    const scroll = scrollY;
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
      parts.push(map);
      if (graph.page.more) {
        parts.push(olderButton(graph));
      }
      parts.push(legend(graph));
    }
    parts.push(footer(state.meta));
    root.replaceChildren(...parts);
    scrollTo(0, scroll);
    drawList();
  }
  function receive(message) {
    switch (message.type) {
      case "setup": {
        state.base = message.base;
        state.view = message.view;
        state.step = message.step;
        state.meta = message.meta;
        state.states = new Map(message.states.map((branch) => [branch.name, branch]));
        state.graph = void 0;
        state.isRunning = false;
        state.error = void 0;
        state.selected = void 0;
        state.unfolded = /* @__PURE__ */ new Set();
        state.graphWidth = 0;
        state.nameChars = NAME_MIN;
        state.axisWidth = 0;
        state.isListOpen = false;
        state.listQuery = "";
        break;
      }
      case "running": {
        state.isRunning = true;
        state.progress = message.progress ?? "";
        state.view = ticking.timer === void 0 ? message.view ?? state.view : state.view;
        break;
      }
      case "graph": {
        const graph = parseGraph(message.graph);
        state.isRunning = false;
        state.progress = "";
        state.graph = graph;
        state.error = graph === void 0 ? "This isn't a sundgren.graph/3 document." : void 0;
        if (graph !== void 0 && ticking.timer === void 0) {
          const listed = new Set(graph.branches);
          state.view = {
            ...state.view,
            pins: state.view.pins.filter((name) => listed.has(name)),
            hides: state.view.hides.filter((name) => listed.has(name))
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
    if (message.type === "graph" && state.graph !== void 0) {
      send({ type: "rendered", oid: state.graph.base.oid, rows: state.graph.lanes.length });
    }
  }
  addEventListener("message", (event) => {
    if (event.origin !== origin) {
      return;
    }
    receive(event.data);
  });
  send({ type: "ready" });
})();
//# sourceMappingURL=webview.js.map
