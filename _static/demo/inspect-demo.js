"use strict";
(() => {
  // src/graph.ts
  var SHOW_MODES = /* @__PURE__ */ new Set(["local", "recent", "mine", "like"]);
  function isShowMode(value) {
    return SHOW_MODES.has(value);
  }

  // src/messages.ts
  var MAX_BRANCHES = 30;
  var CONTROLS = /* @__PURE__ */ new Set(["base", "target", "comments", "like"]);
  function isObject(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function isLine(value) {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 1;
  }
  function isOid(value) {
    return typeof value === "string" && /^[\da-f]{4,64}$/u.test(value);
  }
  function isPath(value) {
    return typeof value === "string" && value !== "" && !value.includes("\0");
  }
  function isRevision(value) {
    return isObject(value) && isOid(value["oid"]) && isPath(value["path"]);
  }
  function isCount(value) {
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
    if (!isShowMode(show) || !isCount(branches) || branches > MAX_BRANCHES || !isCount(commits) || commits < 1 || !isBranchNames(pins) || !isBranchNames(hides) || !isTimeOrMissing(since) || !isTimeOrMissing(until)) {
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
    if (!isObject(value)) {
      return void 0;
    }
    const { kind, oid, name } = value;
    if (kind === "commit") {
      return isOid(oid) ? { kind, oid } : void 0;
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
    if (!isObject(value)) {
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

  // src/containment.ts
  var RELATIONS = /* @__PURE__ */ new Set(["same", "a-in-b", "b-in-a", "overlap", "unrelated", "empty"]);
  var FOUND = /* @__PURE__ */ new Set(["exact", "fuzzy", "missing", "comment"]);
  function isObject2(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function isCount2(value) {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
  }
  function isRatio(value) {
    return value === void 0 || typeof value === "number" && value >= 0 && value <= 1;
  }
  function isFileShare(value) {
    return isObject2(value) && typeof value["path"] === "string" && isCount2(value["lines"]) && isCount2(value["exact"]) && isCount2(value["fuzzy"]);
  }
  function isOptional(value, isValid) {
    return value === void 0 || isValid(value);
  }
  function isCounterpart(value) {
    return isObject2(value) && typeof value["path"] === "string" && isOptional(value["item"], (item) => typeof item === "string") && isOptional(value["start"], isCount2) && isOptional(value["end"], isCount2) && isCount2(value["lines"]);
  }
  function isItemShare(value) {
    return isObject2(value) && typeof value["path"] === "string" && isCount2(value["lines"]) && isCount2(value["exact"]) && isCount2(value["fuzzy"]) && isOptional(value["item"], (item) => typeof item === "string") && isOptional(value["kind"], (kind) => typeof kind === "string") && isOptional(value["start"], isCount2) && isOptional(value["end"], isCount2) && isOptional(value["counterpart"], isCounterpart);
  }
  function isDirection(value) {
    return isObject2(value) && isCount2(value["lines"]) && isCount2(value["exact"]) && isCount2(value["fuzzy"]) && isCount2(value["in_order"]) && isRatio(value["containment"]) && isRatio(value["exact_containment"]) && isRatio(value["ordered_containment"]) && Array.isArray(value["files"]) && value["files"].every((file) => isFileShare(file)) && isOptional(
      value["items"],
      (items) => Array.isArray(items) && items.every((item) => isItemShare(item))
    ) && isOptional(
      value["lines_found"],
      (lines) => Array.isArray(lines) && lines.every((line) => isLineFound(line))
    );
  }
  function isAt(value) {
    return isObject2(value) && typeof value["path"] === "string" && isCount2(value["line"]);
  }
  function isLineFound(value) {
    return isObject2(value) && typeof value["path"] === "string" && isCount2(value["line"]) && typeof value["text"] === "string" && isOptional(value["item"], (item) => typeof item === "string") && typeof value["found"] === "string" && FOUND.has(value["found"]) && isRatio(value["score"]) && isOptional(value["edits"], isCount2) && isOptional(value["at"], isAt);
  }
  function isSide(value) {
    return isObject2(value) && typeof value["rev"] === "string" && typeof value["oid"] === "string" && typeof value["fork"] === "string" && isCount2(value["commits"]) && isCount2(value["files"]) && isCount2(value["lines"]) && isCount2(value["comment_lines"]) && isCount2(value["files_read"]);
  }
  function parseContainment(value) {
    return isObject2(value) && value["schema"] === "sundgren.containment/0" && typeof value["repository"] === "string" && typeof value["base"] === "string" && typeof value["base_oid"] === "string" && typeof value["threshold"] === "number" && (value["comments"] === "ignored" || value["comments"] === "counted") && isSide(value["a"]) && isSide(value["b"]) && isDirection(value["a_in_b"]) && isDirection(value["b_in_a"]) && typeof value["relation"] === "string" && RELATIONS.has(value["relation"]) ? value : void 0;
  }

  // src/docs/host.ts
  function isObject3(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }
  function dataNamed(name) {
    const value = Reflect.get(globalThis, name);
    return value;
  }
  function parseResults(value) {
    if (!Array.isArray(value)) {
      return void 0;
    }
    const results = [];
    for (const item of value) {
      const branch = isObject3(item) ? item["branch"] : void 0;
      const result = isObject3(item) ? parseContainment(item["result"]) : void 0;
      if (typeof branch !== "string" || result === void 0) {
        return void 0;
      }
      results.push({ branch, result });
    }
    return results;
  }
  function demoContainment() {
    const value = dataNamed("sundgrenDemoContainment");
    if (!isObject3(value)) {
      return void 0;
    }
    const { base, target, targets } = value;
    if (typeof base !== "string" || typeof target !== "string" || !isObject3(targets)) {
      return void 0;
    }
    const byTarget = /* @__PURE__ */ new Map();
    for (const [name, modes] of Object.entries(targets)) {
      const ignored = isObject3(modes) ? parseResults(modes["ignored"]) : void 0;
      const counted = isObject3(modes) ? parseResults(modes["counted"]) : void 0;
      if (ignored === void 0 || counted === void 0) {
        return void 0;
      }
      byTarget.set(name, { ignored, counted });
    }
    return { base, target, names: Object.keys(targets), targets: byTarget };
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

  // src/docs/inspect-demo.ts
  function send(message) {
    post(message);
  }
  function start(demo2) {
    const query = new URLSearchParams(location.search);
    const asked = query.get("target");
    const target = asked !== null && demo2.targets.has(asked) ? asked : demo2.target;
    const comments = query.get("comments") === "counted" ? "counted" : "ignored";
    const modes = demo2.targets.get(target);
    const results = (comments === "counted" ? modes?.counted : modes?.ignored) ?? [];
    const entry = results.find((candidate) => candidate.branch === query.get("branch")) ?? results[0];
    const path = query.get("path");
    const line = Number(query.get("line"));
    const back = document.querySelector(".sg-back");
    if (back !== null) {
      back.href = `containment.html?${new URLSearchParams({ target, comments }).toString()}`;
    }
    standIn((value) => {
      if (entry === void 0 || parseFromPage(value)?.type !== "ready") {
        return;
      }
      const focus = path === null ? void 0 : { path, ...Number.isSafeInteger(line) && line > 0 && { line } };
      send({
        type: "inspect",
        branch: entry.branch,
        target,
        base: demo2.base,
        comments,
        threshold: 0.9,
        result: entry.result,
        ...focus !== void 0 && { focus }
      });
    });
  }
  followTheme();
  var demo = demoContainment();
  if (demo !== void 0) {
    start(demo);
  }
})();
