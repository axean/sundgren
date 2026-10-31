// The inspect page: the lines one branch added, by file and item, each looked up in the other
// branch. By default only what differs (edited and not found lines) is shown; the rest is folded.
// Runs in the webview; bundled to dist/inspect.js.

import type { Direction, ItemShare, LineFound, Side } from "../containment.js";
import {
  changed,
  foldState,
  characters,
  groupLines,
  isDifferent,
  segments,
  tally,
  textAt,
  type ItemGroup,
  type Tally,
  type ToInspectPage,
} from "../inspect.js";
import type { FromPage } from "../messages.js";
import { element, fileName, footer, info, movement, percent, short } from "./common.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

type Inspection = Extract<ToInspectPage, { type: "inspect" }>;

/** Whose lines: the branch's, looked up in the target, or the target's, looked up in the branch. */
type Way = "branch" | "target";

/** Only what differs, folding the rest, or every line. */
type Mode = "differences" | "all";

const state: {
  message: ToInspectPage | undefined;
  way: Way;
  mode: Mode;
  /** Folds the user opened: files, items and runs, by key. */
  opened: Set<string>;
  isFocused: boolean;
} = {
  message: undefined,
  way: "branch",
  mode: "differences",
  opened: new Set(),
  isFocused: false,
};

/** One way round: whose lines, looked up where. */
interface View {
  readonly own: Side;
  readonly other: Side;
  readonly ownName: string;
  readonly otherName: string;
  readonly direction: Direction;
  /** The other side's lines by `path:line`, for the text a match points at. */
  readonly texts: Map<string, string>;
}

function view(message: Inspection, way: Way): View {
  const { result } = message;
  return way === "branch"
    ? {
        own: result.a,
        other: result.b,
        ownName: message.branch,
        otherName: message.target,
        direction: result.a_in_b,
        texts: textAt(result.b_in_a.lines_found ?? []),
      }
    : {
        own: result.b,
        other: result.a,
        ownName: message.target,
        otherName: message.branch,
        direction: result.b_in_a,
        texts: textAt(result.a_in_b.lines_found ?? []),
      };
}

/** Text that opens a file at a commit when clicked. */
function opener(className: string, text: string, oid: string, path: string, line: number) {
  const node = element("span", `${className} link`, text);
  node.title = `Open ${path} at ${short(oid)}, line ${String(line)}`;
  node.addEventListener("click", () => {
    send({ type: "open", revision: { oid, path }, line });
  });
  return node;
}

