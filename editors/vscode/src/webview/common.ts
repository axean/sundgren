// Pieces the containment and inspect pages share. Runs in the webview.

import type { Meta } from "../containment.js";

const REPOSITORY_URL = "https://github.com/axean/sundgren";
const AUTHOR = "Axel E. Andersson";

export function element<K extends keyof HTMLElementTagNameMap>(
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

export function percent(value: number | undefined): string {
  return value === undefined ? "–" : `${String(Math.round(value * 100))}%`;
}

/** A commit id as shown: its first 7 characters. */
export function short(oid: string): string {
  return oid.slice(0, 7);
}

/** `path`'s last part, for a file named again next to its full path. */
export function fileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

/** A note: `text`, explained by `help` on hover. */
export function info(text: string, help: string): HTMLSpanElement {
  const node = element("span", "info");
  node.append(text, element("span", "info-mark", "ⓘ"));
  node.title = help;
  return node;
}

/** The footer: what this is, which versions, where it comes from, and who made it. */
export function footer(meta: Meta | undefined): HTMLElement {
  const node = element("footer", "footer");
  const repository = element("a", "", "github.com/axean/sundgren");
  repository.href = REPOSITORY_URL;
  const parts: (string | HTMLElement)[] = [element("span", "footer-name", "sundgren")];
  if (meta !== undefined) {
    parts.push(`extension ${meta.version} (${meta.commit})`, meta.binary);
  }
  parts.push(repository, `by ${AUTHOR}`);
  for (const [index, part] of parts.entries()) {
    if (index > 0) {
      node.append(element("span", "footer-dot", "·"));
    }
    node.append(part);
  }
  return node;
}

/**
How an item ended up in the other branch: moved (another file), renamed (another name), both,
or undefined if it is the same item in the same file.
*/
export function movement(
  path: string,
  item: string | undefined,
  there: { readonly path: string; readonly item?: string },
): string | undefined {
  const isMoved = there.path !== path;
  const isRenamed =
    item !== undefined &&
    there.item !== undefined &&
    (item.split(/[.:]/u).at(-1) ?? item) !== (there.item.split(/[.:]/u).at(-1) ?? there.item);
  if (isMoved && isRenamed) {
    return "moved, renamed";
  }
  if (isMoved) {
    return "moved";
  }
  return isRenamed ? "renamed" : undefined;
}
