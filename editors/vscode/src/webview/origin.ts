// The origin page: for one file, per commit of another repository, the file there its lines are
// most found in, as rows like the containment page's. Runs in the webview; bundled to
// dist/origin.js.

import type { Meta } from "../containment.js";
import type { Control, FromPage } from "../messages.js";
import { CLEAR_LEAD, verdict, type FileOrigin, type Origin, type ToOriginPage } from "../origin.js";
import { controlButton, element, footer, info, percent, short } from "./common.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

const VERDICTS = {
  clear: { className: "a-in-b", text: "clear" },
  close: { className: "overlap", text: "close call" },
  none: { className: "unrelated", text: "no source" },
} as const;

const state: {
  path: string;
  other: string;
  refs: readonly string[];
  threshold: number;
  meta: Meta | undefined;
  file: FileOrigin | undefined;
  error: string | undefined;
} = {
  path: "",
  other: "",
  refs: [],
  threshold: 0.9,
  meta: undefined,
  file: undefined,
  error: undefined,
};

function control(name: Control, label: string, value: string, help: string): HTMLButtonElement {
  return controlButton(label, value, help, () => {
    send({ type: "choose", control: name });
  });
}

function bar(className: string, fraction: number): HTMLDivElement {
  const node = element("div", className);
  node.style.width = `${String(Math.max(0, Math.min(1, fraction)) * 100)}%`;
  return node;
}

/** Solid: the distinctive share; light: the rest of the share; thin: the second file's share. */
function bars(origin: Origin): HTMLDivElement {
  const container = element("div", "bars");
  const best = origin.best;
  const main = element("div", "track");
  if (best !== undefined) {
    main.append(bar("exact", best.distinctive), bar("fuzzy", best.share - best.distinctive));
  }
  const reverse = element("div", "track thin");
  reverse.append(bar("reverse", origin.second?.share ?? 0));
  container.append(main, reverse);
  return container;
}

/** The row's hover text: the numbers behind it. */
function rowHelp(file: FileOrigin, origin: Origin): string {
  const { best, second } = origin;
  if (best === undefined) {
    return `No file of ${state.other} at ${origin.rev} shares a line with ${file.path}.`;
  }
  const next =
    second === undefined
      ? "No other file shares a line."
      : `Next: ${second.path}, ${percent(second.share)}; the lead is ${percent(origin.gap)}.`;
  return (
    `${percent(best.share)} of ${file.path}'s ${String(file.lines)} lines are in ${best.path} ` +
    `at ${origin.rev} (${String(best.exact)} the same, ${String(best.fuzzy)} edited); ` +
    `${percent(best.distinctive)} are lines in at most two files there. ${next}`
  );
}

function originRow(file: FileOrigin, origin: Origin): HTMLElement {
  const block = element("section", "branch");
  const row = element("div", "row");
  const kind = VERDICTS[verdict(origin)];
  const rev = element("span", "name", origin.rev);
  rev.title = `${origin.rev} in ${state.other}: ${origin.oid}`;
  const lead =
    origin.best === undefined ? "" : `ahead by ${percent(origin.gap)} · ${short(origin.oid)}`;
  row.append(
    element("span", `relation ${kind.className}`, kind.text),
    rev,
    element("span", "message", lead),
    bars(origin),
    element("span", "percent", percent(origin.best?.share)),
    element("span", "percent reverse-percent", percent(origin.second?.share)),
  );
  row.title = rowHelp(file, origin);
  block.append(row);
  const { best, second } = origin;
  if (best !== undefined) {
    row.classList.add("clickable");
    row.dataset["rev"] = origin.rev;
    const detail = element("div", "detail");
    detail.append(
      element("p", "note", `from ${best.path} · ${percent(best.distinctive)} distinctive`),
    );
    if (second !== undefined) {
      detail.append(element("p", "note", `next ${second.path} · ${percent(second.share)}`));
    }
    block.append(detail);
  }
  return block;
}

function render(): HTMLElement[] {
  const header = element("header", "header stacked");
  const heading = element("div", "heading");
  heading.append(
    element("span", "title", `Origin of ${state.path}`),
    element("span", "count", `in ${state.other}`),
  );
  const toolbar = element("div", "toolbar");
  toolbar.append(
    control(
      "other",
      "Other",
      state.other,
      "The repository the file may come from. Click to pick another.",
    ),
    control(
      "refs",
      "Commits",
      state.refs.join(", "),
      `The commits of ${state.other} the file is compared with. Click to pick others.`,
    ),
    info(
      `edited lines: ≥ ${percent(state.threshold)} similar`,
      "A line counts as found if the other file has the same line, or one at least this similar (normalized Levenshtein distance). Whitespace is ignored.",
    ),
  );
  header.append(heading, toolbar);
  const legend = element(
    "p",
    "legend",
    `For each commit of ${state.other}: the file ${state.path}'s lines are most found in, comment ` +
      `lines left out. Bar: how much of the file is there (solid: lines in at most two files ` +
      `there; light: the rest, boilerplate included); thin bar: the next file. A clear lead is ` +
      `${percent(CLEAR_LEAD)} or more over the next file. Click a row to inspect it line by line.`,
  );
  const list = element("div", "rows");
  if (state.error !== undefined) {
    list.append(element("p", "message", state.error));
  } else if (state.file === undefined) {
    list.append(element("p", "message", "comparing…"));
  } else {
    const file = state.file;
    list.append(...file.origins.map((origin) => originRow(file, origin)));
  }
  list.addEventListener("click", (event) => {
    const row =
      event.target instanceof Element ? event.target.closest(".row.clickable") : undefined;
    const rev = row instanceof HTMLElement ? row.dataset["rev"] : undefined;
    if (rev !== undefined) {
      send({ type: "inspect", branch: rev });
    }
  });
  return [header, legend, list, footer(state.meta)];
}

function receive(message: ToOriginPage): void {
  switch (message.type) {
    case "start": {
      state.path = message.path;
      state.other = message.other;
      state.refs = message.refs;
      state.threshold = message.threshold;
      state.meta = message.meta;
      state.file = undefined;
      state.error = undefined;
      break;
    }
    case "result": {
      state.file = message.file;
      break;
    }
    case "failed": {
      state.error = message.message;
      break;
    }
  }
  document.querySelector("#root")?.replaceChildren(...render());
  if (message.type !== "start") {
    send({ type: "rendered", oid: "origin", rows: state.file?.origins.length ?? 0 });
  }
}

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToOriginPage>) => {
  if (event.origin !== origin) {
    return;
  }
  receive(event.data);
});

send({ type: "ready" });
