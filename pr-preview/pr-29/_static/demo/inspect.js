"use strict";
(() => {
  // src/inspect.ts
  function groupLines(lines) {
    const files = [];
    for (const line of lines) {
      let file = files.at(-1);
      if (file?.path !== line.path) {
        file = { path: line.path, items: /* @__PURE__ */ new Map() };
        files.push(file);
      }
      const key = line.item ?? "";
      const group = file.items.get(key);
      if (group === void 0) {
        file.items.set(key, [line]);
      } else {
        group.push(line);
      }
    }
    return files.map((file) => ({
      path: file.path,
      items: [...file.items].map(([key, grouped]) => ({
        item: key === "" ? void 0 : key,
        lines: grouped
      }))
    }));
  }
  function textAt(lines) {
    return new Map(lines.map((line) => [`${line.path}:${String(line.line)}`, line.text]));
  }
  function isDifferent(line) {
    return line.found === "fuzzy" || line.found === "missing";
  }
  function segments(lines, isFolding) {
    if (!isFolding) {
      return lines.map((line) => ({ kind: "line", line }));
    }
    const out = [];
    let run = [];
    for (const line of lines) {
      if (isDifferent(line)) {
        if (run.length > 0) {
          out.push({ kind: "fold", lines: run });
          run = [];
        }
        out.push({ kind: "line", line });
      } else {
        run.push(line);
      }
    }
    if (run.length > 0) {
      out.push({ kind: "fold", lines: run });
    }
    return out;
  }
  function tally(lines) {
    const count = (found) => lines.filter((line) => line.found === found).length;
    return {
      exact: count("exact"),
      fuzzy: count("fuzzy"),
      missing: count("missing"),
      comment: count("comment")
    };
  }
  var DIFF_LIMIT = 500;
  var SEGMENTER = new Intl.Segmenter();
  function characters(text) {
    return Array.from(SEGMENTER.segment(text), (part) => part.segment);
  }
  function middle(a, b) {
    let start = 0;
    while (start < a.length && start < b.length && a.at(start) === b.at(start)) {
      start += 1;
    }
    let end = 0;
    while (end < a.length - start && end < b.length - start && a.at(a.length - 1 - end) === b.at(b.length - 1 - end)) {
      end += 1;
    }
    return a.map((_, index) => index >= start && index < a.length - end);
  }
  function lcsTable(a, b) {
    const rows = [Array.from({ length: b.length + 1 }, () => 0)];
    for (let row = a.length - 1; row >= 0; row -= 1) {
      const below = rows.at(-1) ?? [];
      const values = [0];
      let next = 0;
      for (let column = b.length - 1; column >= 0; column -= 1) {
        next = a.at(row) === b.at(column) ? (below.at(column + 1) ?? 0) + 1 : Math.max(below.at(column) ?? 0, next);
        values.push(next);
      }
      rows.push(values.toReversed());
    }
    return rows.toReversed();
  }
  function changed(text, other) {
    const a = characters(text);
    const b = characters(other);
    if (a.length * b.length > DIFF_LIMIT * DIFF_LIMIT) {
      return middle(a, b);
    }
    const lengths = lcsTable(a, b);
    const marks = [];
    let here = 0;
    let there = 0;
    while (here < a.length && there < b.length) {
      if (a.at(here) === b.at(there)) {
        marks.push(false);
        here += 1;
        there += 1;
      } else if ((lengths.at(here + 1)?.at(there) ?? 0) >= (lengths.at(here)?.at(there + 1) ?? 0)) {
        marks.push(true);
        here += 1;
      } else {
        there += 1;
      }
    }
    while (marks.length < a.length) {
      marks.push(true);
    }
    return marks;
  }
  function foldState(toggled, key, isOpenAtFirst) {
    const toggleKey = isOpenAtFirst ? `closed:${key}` : key;
    return { isOpen: toggled.has(toggleKey) !== isOpenAtFirst, toggleKey };
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

  // src/webview/inspect.ts
  var vscode = acquireVsCodeApi();
  function send(message) {
    vscode.postMessage(message);
  }
  var state = {
    message: void 0,
    way: "branch",
    mode: "differences",
    opened: /* @__PURE__ */ new Set(),
    isFocused: false
  };
  function view(message, way) {
    const { result } = message;
    return way === "branch" ? {
      own: result.a,
      other: result.b,
      ownName: message.branch,
      otherName: message.target,
      direction: result.a_in_b,
      texts: textAt(result.b_in_a.lines_found ?? [])
    } : {
      own: result.b,
      other: result.a,
      ownName: message.target,
      otherName: message.branch,
      direction: result.b_in_a,
      texts: textAt(result.a_in_b.lines_found ?? [])
    };
  }
  function opener(className, text, oid, path, line) {
    const node = element("span", `${className} link`, text);
    node.title = `Open ${path} at ${short(oid)}, line ${String(line)}`;
    node.addEventListener("click", () => {
      send({ type: "open", revision: { oid, path }, line });
    });
    return node;
  }
  function toggle(node, key) {
    node.classList.add("toggle");
    node.addEventListener("click", () => {
      if (state.opened.has(key)) {
        state.opened.delete(key);
      } else {
        state.opened.add(key);
      }
      render();
    });
    return node;
  }
  function counts(t) {
    const parts = [];
    for (const [count, label2] of [
      [t.exact, "same"],
      [t.fuzzy, "edited"],
      [t.missing, "not found"],
      [t.comment, "comments"]
    ]) {
      if (count > 0) {
        parts.push(`${String(count)} ${label2}`);
      }
    }
    return parts.join(", ");
  }
  function marked(text, other) {
    const node = element("span", "code");
    const marks = changed(text, other);
    let run = "";
    let isMarked = false;
    const flush = () => {
      if (run !== "") {
        node.append(isMarked ? element("mark", "", run) : run);
      }
      run = "";
    };
    const parts = characters(text);
    for (const [index, character] of parts.entries()) {
      const isChanged = marks.at(index) === true;
      if (isChanged !== isMarked) {
        flush();
        isMarked = isChanged;
      }
      run += character;
    }
    flush();
    return node;
  }
  function label(line, v) {
    switch (line.found) {
      case "fuzzy": {
        const edits = line.edits ?? 0;
        const node = element(
          "span",
          "found",
          `${percent(line.score)} similar \xB7 ${String(edits)} ${edits === 1 ? "char" : "chars"}`
        );
        node.title = `Found as an edited line: ${String(edits)} characters inserted, deleted or replaced (whitespace collapsed). Similarity = 1 \u2212 edits / length of the longer line = ${percent(line.score)}.`;
        return node;
      }
      case "missing": {
        return element("span", "found", `not in ${v.otherName}`);
      }
      case "exact": {
        return element("span", "found", "same");
      }
      case "comment": {
        return element("span", "found", "comment, left out");
      }
    }
  }
  function lineRows(line, v) {
    const block = element("div", `line-block found-${line.found}`);
    const at = line.at;
    const there = at === void 0 ? void 0 : v.texts.get(`${at.path}:${String(at.line)}`);
    const row = element("div", "line");
    row.append(
      opener("number", String(line.line), v.own.oid, line.path, line.line),
      label(line, v),
      there !== void 0 && line.found === "fuzzy" ? marked(line.text, there) : element("span", "code", line.text)
    );
    block.append(row);
    if (at === void 0 || line.found !== "fuzzy") {
      return block;
    }
    const place = at.path === line.path ? `:${String(at.line)}` : `${fileName(at.path)}:${String(at.line)}`;
    const under = element("div", "line other");
    under.append(
      element("span", "number"),
      opener("where", `${v.otherName} ${place}`, v.other.oid, at.path, at.line),
      there === void 0 ? element("span", "code") : marked(there, line.text)
    );
    block.append(under);
    return block;
  }
  function fold(lines, key, v) {
    if (state.opened.has(key)) {
      const close2 = toggle(element("div", "fold open", "\u25BE fold"), key);
      return [close2, ...lines.map((line) => lineRows(line, v))];
    }
    const first = lines[0]?.line ?? 0;
    const last = lines.at(-1)?.line ?? first;
    const span = first === last ? String(first) : `${String(first)}\u2013${String(last)}`;
    const summary2 = counts(tally(lines));
    return [toggle(element("div", "fold", `\xB7\xB7\xB7 ${span}: ${summary2}`), key)];
  }
  function itemHeading(group, path, v, key, isOpen) {
    const share = (v.direction.items ?? []).find(
      (item) => item.path === path && item.item === group.item
    );
    const heading = element("div", isOpen ? "item-heading" : "item-heading closed");
    heading.dataset["path"] = path;
    if (share?.start !== void 0) {
      heading.dataset["start"] = String(share.start);
    }
    heading.append(
      element("span", "caret", isOpen ? "\u25BE" : "\u25B8"),
      element("span", "item-kind", share?.kind ?? ""),
      element("span", "item-name", group.item ?? "outside items")
    );
    if (share?.start !== void 0 && share.end !== void 0) {
      heading.append(element("span", "item-lines", `${String(share.start)}\u2013${String(share.end)}`));
    }
    const summary2 = counts(tally(group.lines));
    heading.append(element("span", "item-counts", summary2));
    const there = share?.counterpart;
    if (share !== void 0 && there !== void 0) {
      const where = there.path === path ? "" : `${there.path} `;
      const target = element("span", "counterpart", `\u2192 ${where}${there.item ?? "outside items"}`);
      target.title = `${String(there.lines)} of its found lines are there, in ${v.otherName}`;
      heading.append(target);
      const moved = movement(path, share.item, there);
      if (moved !== void 0) {
        heading.append(element("span", "movement", moved));
      }
    }
    return toggle(heading, key);
  }
  function itemFold(group, path) {
    return {
      key: `item:${path}:${group.item ?? ""}`,
      isOpenAtFirst: state.mode === "all" || group.lines.some((line) => isDifferent(line))
    };
  }
  function fileFold(path, lines) {
    return {
      key: `file:${path}`,
      isOpenAtFirst: state.mode === "all" || lines.some((line) => isDifferent(line))
    };
  }
  function close({ key, isOpenAtFirst }) {
    if (isOpenAtFirst) {
      state.opened.add(`closed:${key}`);
    } else {
      state.opened.delete(key);
    }
  }
  function collapseAll(v) {
    const files = groupLines(v.direction.lines_found ?? []);
    for (const file of files) {
      close(
        fileFold(
          file.path,
          file.items.flatMap((group) => group.lines)
        )
      );
      for (const group of file.items) {
        close(itemFold(group, file.path));
      }
    }
    for (const key of state.opened) {
      if (key.startsWith("run:")) {
        state.opened.delete(key);
      }
    }
  }
  function itemBlock(group, path, v) {
    const isFolding = state.mode === "differences";
    const { key, isOpenAtFirst } = itemFold(group, path);
    const { isOpen, toggleKey } = foldState(state.opened, key, isOpenAtFirst);
    const heading = itemHeading(group, path, v, toggleKey, isOpen);
    if (!isOpen) {
      return [heading];
    }
    const rows = [heading];
    for (const segment of segments(group.lines, isFolding)) {
      if (segment.kind === "line") {
        rows.push(lineRows(segment.line, v));
      } else {
        rows.push(...fold(segment.lines, `run:${path}:${String(segment.lines[0]?.line)}`, v));
      }
    }
    return rows;
  }
  function fileSection(path, items, v) {
    const section = element("section", "file");
    const lines = items.flatMap((group) => group.lines);
    const { key, isOpenAtFirst } = fileFold(path, lines);
    const { isOpen, toggleKey } = foldState(state.opened, key, isOpenAtFirst);
    const summary2 = counts(tally(lines));
    const title = element("h3", "file-heading");
    title.dataset["path"] = path;
    const caret = toggle(element("span", "caret", isOpen ? "\u25BE" : "\u25B8"), toggleKey);
    title.append(
      caret,
      opener("file-name", path, v.own.oid, path, 1),
      element("span", "item-counts", summary2)
    );
    section.append(title);
    if (isOpen) {
      for (const group of items) {
        section.append(...itemBlock(group, path, v));
      }
    }
    return section;
  }
  function summary(v, message) {
    const d = v.direction;
    const t = tally(d.lines_found ?? []);
    const comments = message.comments === "ignored" && t.comment > 0 ? ` ${String(t.comment)} comment lines are left out.` : "";
    return element(
      "p",
      "legend",
      `Lines from ${v.ownName}, looked up in ${v.otherName}. ${v.ownName} added ${String(d.lines)} lines; ${percent(d.containment)} are in ${v.otherName}: ${String(d.exact)} the same, ${String(d.fuzzy)} edited, ${String(t.missing)} not there.${comments} ` + (state.mode === "differences" ? "Only edited and missing lines are shown; \u25B8 opens the rest." : "Every line is shown.")
    );
  }
  function choice(current, value, text, help, pick) {
    const button = element("button", current === value ? "control selected" : "control", text);
    button.type = "button";
    button.title = help;
    button.addEventListener("click", () => {
      pick(value);
      render();
    });
    return button;
  }
  function header(message, v) {
    const node = element("header", "header stacked");
    const heading = element("div", "heading");
    const since = work(v.own, v.ownName, message.base);
    const title = element("span", "title", `${v.ownName} \u2192 ${v.otherName}`);
    title.title = `The lines ${v.ownName} added, each looked up in ${v.otherName}`;
    heading.append(title, since);
    const toolbar = element("div", "toolbar");
    const setWay = (way) => {
      state.way = way;
    };
    const setMode = (mode) => {
      state.mode = mode;
    };
    const whose = element("span", "group");
    whose.append(
      element("span", "control-label", "Lines from"),
      choice(
        state.way,
        "branch",
        "Branch",
        `${message.branch}: what it added, each looked up in ${message.target}: how much of it is there already?`,
        setWay
      ),
      choice(
        state.way,
        "target",
        "Target",
        `${message.target}: what it added, each looked up in ${message.branch}: which parts of it did ${message.branch} take?`,
        setWay
      )
    );
    const show = element("span", "group");
    show.append(
      choice(
        state.mode,
        "differences",
        "Differences",
        "Edited and missing lines; the rest folded.",
        setMode
      ),
      choice(state.mode, "all", "All lines", "Every line, nothing folded.", setMode)
    );
    const collapse = element("button", "control", "Collapse all");
    collapse.type = "button";
    collapse.title = "Close every file and item; open them one by one with \u25B8.";
    collapse.addEventListener("click", () => {
      collapseAll(v);
      render();
    });
    show.append(collapse);
    toolbar.append(
      whose,
      show,
      info(
        `edited: \u2265 ${percent(message.threshold)} similar`,
        "An edited line is found as a line at least this similar: 1 \u2212 edits / length of the longer line. The part that differs is marked in both."
      )
    );
    node.append(heading, toolbar);
    return node;
  }
  function body(message, v) {
    const node = element("div", "inspect");
    const lines = v.direction.lines_found ?? [];
    if (lines.length === 0) {
      node.append(
        element("p", "message", `${v.ownName} added nothing since it left ${message.base}.`)
      );
      return node;
    }
    for (const file of groupLines(lines)) {
      node.append(fileSection(file.path, file.items, v));
    }
    return node;
  }
  function openFocus(message) {
    const target = message.focus;
    if (target === void 0 || state.isFocused) {
      return;
    }
    const file = `file:${target.path}`;
    state.opened.add(file);
    state.opened.delete(`closed:${file}`);
    const shares = message.result.a_in_b.items ?? [];
    for (const share of shares) {
      if (share.path !== target.path || share.start !== target.line) {
        continue;
      }
      const item = `item:${share.path}:${share.item ?? ""}`;
      state.opened.add(item);
      state.opened.delete(`closed:${item}`);
    }
  }
  function scrollToFocus(message) {
    const target = message.focus;
    if (target === void 0 || state.isFocused) {
      return;
    }
    state.isFocused = true;
    const headings = [...document.querySelectorAll("[data-path]")];
    const found = headings.find(
      (h) => h.dataset["path"] === target.path && target.line !== void 0 && h.dataset["start"] === String(target.line)
    ) ?? headings.find((h) => h.dataset["path"] === target.path);
    found?.scrollIntoView({ block: "start" });
    found?.classList.add("focused");
  }
  function render() {
    const root = document.querySelector("#root");
    const message = state.message;
    if (root === null || message === void 0) {
      return;
    }
    if (message.type === "error") {
      root.replaceChildren(
        element("p", "message", `Can't inspect ${message.branch}: ${message.message}`),
        footer(void 0)
      );
      send({ type: "rendered", oid: message.branch, rows: 0 });
      return;
    }
    openFocus(message);
    const v = view(message, state.way);
    root.replaceChildren(
      header(message, v),
      summary(v, message),
      body(message, v),
      footer(message.meta)
    );
    scrollToFocus(message);
    send({ type: "rendered", oid: message.branch, rows: v.direction.lines_found?.length ?? 0 });
  }
  addEventListener("message", (event) => {
    if (event.origin !== origin) {
      return;
    }
    state.message = event.data;
    state.way = "branch";
    state.opened = /* @__PURE__ */ new Set();
    state.isFocused = false;
    render();
  });
  send({ type: "ready" });
})();
//# sourceMappingURL=inspect.js.map
