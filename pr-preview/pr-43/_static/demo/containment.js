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

  // src/stage.ts
  var COPY_HELP = "the server's copy of your branch, with commits your branch doesn't have.";
  var STAGES = {
    "open none": { stage: "draft", help: "Not merged, never pushed." },
    "open gone": {
      stage: "loose",
      help: "Not merged by hash (maybe not merged into the base at all), remote branch deleted. Usually a squash merge."
    },
    "open level": { stage: "steady", help: "Not merged; the same as its remote branch." },
    "open ahead": { stage: "budding", help: "Not merged; has commits its remote branch doesn't." },
    "open behind": {
      stage: "lagging",
      help: "Not merged; its remote branch has commits it doesn't."
    },
    "open diverged": {
      stage: "tangled",
      help: "Not merged; it and its remote branch each have commits the other doesn't."
    },
    "open remote-only": { stage: "remote", help: "Not merged; only on the remote, no local branch." },
    "open remote-copy": { stage: "remote", help: `Not merged; ${COPY_HELP}` },
    "merged none": { stage: "done", help: "Merged into the base; never pushed." },
    "merged gone": { stage: "done", help: "Merged into the base; remote branch deleted." },
    "merged level": { stage: "done", help: "Merged into the base; remote branch still there." },
    "merged ahead": {
      stage: "budding",
      help: "Merged into the base; has commits its remote branch doesn't."
    },
    "merged behind": {
      stage: "lagging",
      help: "Merged into the base; its remote branch has newer commits that aren't."
    },
    "merged diverged": {
      stage: "tangled",
      help: "Merged into the base; it and its remote branch each have commits the other doesn't, and the remote's aren't in the base."
    },
    "merged remote-only": {
      stage: "done",
      help: "Merged into the base; only on the remote, no local branch."
    },
    "merged remote-copy": { stage: "remote", help: `Merged into the base; ${COPY_HELP}` }
  };
  var STAGE_MARKS = {
    draft: { name: "Draft", mark: "\u25CB", tone: "muted", summary: "Not merged, never pushed." },
    loose: {
      name: "Loose",
      mark: "\u2298",
      tone: "warn",
      summary: "Not merged, remote branch deleted: usually a squash merge."
    },
    steady: {
      name: "Steady",
      mark: "\u2550",
      tone: "text",
      summary: "Not merged, the same as its remote branch."
    },
    budding: {
      name: "Budding",
      mark: "\u25B2",
      tone: "warn",
      summary: "Has commits its remote branch doesn't."
    },
    lagging: {
      name: "Lagging",
      mark: "\u25BC",
      tone: "warn",
      summary: "Its remote branch has commits it doesn't."
    },
    tangled: {
      name: "Tangled",
      mark: "\u22C8",
      tone: "bad",
      summary: "It and its remote branch each have commits the other doesn't."
    },
    remote: {
      name: "Remote",
      mark: "\u25C7",
      tone: "muted",
      summary: "Only on the remote, or the remote's copy of a local branch drawn beside it."
    },
    done: {
      name: "Done",
      mark: "\u25C6",
      tone: "good",
      summary: "Merged into the base, and its remote branch is gone or the same."
    }
  };
  var BASE = {
    text: "\u25C9",
    help: "Base: every other lane is placed against it.",
    tone: "muted"
  };
  var UNCLEAR = { text: "?", help: "Unclear: no branch data for this lane.", tone: "muted" };
  var MERGED = {
    merged: { text: "\u2208", help: "Merged into the base.", tone: "muted" },
    open: { text: "\u2209", help: "Not merged into the base.", tone: "muted" },
    base: { text: "\xB7", help: "The base itself.", tone: "muted" }
  };
  var REMOTE = {
    none: { text: "\u2205", help: "Never pushed: no remote branch.", tone: "muted" },
    gone: { text: "\xD7", help: "Remote branch deleted.", tone: "muted" },
    level: { text: "=", help: "The same as its remote branch.", tone: "muted" },
    ahead: { text: "+", help: "Has commits its remote branch doesn't.", tone: "muted" },
    behind: { text: "\u2212", help: "Its remote branch has commits it doesn't.", tone: "muted" },
    diverged: {
      text: "\xB1",
      help: "It and its remote branch each have commits the other doesn't.",
      tone: "muted"
    },
    "remote-only": { text: "\u2204", help: "Only on the remote: no local branch.", tone: "muted" },
    "remote-copy": { text: "\u2248", help: "The server's copy of a local branch.", tone: "muted" }
  };
  var NO_REMOTE_DATA = { text: "\xB7", help: "No branch data.", tone: "muted" };
  var REMOTE_MARKS = new Map(Object.entries(REMOTE));
  var WORKTREE = {
    none: { text: "\xB7", help: "Not checked out.", tone: "muted" },
    unchanged: { text: "\u25A1", help: "", tone: "muted" },
    changed: { text: "\u25A0", help: "", tone: "warn" }
  };
  var STAGE_LEGEND = [
    ...Object.values(STAGE_MARKS).map(({ name, mark, tone, summary }) => ({
      text: mark,
      name,
      summary,
      tone
    })),
    { text: UNCLEAR.text, name: "Unclear", summary: "No branch data for the lane.", tone: "muted" },
    {
      text: BASE.text,
      name: "Base",
      summary: "Every other lane is placed against it.",
      tone: "muted"
    }
  ];
  var STATE_LEGEND = [
    { ...MERGED.merged, name: "Merged", summary: MERGED.merged.help },
    { ...MERGED.open, name: "Not merged", summary: MERGED.open.help },
    { ...REMOTE.none, name: "Never pushed", summary: "No remote branch." },
    { ...REMOTE.gone, name: "Deleted", summary: "Its remote branch was deleted." },
    { ...REMOTE.level, name: "Same", summary: REMOTE.level.help },
    { ...REMOTE.ahead, name: "Ahead", summary: REMOTE.ahead.help },
    { ...REMOTE.behind, name: "Behind", summary: REMOTE.behind.help },
    { ...REMOTE.diverged, name: "Both", summary: REMOTE.diverged.help },
    { ...REMOTE["remote-only"], name: "No local", summary: REMOTE["remote-only"].help },
    { ...REMOTE["remote-copy"], name: "Copy", summary: REMOTE["remote-copy"].help },
    { ...WORKTREE.none, name: "Not checked out", summary: "Not checked out in any work tree." },
    { ...WORKTREE.unchanged, name: "Unchanged", summary: "Checked out, no uncommitted changes." },
    { ...WORKTREE.changed, name: "Changed", summary: "Checked out, with uncommitted changes." }
  ];
  function tracking(lane, state2) {
    if (lane.kind === "remote") {
      return lane.only === "remote" ? "remote-only" : "remote-copy";
    }
    if (state2 === void 0 || state2.isRemote) {
      return void 0;
    }
    if (state2.upstream === void 0) {
      return "none";
    }
    if (state2.isGone === true) {
      return "gone";
    }
    const isAhead = (state2.ahead ?? 0) > 0;
    const isBehind = (state2.behind ?? 0) > 0;
    if (isAhead && isBehind) {
      return "diverged";
    }
    if (isAhead) {
      return "ahead";
    }
    return isBehind ? "behind" : "level";
  }
  function stageMark(lane, state2) {
    if (lane.kind === "reference") {
      return BASE;
    }
    const how = tracking(lane, state2);
    if (how === void 0) {
      return UNCLEAR;
    }
    const row = STAGES[`${lane.merged ? "merged" : "open"} ${how}`];
    const { name, mark, tone } = STAGE_MARKS[row.stage];
    return { text: mark, help: `${name}: ${row.help}`, tone };
  }
  function worktreeMark(state2) {
    const worktree = state2?.worktree;
    if (worktree === void 0) {
      return WORKTREE.none;
    }
    const changes = state2?.changes;
    if (changes === void 0) {
      return { ...WORKTREE.unchanged, help: `Checked out in ${worktree}.` };
    }
    return changes.changed + changes.untracked === 0 ? { ...WORKTREE.unchanged, help: `Checked out in ${worktree}; no uncommitted changes.` } : {
      ...WORKTREE.changed,
      help: `Checked out in ${worktree}; uncommitted: ${String(changes.changed)} changed, ${String(changes.untracked)} untracked files.`
    };
  }
  function stateMarks(lane, state2) {
    let merged = lane.merged ? MERGED.merged : MERGED.open;
    if (lane.kind === "reference") {
      merged = MERGED.base;
    }
    const how = tracking(lane, state2);
    const remote = how === void 0 ? void 0 : REMOTE_MARKS.get(how);
    return [merged, remote ?? NO_REMOTE_DATA, worktreeMark(state2)];
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
  function work(side, branch, base) {
    const node = element("span", "work");
    const fork = element("span", "fork", short(side.fork));
    node.append(`+${String(side.lines)} \xB7 ${String(side.commits)}c \xB7 `, fork);
    const commits = side.commits === 1 ? "1 commit" : `${String(side.commits)} commits`;
    node.title = `${branch} added ${String(side.lines)} lines in ${commits} since it left ${base} at ` + side.fork + (side.comment_lines > 0 ? ` (and ${String(side.comment_lines)} comment lines).` : ".");
    return node;
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
  function repositoryControl(repository, onClick) {
    if (repository === void 0) {
      return [];
    }
    const name = repository.split(/[\\/]/u).findLast((part) => part !== "") ?? repository;
    const help = `${repository}
Click to pick another: an open folder's repository, one of its submodules, or the active file's.`;
    return [controlButton("Repository", name, help, onClick)];
  }
  function markSpan(mark) {
    const node = element("span", `mark-${mark.tone}`, mark.text);
    node.title = mark.help;
    return node;
  }
  function legendSection(title, intro, rows) {
    const section = element("div", "legend-section");
    const table = element("table", "legend");
    for (const row of rows) {
      const symbol = element("td", "legend-mark");
      symbol.append(markSpan({ text: row.text, help: row.name, tone: row.tone }));
      const line = element("tr", "");
      line.append(
        symbol,
        element("td", "legend-name", row.name),
        element("td", "legend-summary", row.summary)
      );
      table.append(line);
    }
    section.append(element("p", "legend-title", title), element("p", "help-intro", intro), table);
    return section;
  }
  function legendHelp() {
    const cell = element("span", "column-help");
    const mark = element("span", "help-mark", "?");
    mark.tabIndex = 0;
    mark.setAttribute("aria-label", "What the Stage and State marks mean");
    const card = element("div", "help-card");
    card.append(
      legendSection(
        "Stage",
        "Where the branch is in its life, from whether it is merged into the base and how it stands against its remote branch. Hover a symbol in a label for its exact case.",
        STAGE_LEGEND
      ),
      legendSection(
        "State",
        "The facts the stage is made from, always in three slots: merged into the base, the remote branch, the work tree.",
        STATE_LEGEND
      )
    );
    mark.append(card);
    cell.append(mark);
    return cell;
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
    repository: void 0,
    target: "",
    base: "",
    comments: "ignored",
    threshold: 0.9,
    meta: void 0,
    states: /* @__PURE__ */ new Map(),
    merged: /* @__PURE__ */ new Set(),
    entries: /* @__PURE__ */ new Map(),
    expanded: /* @__PURE__ */ new Set(),
    isDone: false
  };
  function control(name, label, value, help) {
    return controlButton(label, value, help, () => {
      send({ type: "choose", control: name });
    });
  }
  function historyButton() {
    const button = element("button", "control", "History");
    button.type = "button";
    button.title = `The branches around ${state.base}'s tip, ranked by how much of their work is in ${state.target}`;
    button.addEventListener("click", () => {
      send({ type: "history" });
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
    const history = element("button", "control", "History");
    history.type = "button";
    history.title = `The branches around ${state.base}'s tip, ${branch} among them`;
    history.addEventListener("click", () => {
      send({ type: "history", branch });
    });
    actions.append(all, history);
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
  function nameCell(branch) {
    const node = element("span", "name-cell");
    node.append(element("span", "name", branch));
    const where = state.states.get(branch);
    if (where !== void 0) {
      const isMerged = state.merged.has(branch);
      const lane = where.isRemote ? { kind: "remote", only: "remote", merged: isMerged } : { kind: "local", merged: isMerged };
      const marks = element("span", "branch-marks");
      const stage = element("span", "lane-stage");
      stage.append(markSpan(stageMark(lane, where)));
      const facts = element("span", "lane-state");
      facts.append(...stateMarks(lane, where).map((mark) => markSpan(mark)));
      marks.append(stage, facts);
      node.append(marks);
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
      work(result.a, branch, state.base),
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
      targetSide === void 0 ? element("span", "detail", "") : work(targetSide, state.target, state.base),
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
      ...repositoryControl(state.repository, () => {
        send({ type: "choose", control: "repository" });
      }),
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
      historyButton(),
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
    if (state.states.size > 0) {
      legend.append(" Stage and state of each branch: ", legendHelp());
    }
    root.replaceChildren(header, legend, list, footer(state.meta));
  }
  function receive(message) {
    switch (message.type) {
      case "start": {
        state.repository = message.repository;
        state.target = message.target;
        state.base = message.base;
        state.comments = message.comments;
        state.threshold = message.threshold;
        state.meta = message.meta;
        state.states = new Map((message.states ?? []).map((branch) => [branch.name, branch]));
        state.merged = new Set(message.merged);
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
