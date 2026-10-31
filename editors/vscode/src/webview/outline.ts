// The outline page: a file as sundgren reads it. Its items as bands beside the lines, nested
// ones further right, and its tokens coloured by kind. Runs in the webview; bundled to
// dist/outline.js.

import type { FromPage } from "../messages.js";
import {
  flatten,
  itemKinds,
  segmentLines,
  tokenKinds,
  type FileOutline,
  type Language,
  type Placed,
  type ToOutlinePage,
  type TokenKind,
} from "../outline.js";

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

const vscode = acquireVsCodeApi();

function send(message: FromPage): void {
  vscode.postMessage(message);
}

/** Item kinds by colour: the brand's data palette. */
const KIND_GROUPS: readonly { readonly label: string; readonly kinds: readonly string[] }[] = [
  { label: "function", kinds: ["function"] },
  { label: "class, struct, enum, union", kinds: ["class", "struct", "enum", "union"] },
  { label: "impl, trait", kinds: ["impl", "trait"] },
  { label: "module", kinds: ["module"] },
  { label: "macro", kinds: ["macro"] },
  { label: "const, static, type", kinds: ["const", "static", "type"] },
];

function kindClass(kind: string): string {
  const group = KIND_GROUPS.findIndex((g) => g.kinds.includes(kind));
  return `k${String(Math.max(group, 0))}`;
}

const TOKEN_LABELS: ReadonlyMap<TokenKind, string> = new Map([["directive", "preprocessor"]]);

/** Token kinds shown in the text colour. */
const PLAIN: ReadonlySet<TokenKind> = new Set(["ident", "punct"]);

/** Band width plus gap, in px; nested items sit one step further right. */
const STEP = 10;

function element<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

/** Lines `start..=end` as CSS top and height. */
function span(node: HTMLElement, start: number, end: number): void {
  node.style.top = `calc(var(--lh) * ${String(start - 1)})`;
  node.style.height = `calc(var(--lh) * ${String(end - start + 1)})`;
}

function header(path: string, file: FileOutline | undefined, items: number): HTMLElement {
  const node = element("div", "header");
  node.append(element("span", "title", path.split(/[/\\]/u).at(-1) ?? path));
  const details = [path];
  if (file?.language !== undefined) {
    details.push(
      file.language,
      `${String(items)} items`,
      `${String(file.tokens?.length ?? 0)} tokens`,
    );
  }
  node.append(element("span", "detail", details.join(" · ")));
  return node;
}

/** The legend for `language`: only the item and token kinds it can have. */
function legend(language: Language): HTMLElement {
  const node = element("div", "legend");
  const kinds = itemKinds(language);
  for (const [index, group] of KIND_GROUPS.entries()) {
    const present = group.kinds.filter((kind) => kinds.includes(kind));
    if (present.length === 0) {
      continue;
    }
    const chip = element("span", "chip");
    chip.append(element("span", `swatch k${String(index)}`), present.join(", "));
    node.append(chip);
  }
  const tokens = tokenKinds(language);
  for (const kind of tokens) {
    node.append(element("span", `chip t-${kind}`, TOKEN_LABELS.get(kind) ?? kind));
  }
  return node;
}

/** The lines of code, each with its number, and the item bands and labels over them. */
function code(source: string, file: FileOutline, placed: readonly Placed[]): HTMLElement {
  const depth = Math.max(0, ...placed.map((p) => p.depth + 1));
  const node = element("div", "code");
  node.style.setProperty("--gutter", `${String(depth * STEP + 6)}px`);

  const labels = new Map<number, Placed[]>();
  for (const p of placed) {
    labels.set(p.item.start, [...(labels.get(p.item.start) ?? []), p]);
  }
  const lines = segmentLines(source, file.tokens ?? []);
  for (const [index, segments] of lines.entries()) {
    const number = index + 1;
    const line = element("div", "line");
    line.append(element("span", "number", String(number)));
    const text = element("span", "text");
    for (const segment of segments) {
      const kind = segment.kind;
      text.append(
        kind === undefined || PLAIN.has(kind)
          ? document.createTextNode(segment.text)
          : element("span", `t-${kind}`, segment.text),
      );
    }
    const starting = labels.get(number) ?? [];
    for (const p of starting) {
      const label = element(
        "span",
        `label ${kindClass(p.item.kind)}`,
        `${p.item.kind} ${p.item.name} · ${String(p.item.start)}–${String(p.item.end)}`,
      );
      label.dataset["item"] = String(placed.indexOf(p));
      text.append(label);
    }
    line.append(text);
    node.append(line);
  }

  const highlight = element("div", "highlight");
  node.append(highlight);
  for (const [index, p] of placed.entries()) {
    const band = element("div", `band ${kindClass(p.item.kind)}`);
    band.style.left = `${String(p.depth * STEP + 2)}px`;
    span(band, p.item.start, p.item.end);
    band.title = `${p.item.kind} ${p.item.name}, lines ${String(p.item.start)}–${String(p.item.end)}`;
    band.dataset["item"] = String(index);
    node.append(band);
  }

  const itemAt = (target: EventTarget | null): Placed | undefined => {
    const index = target instanceof HTMLElement ? target.dataset["item"] : undefined;
    return index === undefined ? undefined : placed[Number(index)];
  };
  node.addEventListener("mouseover", (event) => {
    const p = itemAt(event.target);
    highlight.className = p === undefined ? "highlight" : `highlight on ${kindClass(p.item.kind)}`;
    if (p !== undefined) {
      span(highlight, p.item.start, p.item.end);
    }
  });
  node.addEventListener("click", (event) => {
    const p = itemAt(event.target);
    if (p !== undefined) {
      send({ type: "reveal", line: p.item.start });
    }
  });
  return node;
}

function receive(message: ToOutlinePage): void {
  const root = document.querySelector("#root");
  if (root === null) {
    return;
  }
  if (message.type === "error") {
    root.replaceChildren(
      header(message.path, undefined, 0),
      element("p", "message", message.message),
    );
    send({ type: "rendered", oid: message.path, rows: 0 });
    return;
  }
  const placed = flatten(message.file.items);
  const top = header(message.path, message.file, placed.length);
  if (message.file.language === undefined) {
    root.replaceChildren(
      top,
      element("p", "message", "sundgren doesn't read this language: only Python, Rust and C."),
    );
  } else {
    root.replaceChildren(
      top,
      legend(message.file.language),
      code(message.source, message.file, placed),
    );
  }
  send({ type: "rendered", oid: message.path, rows: placed.length });
}

// VS Code's webview host posts the extension's messages with this frame's own origin.
addEventListener("message", (event: MessageEvent<ToOutlinePage>) => {
  if (event.origin !== origin) {
    return;
  }
  receive(event.data);
});

send({ type: "ready" });
