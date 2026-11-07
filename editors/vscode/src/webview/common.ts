// Pieces the containment and inspect pages share. Runs in the webview.

import type { Meta, Side } from "../containment.js";
import { STAGE_LEGEND, STATE_LEGEND, type LegendRow, type Mark } from "../stage.js";

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

/**
A branch's work in short: `+25 · 2c · d72e29f`, lines added, commits, and the commit of the base
it left, marked as such; the sentence on hover.
*/
export function work(side: Side, branch: string, base: string): HTMLSpanElement {
  const node = element("span", "work");
  const fork = element("span", "fork", short(side.fork));
  node.append(`+${String(side.lines)} · ${String(side.commits)}c · `, fork);
  const commits = side.commits === 1 ? "1 commit" : `${String(side.commits)} commits`;
  node.title =
    `${branch} added ${String(side.lines)} lines in ${commits} since it left ${base} at ` +
    side.fork +
    (side.comment_lines > 0 ? ` (and ${String(side.comment_lines)} comment lines).` : ".");
  return node;
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

/** A toolbar button: `label` and the current `value`, with a caret; `onClick` changes it. */
export function controlButton(
  label: string,
  value: string,
  help: string,
  onClick: () => void,
): HTMLButtonElement {
  const button = element("button", "control");
  button.type = "button";
  button.title = help;
  const shown = element("span", "control-value", value);
  shown.title = value;
  button.append(element("span", "control-label", label), shown);
  button.append(element("span", "control-caret", "▾"));
  button.addEventListener("click", onClick);
  return button;
}

/** The Repository control, if the page was told its repository: the folder's name, on a click another. */
export function repositoryControl(
  repository: string | undefined,
  onClick: () => void,
): HTMLButtonElement[] {
  if (repository === undefined) {
    return [];
  }
  const name = repository.split(/[\\/]/u).findLast((part) => part !== "") ?? repository;
  const help = `${repository}\nClick to pick another: an open folder's repository, one of its submodules, or the active file's.`;
  return [controlButton("Repository", name, help, onClick)];
}

/** A small badge: `text`, coloured by `tone`, explained by `help`. */
export function badge(tone: string, text: string, help: string): HTMLSpanElement {
  const node = element("span", `badge ${tone}`, text);
  node.title = help;
  return node;
}

/** A symbol, coloured by its tone, with its hover text. */
export function markSpan(mark: Mark): HTMLSpanElement {
  const node = element("span", `mark-${mark.tone}`, mark.text);
  node.title = mark.help;
  return node;
}

/** A legend section: its title, what the column says, then each symbol, its name and meaning. */
function legendSection(title: string, intro: string, rows: readonly LegendRow[]): HTMLElement {
  const section = element("div", "legend-section");
  const table = element("table", "legend");
  for (const row of rows) {
    const symbol = element("td", "legend-mark");
    symbol.append(markSpan({ text: row.text, help: row.name, tone: row.tone }));
    const line = element("tr", "");
    line.append(
      symbol,
      element("td", "legend-name", row.name),
      element("td", "legend-summary", row.summary),
    );
    table.append(line);
  }
  section.append(element("p", "legend-title", title), element("p", "help-intro", intro), table);
  return section;
}

/** A circled ? that shows, on hover or click, what each Stage and State symbol means. */
export function legendHelp(): HTMLSpanElement {
  const cell = element("span", "column-help");
  const mark = element("span", "help-mark", "?");
  mark.tabIndex = 0;
  mark.setAttribute("aria-label", "What the Stage and State marks mean");
  const card = element("div", "help-card");
  card.append(
    legendSection(
      "Stage",
      "Where the branch is in its life, from whether it is merged into the base and how it stands against its remote branch. Hover a symbol in a label for its exact case.",
      STAGE_LEGEND,
    ),
    legendSection(
      "State",
      "The facts the stage is made from, always in three slots: merged into the base, the remote branch, the work tree.",
      STATE_LEGEND,
    ),
  );
  mark.append(card);
  cell.append(mark);
  return cell;
}
