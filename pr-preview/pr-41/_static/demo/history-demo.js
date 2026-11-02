"use strict";
(() => {
  // src/graph.ts
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
    const { commits, more, newer } = value;
    const since = new Optional(value["since"], isWhole);
    const until = new Optional(value["until"], isWhole);
    return typeof more === "boolean" && typeof newer === "boolean" && isCount(commits) && commits >= 1 && since.isValid && until.isValid ? { commits, more, newer, ...field("since", since.value), ...field("until", until.value) } : void 0;
  }
  function isWhole(value) {
    return isNumber(value) && Number.isSafeInteger(value);
  }
  function parseBranch(value) {
    if (!isObject(value)) {
      return void 0;
    }
    const { name, local, authors } = value;
    return typeof local === "boolean" && isString(name) && isArrayOf(authors, isString) ? { name, local, authors } : void 0;
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
    const { oid, parents, author, email, committed, subject, body, refs, row, lanes } = value;
    return isOid(oid) && isArrayOf(parents, isOid) && isString(author) && isString(email) && isNumber(committed) && Number.isSafeInteger(committed) && isString(subject) && isString(body) && isArrayOf(refs, isString) && isCount(row) && isArrayOf(lanes, isCount) ? { oid, parents, author, email, committed, subject, body, refs, row, lanes } : void 0;
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
    const { repository, threshold } = value;
    const branches = parseAll(value["branches"], parseBranch);
    const base = parseNamed(value["base"]);
    const show = parseShow(value["show"]);
    const page = parsePage(value["page"]);
    const lanes = parseAll(value["lanes"], parseLane);
    const commits = parseAll(value["commits"], parseCommit);
    const folds = parseAll(value["folds"], parseFold);
    const grid = parseGrid(value["grid"]);
    if (branches === void 0 || base === void 0 || show === void 0 || page === void 0 || lanes === void 0 || commits === void 0 || folds === void 0 || grid === void 0 || !isString(repository) || !isNumber(threshold)) {
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

  // src/messages.ts
  var MAX_BRANCHES = 30;
  var CONTROLS = /* @__PURE__ */ new Set(["base", "target", "comments", "like"]);
  function isObject2(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function isLine(value) {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 1;
  }
  function isOid2(value) {
    return typeof value === "string" && /^[\da-f]{4,64}$/u.test(value);
  }
  function isPath(value) {
    return typeof value === "string" && value !== "" && !value.includes("\0");
  }
  function isRevision(value) {
    return isObject2(value) && isOid2(value["oid"]) && isPath(value["path"]);
  }
  function isCount2(value) {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
  }
  function isBranchName(value) {
    return typeof value === "string" && value !== "" && !value.startsWith("-") && !value.includes("\0");
  }
  function isTimeOrMissing(value) {
    return value === void 0 || typeof value === "number" && Number.isSafeInteger(value);
  }
  function isBranchNames(value) {
    return Array.isArray(value) && value.every((name) => isBranchName(name));
  }
  function parseView(value) {
    const { show, like, branches, commits, pins, hides, since, until } = value;
    if (!isShowMode(show) || !isCount2(branches) || branches > MAX_BRANCHES || !isCount2(commits) || commits < 1 || !isBranchNames(pins) || !isBranchNames(hides) || !isTimeOrMissing(since) || !isTimeOrMissing(until)) {
      return void 0;
    }
    const view = {
      type: "view",
      show,
      branches,
      commits,
      pins,
      hides,
      ...since !== void 0 && { since },
      ...until !== void 0 && { until }
    };
    if (like === void 0) {
      return show === "like" ? void 0 : view;
    }
    return isBranchName(like) ? { ...view, like } : void 0;
  }
  function parseSelection(value) {
    if (!isObject2(value)) {
      return void 0;
    }
    const { kind, oid, name } = value;
    if (kind === "commit") {
      return isOid2(oid) ? { kind, oid } : void 0;
    }
    return kind === "lane" && typeof name === "string" && name !== "" ? { kind, name } : void 0;
  }
  function parseInspect(value) {
    const { branch, path, line } = value;
    if (typeof branch !== "string" || branch === "") {
      return void 0;
    }
    if (path !== void 0 && !isPath(path)) {
      return void 0;
    }
    if (line !== void 0 && !isLine(line)) {
      return void 0;
    }
    return {
      type: "inspect",
      branch,
      ...path !== void 0 && { path },
      ...line !== void 0 && { line }
    };
  }
  function parseOpen(value) {
    const { revision, line } = value;
    if (!isRevision(revision)) {
      return void 0;
    }
    if (line === void 0) {
      return { type: "open", revision };
    }
    return isLine(line) ? { type: "open", revision, line } : void 0;
  }
  function parseHistoryMessage(value) {
    switch (value["type"]) {
      case "history": {
        const { branch } = value;
        if (branch === void 0) {
          return { type: "history" };
        }
        return typeof branch === "string" && branch !== "" ? { type: "history", branch } : void 0;
      }
      case "view": {
        return parseView(value);
      }
      case "select": {
        const selection = parseSelection(value["selection"]);
        return selection === void 0 ? void 0 : { type: "select", selection };
      }
      default: {
        return void 0;
      }
    }
  }
  function parseFromPage(value) {
    if (!isObject2(value)) {
      return void 0;
    }
    switch (value["type"]) {
      case "ready": {
        return { type: "ready" };
      }
      case "rendered": {
        const { oid, rows } = value;
        return typeof oid === "string" && typeof rows === "number" ? { type: "rendered", oid, rows } : void 0;
      }
      case "reveal": {
        const { line } = value;
        return isLine(line) ? { type: "reveal", line } : void 0;
      }
      case "choose": {
        const { control } = value;
        return typeof control === "string" && CONTROLS.has(control) ? { type: "choose", control } : void 0;
      }
      case "inspect": {
        return parseInspect(value);
      }
      case "open": {
        return parseOpen(value);
      }
      default: {
        return parseHistoryMessage(value);
      }
    }
  }

  // src/docs/host.ts
  function isObject3(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function isStrings(value) {
    return Array.isArray(value) && value.every((item) => typeof item === "string");
  }
  function isNumbers(value) {
    return Array.isArray(value) && value.every((item) => typeof item === "number");
  }
  function dataNamed(name) {
    const value = Reflect.get(globalThis, name);
    return value;
  }
  function parseDemoGraph(value) {
    if (!isObject3(value)) {
      return void 0;
    }
    const { show, commits, hides, dates, graph } = value;
    if (typeof commits !== "number" || !isStrings(show) || !isStrings(hides)) {
      return void 0;
    }
    if (dates === void 0) {
      return { show, commits, hides, graph };
    }
    const [since, until] = isNumbers(dates) ? dates : [];
    return since === void 0 || until === void 0 ? void 0 : { show, commits, hides, dates: [since, until], graph };
  }
  function demoGraphs() {
    const value = dataNamed("sundgrenDemoGraphs");
    if (!isObject3(value)) {
      return void 0;
    }
    const { page, days, graphs } = value;
    if (typeof page !== "number" || !isNumbers(days) || !Array.isArray(graphs)) {
      return void 0;
    }
    const parsed = graphs.map((item) => parseDemoGraph(item));
    const made = parsed.filter((item) => item !== void 0);
    return made.length === parsed.length ? { page, days, graphs: made } : void 0;
  }
  function standIn(onMessage) {
    Object.defineProperty(globalThis, "acquireVsCodeApi", {
      value: () => ({ postMessage: onMessage }),
      configurable: true
    });
  }
  function post(message) {
    postMessage(message, origin);
  }
  function parentRoot() {
    try {
      return frameElement === null ? void 0 : parent.document.documentElement;
    } catch {
      return void 0;
    }
  }
  function syncTheme() {
    const root = parentRoot();
    const isDark = root === void 0 ? matchMedia("(prefers-color-scheme: dark)").matches : root.classList.contains("dark");
    document.body.classList.toggle("vscode-dark", isDark);
    document.body.classList.toggle("vscode-light", !isDark);
  }
  function followTheme() {
    syncTheme();
    const root = parentRoot();
    if (root !== void 0) {
      new MutationObserver(syncTheme).observe(root, {
        attributes: true,
        attributeFilter: ["class"]
      });
    }
  }
  function picker(title, options, current, onPick, note2) {
    const backdrop = document.createElement("div");
    backdrop.className = "sg-picker-backdrop";
    const box = document.createElement("div");
    box.className = "sg-picker";
    box.setAttribute("role", "dialog");
    const heading = document.createElement("p");
    heading.className = "sg-picker-title";
    heading.textContent = title;
    box.append(heading);
    const close = () => {
      backdrop.remove();
    };
    for (const option of options) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = option === current ? "sg-picker-option current" : "sg-picker-option";
      button.textContent = option;
      button.addEventListener("click", () => {
        close();
        onPick(option);
      });
      box.append(button);
    }
    if (note2 !== void 0) {
      const foot = document.createElement("p");
      foot.className = "sg-picker-note";
      foot.textContent = note2;
      box.append(foot);
    }
    backdrop.addEventListener("click", (event) => {
      if (event.target === backdrop) {
        close();
      }
    });
    backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        close();
      }
    });
    backdrop.append(box);
    document.body.append(backdrop);
    box.querySelector("button")?.focus();
  }

  // src/docs/history-views.ts
  var DAY = 86400;
  function modeOf(view) {
    return view.show[0] === "--like" ? "like" : view.show[1] ?? "";
  }
  function alike(a, b) {
    const all = /* @__PURE__ */ new Set([...a, ...b]);
    return all.size === 0 ? 1 : a.filter((name) => b.includes(name)).length / all.size;
  }
  function daysKey(days, since, until) {
    if (since === void 0 && until === void 0) {
      return "any";
    }
    const from = since ?? 0;
    const to = until ?? Number.MAX_SAFE_INTEGER;
    const covered = days.filter((day) => day <= to && day + DAY - 1 >= from);
    if (covered.length > 0) {
      return covered.join(",");
    }
    return to < (days[0] ?? 0) ? "before" : "after";
  }
  function closest(graphs, days, view) {
    const wanted = daysKey(days, view.since, view.until);
    const keyOf = (made) => made.dates === void 0 ? "any" : daysKey(days, made.dates[0], made.dates[1]);
    const same = graphs.filter((made) => modeOf(made) === view.show);
    const pool = same.length > 0 ? same : graphs;
    const exact = same.find(
      (made) => keyOf(made) === wanted && made.hides.length === view.hides.length && alike(made.hides, view.hides) === 1 && (view.since !== void 0 || made.commits === view.commits)
    );
    if (exact !== void 0) {
      return { view: exact, isExact: true };
    }
    const score = (made) => 2 * (keyOf(made) === wanted ? 1 : 0) + alike(made.hides, view.hides);
    const best = Math.max(...pool.map((made) => score(made)));
    const nearest = pool.filter((made) => score(made) === best).toSorted((a, b) => Math.abs(a.commits - view.commits) - Math.abs(b.commits - view.commits));
    return { view: nearest[0], isExact: false };
  }

  // src/docs/history-demo.ts
  var BASE = "main";
  var RUN_MS = 150;
  function send(message) {
    post(message);
  }
  function note(isExact) {
    const shown = document.querySelector(".sg-demo-note");
    if (isExact) {
      shown?.remove();
      return;
    }
    if (shown !== null) {
      return;
    }
    const node = document.createElement("div");
    node.className = "sg-demo-note";
    node.textContent = "The demo has views made in advance, and none for exactly this: it shows the closest. In VS Code, sundgren draws what you ask for.";
    document.body.append(node);
  }
  function start(demo2) {
    let current = {
      show: "local",
      branches: 12,
      commits: demo2.page,
      pins: [],
      hides: []
    };
    const answer = (view) => {
      const found = closest(demo2.graphs, demo2.days, view);
      const graph = parseGraph(found.view?.graph);
      if (graph === void 0) {
        send({ type: "error", message: "The demo has no view for this." });
        return;
      }
      send({ type: "graph", graph });
      note(found.isExact);
    };
    const run = (view) => {
      current = view;
      send({ type: "running", view });
      setTimeout(() => {
        answer(view);
      }, RUN_MS);
    };
    const likes = [
      ...new Set(
        demo2.graphs.filter((made) => modeOf(made) === "like").map((made) => made.show[1] ?? "")
      )
    ];
    const choose = (control) => {
      if (control === "base") {
        picker(
          `Base: the reference line (now ${BASE})`,
          [BASE],
          BASE,
          () => {
            run(current);
          },
          "The demo has views of one base; in VS Code, any branch."
        );
      } else if (control === "like") {
        picker(
          "Like: show the branches whose work is most found in this one",
          likes,
          current.like,
          (like) => {
            run({ ...current, show: "like", like });
          },
          "The demo ranks by these; in VS Code, any branch."
        );
      }
    };
    standIn((value) => {
      const message = parseFromPage(value);
      switch (message?.type) {
        case "ready": {
          send({ type: "setup", base: BASE, view: current, step: demo2.page, states: [] });
          answer(current);
          break;
        }
        case "view": {
          const { show, like, branches, commits, pins, hides, since, until } = message;
          run({
            show,
            ...like !== void 0 && { like },
            branches,
            commits,
            pins,
            hides,
            ...since !== void 0 && { since },
            ...until !== void 0 && { until }
          });
          break;
        }
        case "choose": {
          choose(message.control);
          break;
        }
        default:
      }
    });
  }
  followTheme();
  var demo = demoGraphs();
  if (demo !== void 0) {
    start(demo);
  }
})();
