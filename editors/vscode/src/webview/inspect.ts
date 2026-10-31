// The inspect page: every line a branch added, by file and item, next to the line of the target it
// was found as (or not found, or left out as a comment). Either way round. Runs in the webview;
// bundled to dist/inspect.js.

import type { Direction, ItemShare, LineFound, Side } from "../containment.js";
import { groupLines, textAt, type ItemGroup, type ToInspectPage } from "../inspect.js";
import type { FromPage } from "../messages.js";
import { element, fileName, footer, info, movement, percent, short } from "./common.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

type Inspection = Extract<ToInspectPage, { type: "inspect" }>;

/** Which way round: the branch's lines in the target, or the target's in the branch. */
type Way = "branch" | "target";

const state: { message: ToInspectPage | undefined; way: Way; isFocused: boolean } = {
  message: undefined,
  way: "branch",
  isFocused: false,
};

const FOUND_LABELS: ReadonlyMap<LineFound["found"], string> = new Map([
  ["exact", "exact"],
  ["fuzzy", "edited"],
  ["missing", "not found"],
  ["comment", "comment"],
]);

function foundLabel(found: LineFound["found"]): string {
  return FOUND_LABELS.get(found) ?? found;
}

/** One way round: whose lines, found where. */
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

/**
`text` with the part that differs from `other` marked: what is left between their common start
and common end.
*/
function marked(text: string, other: string): HTMLElement {
  let start = 0;
  while (
    start < text.length &&
    start < other.length &&
    text.charAt(start) === other.charAt(start)
  ) {
    start += 1;
  }
  let end = 0;
  while (
    end < text.length - start &&
    end < other.length - start &&
    text.charAt(text.length - 1 - end) === other.charAt(other.length - 1 - end)
  ) {
    end += 1;
  }
  const node = element("span", "code");
  node.append(
    text.slice(0, start),
    element("mark", "", text.slice(start, text.length - end)),
    text.slice(text.length - end),
  );
  return node;
}

/** One line: number, how it was found, its text, and where it was found with that line's text. */
function lineRow(line: LineFound, v: View): HTMLElement {
  const row = element("div", `line found-${line.found}`);
  row.append(
    opener("number", String(line.line), v.own.oid, line.path, line.line),
    element("span", "found", line.found === "fuzzy" ? percent(line.score) : foundLabel(line.found)),
  );
  const at = line.at;
  const there = at === undefined ? undefined : v.texts.get(`${at.path}:${String(at.line)}`);
  row.append(
    there !== undefined && line.found === "fuzzy"
      ? marked(line.text, there)
      : element("span", "code", line.text),
  );
  if (at === undefined) {
    row.append(element("span", "where"), element("span", "code other"));
    return row;
  }
  const place =
    at.path === line.path ? `:${String(at.line)}` : `${fileName(at.path)}:${String(at.line)}`;
  row.append(opener("where", `→ ${place}`, v.other.oid, at.path, at.line));
  let shown: HTMLElement;
  if (line.found === "exact") {
    shown = element("span", "code other same", "same line");
  } else if (there === undefined) {
    shown = element("span", "code other", "");
  } else {
    shown = marked(there, line.text);
    shown.classList.add("other");
  }
  row.append(shown);
  return row;
}

/** An item's heading: its name and lines, and where most of it went. */
function itemHeading(group: ItemGroup, path: string, v: View): HTMLElement {
  const share: ItemShare | undefined = (v.direction.items ?? []).find(
    (item) => item.path === path && item.item === group.item,
  );
  const heading = element("div", "item-heading");
  heading.dataset["path"] = path;
  if (share?.start !== undefined) {
    heading.dataset["start"] = String(share.start);
  }
  heading.append(
    element("span", "item-kind", share?.kind ?? ""),
    element("span", "item-name", group.item ?? "outside items"),
  );
  if (share?.start !== undefined && share.end !== undefined) {
    heading.append(element("span", "item-lines", `${String(share.start)}–${String(share.end)}`));
  }
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
  return heading;
}

function summary(v: View, message: Inspection): HTMLElement {
  const d = v.direction;
  const comments = v.own.comment_lines;
  let commentNote = "";
  if (comments > 0) {
    commentNote =
      message.comments === "ignored"
        ? `; ${String(comments)} comment lines left out`
        : `; ${String(comments)} of them comments`;
  }
  return element(
    "p",
    "legend",
    `${String(d.exact + d.fuzzy)} of ${v.ownName}'s ${String(d.lines)} lines are in ` +
      `${v.otherName} (${percent(d.containment)}): ${String(d.exact)} exact, ` +
      `${String(d.fuzzy)} edited${commentNote}. Click a line number or a place to open the file.`,
  );
}

function wayButton(way: Way, label: string): HTMLButtonElement {
  const button = element("button", state.way === way ? "control selected" : "control", label);
  button.type = "button";
  button.addEventListener("click", () => {
    state.way = way;
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
  heading.append(element("span", "title", `${v.ownName} in ${v.otherName}`), since);
  const toolbar = element("div", "toolbar");
  toolbar.append(
    wayButton("branch", `${message.branch} in ${message.target}`),
    wayButton("target", `${message.target} in ${message.branch}`),
  );
  const key = element("span", "key");
  for (const found of ["exact", "fuzzy", "missing", "comment"] as const) {
    key.append(element("span", `chip found-${found}`, foundLabel(found)));
  }
  toolbar.append(
    key,
    info(
      `edited: ≥ ${percent(message.threshold)} similar`,
      "An edited line is found as a line at least this similar (normalized Levenshtein); the part that differs is marked.",
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
    const section = element("section", "file");
    const title = element("h3", "file-heading");
    title.dataset["path"] = file.path;
    title.append(opener("file-name", file.path, v.own.oid, file.path, 1));
    section.append(title);
    for (const group of file.items) {
      section.append(itemHeading(group, file.path, v));
      section.append(...group.lines.map((line) => lineRow(line, v)));
    }
    node.append(section);
  }
  return node;
}

/** Scroll to the focus the extension asked for, once. */
function focus(message: Inspection): void {
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
  const v = view(message, state.way);
  root.replaceChildren(
    header(message, v),
    summary(v, message),
    body(message, v),
    footer(message.meta),
  );
  focus(message);
  send({ type: "rendered", oid: message.branch, rows: v.direction.lines_found?.length ?? 0 });
}

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToInspectPage>) => {
  if (event.origin !== origin) {
    return;
  }
  state.message = event.data;
  state.way = "branch";
  state.isFocused = false;
  render();
});

send({ type: "ready" });