/** Something that opens (or closes) the fold `key` when clicked. */
function toggle(node: HTMLElement, key: string): HTMLElement {
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

/** "30 exact, 2 edited, 1 not found, 4 comments", leaving out what is zero. */
function counts(t: Tally): string {
  const parts: string[] = [];
  for (const [count, label] of [
    [t.exact, "same"],
    [t.fuzzy, "edited"],
    [t.missing, "not found"],
    [t.comment, "comments"],
  ] as const) {
    if (count > 0) {
      parts.push(`${String(count)} ${label}`);
    }
  }
  return parts.join(", ");
}

/** `text` with the characters `other` doesn't have marked. */
function marked(text: string, other: string): HTMLElement {
  const node = element("span", "code");
  const marks = changed(text, other);
  let run = "";
  let isMarked = false;
  const flush = (): void => {
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

/** How a line was found, as its label says it. */
function label(line: LineFound, v: View): HTMLElement {
  switch (line.found) {
    case "fuzzy": {
      const edits = line.edits ?? 0;
      const node = element(
        "span",
        "found",
        `${percent(line.score)} similar · ${String(edits)} ${edits === 1 ? "char" : "chars"}`,
      );
      node.title =
        `Found as an edited line: ${String(edits)} characters inserted, deleted or replaced ` +
        `(whitespace collapsed). Similarity = 1 − edits / length of the longer line = ` +
        `${percent(line.score)}.`;
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

/**
A line, and under it the line of the other branch it was found as, so the two line up character
by character; the part that differs is marked in both.
*/
function lineRows(line: LineFound, v: View): HTMLElement {
  const block = element("div", `line-block found-${line.found}`);
  const at = line.at;
  const there = at === undefined ? undefined : v.texts.get(`${at.path}:${String(at.line)}`);
  const row = element("div", "line");
  row.append(
    opener("number", String(line.line), v.own.oid, line.path, line.line),
    label(line, v),
    there !== undefined && line.found === "fuzzy"
      ? marked(line.text, there)
      : element("span", "code", line.text),
  );
  block.append(row);
  if (at === undefined || line.found !== "fuzzy") {
    return block;
  }
  const place =
    at.path === line.path ? `:${String(at.line)}` : `${fileName(at.path)}:${String(at.line)}`;
  const under = element("div", "line other");
  under.append(
    element("span", "number"),
    opener("where", `${v.otherName} ${place}`, v.other.oid, at.path, at.line),
    there === undefined ? element("span", "code") : marked(there, line.text),
  );
  block.append(under);
  return block;
}

/** A folded run of lines: "··· 12 same", which opens to show them. */
function fold(lines: readonly LineFound[], key: string, v: View): HTMLElement[] {
  if (state.opened.has(key)) {
    const close = toggle(element("div", "fold open", "▾ fold"), key);
    return [close, ...lines.map((line) => lineRows(line, v))];
  }
  const first = lines[0]?.line ?? 0;
  const last = lines.at(-1)?.line ?? first;
  const span = first === last ? String(first) : `${String(first)}–${String(last)}`;
  const summary = counts(tally(lines));
  return [toggle(element("div", "fold", `··· ${span}: ${summary}`), key)];
}

/** An item's heading: its kind, name and lines, how its lines were found, and where it went. */
function itemHeading(group: ItemGroup, path: string, v: View, key: string, isOpen: boolean) {
  const share: ItemShare | undefined = (v.direction.items ?? []).find(
    (item) => item.path === path && item.item === group.item,
  );
  const heading = element("div", isOpen ? "item-heading" : "item-heading closed");
  heading.dataset["path"] = path;
  if (share?.start !== undefined) {
    heading.dataset["start"] = String(share.start);
  }
  heading.append(
    element("span", "caret", isOpen ? "▾" : "▸"),
    element("span", "item-kind", share?.kind ?? ""),
    element("span", "item-name", group.item ?? "outside items"),
  );
  if (share?.start !== undefined && share.end !== undefined) {
    heading.append(element("span", "item-lines", `${String(share.start)}–${String(share.end)}`));
  }
  const summary = counts(tally(group.lines));
  heading.append(element("span", "item-counts", summary));
  const there = share?.counterpart;
  if (share !== undefined && there !== undefined) {
    const where = there.path === path ? "" : `${there.path} `;
    const target = element("span", "counterpart", `→ ${where}${there.item ?? "outside items"}`);
    target.title = `${String(there.lines)} of its found lines are there, in ${v.otherName}`;
    heading.append(target);
    const moved = movement(path, share.item, there);
    if (moved !== undefined) {
      heading.append(element("span", "movement", moved));
    }
  }
  return toggle(heading, key);
}

/** One item's lines: everything, or only what differs with the rest folded. */
function itemBlock(group: ItemGroup, path: string, v: View): HTMLElement[] {
  const key = `item:${path}:${group.item ?? ""}`;
  const isFolding = state.mode === "differences";
  const hasDifferences = group.lines.some((line) => isDifferent(line));
  // With every line shown, or something different in it, an item starts open.
  const { isOpen, toggleKey } = foldState(state.opened, key, !isFolding || hasDifferences);
  const heading = itemHeading(group, path, v, toggleKey, isOpen);
  if (!isOpen) {
    return [heading];
  }
  const rows: HTMLElement[] = [heading];
  for (const segment of segments(group.lines, isFolding)) {
    if (segment.kind === "line") {
      rows.push(lineRows(segment.line, v));
    } else {
      rows.push(...fold(segment.lines, `run:${path}:${String(segment.lines[0]?.line)}`, v));
    }
  }
  return rows;
}

/** One file: its heading, and its items unless it is closed. */
function fileSection(path: string, items: readonly ItemGroup[], v: View): HTMLElement {
  const section = element("section", "file");
  const lines = items.flatMap((group) => group.lines);
  const hasDifferences = lines.some((line) => isDifferent(line));
  const key = `file:${path}`;
  const { isOpen, toggleKey } = foldState(
    state.opened,
    key,
    state.mode === "all" || hasDifferences,
  );
  const summary = counts(tally(lines));
  const title = element("h3", "file-heading");
  title.dataset["path"] = path;
  const caret = toggle(element("span", "caret", isOpen ? "▾" : "▸"), toggleKey);
  title.append(
    caret,
    opener("file-name", path, v.own.oid, path, 1),
    element("span", "item-counts", summary),
  );
  section.append(title);
  if (isOpen) {
    for (const group of items) {
      section.append(...itemBlock(group, path, v));
    }
  }
  return section;
}

function summary(v: View, message: Inspection): HTMLElement {
  const d = v.direction;
  const t = tally(d.lines_found ?? []);
  const comments =
    message.comments === "ignored" && t.comment > 0
      ? ` ${String(t.comment)} comment lines are left out.`
      : "";
  return element(
    "p",
    "legend",
    `Lines from ${v.ownName}, looked up in ${v.otherName}. ` +
      `${v.ownName} added ${String(d.lines)} lines; ${percent(d.containment)} are in ` +
      `${v.otherName}: ${String(d.exact)} the same, ${String(d.fuzzy)} edited, ` +
      `${String(t.missing)} not there.${comments} ` +
      (state.mode === "differences"
        ? "Only edited and missing lines are shown; ▸ opens the rest."
        : "Every line is shown."),
  );
}

function choice<T extends string>(
  current: T,
  value: T,
  text: string,
  help: string,
  pick: (value: T) => void,
): HTMLButtonElement {
  const button = element("button", current === value ? "control selected" : "control", text);
  button.type = "button";
  button.title = help;
  button.addEventListener("click", () => {
    pick(value);
    render();
  });
  return button;
}

function header(message: Inspection, v: View): HTMLElement {
  const node = element("header", "header stacked");
  const heading = element("div", "heading");
  const since = element("span", "detail", `${String(v.own.lines)} lines since `);
  const fork = element("span", "fork", short(v.own.fork));
  fork.title = `${v.ownName} left ${message.base} at ${v.own.fork}`;
  since.append(fork);
  const title = element("span", "title", `${v.ownName} → ${v.otherName}`);
  title.title = `The lines ${v.ownName} added, each looked up in ${v.otherName}`;
  heading.append(title, since);

  const toolbar = element("div", "toolbar");
  const setWay = (way: Way): void => {
    state.way = way;
  };
  const setMode = (mode: Mode): void => {
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
      setWay,
    ),
    choice(
      state.way,
      "target",
      "Target",
      `${message.target}: what it added, each looked up in ${message.branch}: which parts of it did ${message.branch} take?`,
      setWay,
    ),
  );
  const show = element("span", "group");
  show.append(
    choice(
      state.mode,
      "differences",
      "Differences",
      "Edited and missing lines; the rest folded.",
      setMode,
    ),
    choice(state.mode, "all", "All lines", "Every line, nothing folded.", setMode),
  );
  toolbar.append(
    whose,
    show,
    info(
      `edited: ≥ ${percent(message.threshold)} similar`,
      "An edited line is found as a line at least this similar: 1 − edits / length of the longer line. The part that differs is marked in both.",
    ),
  );
  node.append(heading, toolbar);
  return node;
}

function body(message: Inspection, v: View): HTMLElement {
  const node = element("div", "inspect");
  const lines = v.direction.lines_found ?? [];
  if (lines.length === 0) {
    node.append(
      element("p", "message", `${v.ownName} added nothing since it left ${message.base}.`),
    );
    return node;
  }
  for (const file of groupLines(lines)) {
    node.append(fileSection(file.path, file.items, v));
  }
  return node;
}

/** Open what the extension asked to focus on, once. */
function openFocus(message: Inspection): void {
  const target = message.focus;
  if (target === undefined || state.isFocused) {
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

/** Scroll to the focus the extension asked for, once. */
function scrollToFocus(message: Inspection): void {
  const target = message.focus;
  if (target === undefined || state.isFocused) {
    return;
  }
  state.isFocused = true;
  const headings = [...document.querySelectorAll<HTMLElement>("[data-path]")];
  const found =
    headings.find(
      (h) =>
        h.dataset["path"] === target.path &&
        target.line !== undefined &&
        h.dataset["start"] === String(target.line),
    ) ?? headings.find((h) => h.dataset["path"] === target.path);
  found?.scrollIntoView({ block: "start" });
  found?.classList.add("focused");
}

function render(): void {
  const root = document.querySelector("#root");
  const message = state.message;
  if (root === null || message === undefined) {
    return;
  }
  if (message.type === "error") {
    root.replaceChildren(
      element("p", "message", `Can't inspect ${message.branch}: ${message.message}`),
      footer(undefined),
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
    footer(message.meta),
  );
  scrollToFocus(message);
  send({ type: "rendered", oid: message.branch, rows: v.direction.lines_found?.length ?? 0 });
}

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToInspectPage>) => {
  if (event.origin !== origin) {
    return;
  }
  state.message = event.data;
  state.way = "branch";
  state.opened = new Set();
  state.isFocused = false;
  render();
});

send({ type: "ready" });
