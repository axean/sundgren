"use strict";
(() => {
  // src/containment.ts
  function median(values) {
    if (values.length === 0) {
      return void 0;
    }
    const sorted = values.toSorted((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    const upper = sorted.at(middle) ?? 0;
    return sorted.length % 2 === 1 ? upper : ((sorted.at(middle - 1) ?? 0) + upper) / 2;
  }
  function compareResults(a, b) {
    const score = (c) => c?.a_in_b.containment ?? -1;
    return score(b) - score(a);
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
  function fileName(path) {
    return path.split("/").at(-1) ?? path;
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
  function movement(path, item, there) {
    const isMoved = there.path !== path;
    const isRenamed = item !== void 0 && there.item !== void 0 && (item.split(/[.:]/u).at(-1) ?? item) !== (there.item.split(/[.:]/u).at(-1) ?? there.item);
    if (isMoved && isRenamed) {
      return "moved, renamed";
    }
    if (isMoved) {
      return "moved";
    }
    return isRenamed ? "renamed" : void 0;
  }

  // src/webview/containment.ts
  var vscode = acquireVsCodeApi();
  function send(message) {
    vscode.postMessage(message);
  }
  var RELATION_LABELS = {
    same: "same work",
    "a-in-b": "in target",
    "b-in-a": "contains target",
    overlap: "overlap",
    unrelated: "unrelated",
    empty: "nothing ahead"
  };
  var state = {
    target: "",
    base: "",
    comments: "ignored",
    threshold: 0.9,
    meta: void 0,
    states: /* @__PURE__ */ new Map(),
    entries: /* @__PURE__ */ new Map(),
    expanded: /* @__PURE__ */ new Set(),
    isDone: false
  };
  function control(name, label, value, help) {
    const button = element("button", "control");
    button.type = "button";
    button.title = help;
    button.append(element("span", "control-label", label), element("span", "control-value", value));
    button.append(element("span", "control-caret", "\u25BE"));
    button.addEventListener("click", () => {
      send({ type: "choose", control: name });
    });
    return button;
  }
  function bar(className, fraction) {
    const node = element("div", className);
    node.style.width = `${String(Math.max(0, Math.min(1, fraction)) * 100)}%`;
    return node;
  }
  function bars(result) {
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
  function fork(oid, branch) {
    const node = element("span", "fork", short(oid));
    node.title = `${branch} left ${state.base} at ${oid}`;
    return node;
  }
  function link(className, text, open) {
    const node = element("span", `${className} link`, text);
    node.addEventListener("click", () => {
      send(open());
    });
    return node;
  }
  function inspect(branch, path, line) {
    return {
      type: "inspect",
      branch,
      ...path !== void 0 && { path },
      ...line !== void 0 && { line }
    };
  }
  function itemRow(branch, item) {
    const row = element("tr", "item");
    const name = element("td", "path");
    const open = () => inspect(branch, item.path, item.start);
    const label = link("item-name", item.item ?? "outside items", open);
    label.title = `${item.kind ?? "lines outside items"}: inspect its lines`;
    name.append(label);
    const there = item.counterpart;
    if (there !== void 0) {
      const elsewhere = there.path === item.path ? "" : `${fileName(there.path)} `;
      const moved = movement(item.path, item.item, there);
      const isSame = moved === void 0 && there.item === item.item;
      const target = link(
        isSame ? "counterpart same" : "counterpart",
        `\u2192 ${elsewhere}${there.item ?? "outside items"}`,
        open
      );
      const where = there.item === void 0 ? there.path : `${there.path}, ${there.item}`;
      target.title = `${String(there.lines)} of its found lines are in ${where} in ${state.target}. Click to inspect them.`;
      name.append(target);
      if (moved !== void 0) {
        name.append(element("span", "movement", moved));
      }
    }
    row.append(
      element("td", "number", String(item.exact)),
      element("td", "number", String(item.fuzzy)),
      element("td", "number", String(item.lines)),
      name
    );
    return row;
  }
  function filesTable(branch, result) {
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
          state.comments === "ignored" ? `${String(result.a.comment_lines)} comment lines left out.` : `${String(result.a.comment_lines)} of its lines are comments, counted.`
        )
      );
    }
    container.append(actions);
    const table = element("table", "files");
    const head = element("tr", "");
    for (const title of ["exact", "fuzzy", "lines", "file, and its items \u2192 where in the target"]) {
      head.append(element("th", "", title));
    }
    table.append(head);
    const files = ahead.files.toSorted(
      (a, b) => b.exact + b.fuzzy - (a.exact + a.fuzzy) || b.lines - a.lines
    );
    const items = ahead.items ?? [];
    for (const file of files) {
      const row = element("tr", "file");
      row.append(
        element("td", "number", String(file.exact)),
        element("td", "number", String(file.fuzzy)),
        element("td", "number", String(file.lines))
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
  function nameCell(branch) {
    const node = element("span", "name-cell");
    node.append(element("span", "name", branch));
    const where = state.states.get(branch);
    if (where !== void 0) {
      const badges = element("span", "badges");
      badges.append(syncBadge(branch, where), ...worktreeBadges(where));
      node.append(badges);
    }
    return node;
  }
  function branchCount(total) {
    let remote = 0;
    for (const branch of state.states.values()) {
      if (branch.isRemote) {
        remote += 1;
      }
    }
    return state.states.size === 0 ? `${String(total)} branches` : `${String(total)} branches: ${String(total - remote)} local, ${String(remote)} remote only`;
  }
  function lines(count, from, branch) {
    const node = element("span", "lines", `${String(count)} lines since `);
    node.append(fork(from, branch));
    return node;
  }
  function branchRow(branch, entry) {
    const block = element("section", "branch");
    const row = element("div", "row");
    row.dataset["branch"] = branch;
    const { result, error } = entry;
    if (result === void 0) {
      row.append(
        element(
          "span",
          error === void 0 ? "relation pending" : "relation failed",
          error ? "failed" : "\u2026"
        ),
        nameCell(branch),
        element("span", "lines", ""),
        element("span", "message", error ?? "comparing")
      );
      block.append(row);
      return block;
    }
    row.classList.add("clickable");
    row.title = `${String(result.a_in_b.exact)} exact + ${String(result.a_in_b.fuzzy)} fuzzy of ${String(result.a.lines)} lines are in ${state.target}; ${percent(result.b_in_a.containment)} of ${state.target}'s ${String(result.b.lines)} lines are in ${branch}. In order: ${percent(result.a_in_b.ordered_containment)}.`;
    row.append(
      element("span", `relation ${result.relation}`, RELATION_LABELS[result.relation]),
      nameCell(branch),
      lines(result.a.lines, result.a.fork, branch),
      bars(result),
      element("span", "percent", percent(result.a_in_b.containment)),
      element("span", "percent reverse-percent", percent(result.b_in_a.containment))
    );
    block.append(row);
    if (state.expanded.has(branch)) {
      block.classList.add("open");
      block.append(filesTable(branch, result));
    }
    return block;
  }
  function medianNote(middle, count) {
    if (middle === void 0) {
      return element("span", "median-note", "");
    }
    const node = element("span", "median-note");
    node.append(
      element("span", "median-mark", "\u2506"),
      ` median ${percent(middle)} of ${String(count)} branches`
    );
    node.title = `The dashed line: the middle of the ${String(count)} branches' bars (how much of each is in ${state.target}); half are above it, half below. Unrelated branches still share some lines (license headers, common imports), so it is about the level that alone reaches.`;
    return node;
  }
  function render() {
    const root = document.querySelector("#root");
    if (root === null) {
      return;
    }
    const entries = [...state.entries];
    const finished = entries.filter(
      ([, entry]) => entry.result !== void 0 || entry.error !== void 0
    );
    const scores = entries.map(([, entry]) => entry.result?.a_in_b.containment).filter((score) => typeof score === "number");
    const middle = median(scores);
    const targetSide = entries.find(([, entry]) => entry.result)?.[1].result?.b;
    const header = element("header", "header stacked");
    const title = element("div", "heading");
    title.append(
      element("span", "title", `Containment in ${state.target}`),
      targetSide === void 0 ? element("span", "detail", "") : lines(targetSide.lines, targetSide.fork, state.target),
      medianNote(middle, scores.length),
      element(
        "span",
        "count",
        state.isDone ? branchCount(entries.length) : `${String(finished.length)} / ${String(entries.length)} compared`
      )
    );
    const similar = percent(state.threshold);
    const oneIn = String(Math.round(1 / (1 - state.threshold)));
    const fuzzyHelp = `A line of a branch counts as found if the target has the same line (exact, solid bar), or a line at least ${similar} similar (edited, light bar): about one character in ${oneIn} may differ (normalized Levenshtein distance). Whitespace is ignored.`;
    const toolbar = element("div", "toolbar");
    toolbar.append(
      control(
        "base",
        "Base",
        state.base,
        "The canonical branch: a branch's work is what it added since it left this one. Click to pick another."
      ),
      control(
        "target",
        "Target",
        state.target,
        "The branch every other branch is compared with. Click to pick another."
      ),
      control(
        "comments",
        "Comments",
        state.comments,
        state.comments === "ignored" ? "Lines that are only a comment (license headers, doc comments) in Python, Rust and C files are not counted as work. Click to count them, and compare again." : "Lines that are only a comment are counted as work. Click to leave them out, and compare again."
      ),
      info(`edited lines: \u2265 ${similar} similar`, fuzzyHelp),
      info(
        "commits only",
        "Only committed work is compared: uncommitted changes in a work tree are not checked."
      )
    );
    header.append(title, toolbar);
    const legend = element(
      "p",
      "legend",
      `Lines: what each branch added since it left ${state.base}, at the commit shown` + (state.comments === "ignored" ? ", comments left out. " : ". ") + `Bar: how much of them is in ${state.target} (solid exact, light edited); thin bar: how much of ${state.target} is in the branch. Click a branch to inspect it.`
    );
    const list = element("div", "rows");
    list.style.setProperty("--median", `${String((middle ?? 0) * 100)}%`);
    list.classList.toggle("has-median", middle !== void 0);
    entries.sort(([, a], [, b]) => compareResults(a.result, b.result));
    list.append(...entries.map(([branch, entry]) => branchRow(branch, entry)));
    list.addEventListener("click", (event) => {
      const row = event.target instanceof Element ? event.target.closest(".row.clickable") : void 0;
      const branch = row instanceof HTMLElement ? row.dataset["branch"] : void 0;
      if (branch === void 0) {
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
  function receive(message) {
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
      if (entry.result !== void 0) {
        rows += 1;
      }
    }
    send({ type: "rendered", oid: "containment", rows });
  }
  addEventListener("message", (event) => {
    if (event.origin !== origin) {
      return;
    }
    receive(event.data);
  });
  send({ type: "ready" });
})();
//# sourceMappingURL=containment.js.map
