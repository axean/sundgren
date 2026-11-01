// Pieces the containment and inspect pages share. Runs in the webview.

import type { Meta, Side } from "../containment.js";
import type { BranchState } from "../git.js";

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

/** A small badge: `text`, coloured by `tone`, explained by `help`. */
export function badge(tone: string, text: string, help: string): HTMLSpanElement {
  const node = element("span", `badge ${tone}`, text);
  node.title = help;
  return node;
}

/** The sync badge: remote only, local only, upstream gone, in sync, or ahead and behind. */
export function syncBadge(branch: string, where: BranchState): HTMLSpanElement {
  if (where.isRemote) {
    return badge("muted", "remote", `Only on the remote: there is no local ${branch}.`);
  }
  if (where.upstream === undefined) {
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
  const arrows = [ahead > 0 ? `↑${String(ahead)}` : "", behind > 0 ? `↓${String(behind)}` : ""];
  return badge(
    "warn",
    arrows.filter((arrow) => arrow !== "").join(" "),
    `${String(ahead)} commits not on ${where.upstream}; ${String(behind)} on ${where.upstream} not here.`,
  );
}

/** The work tree badges: checked out somewhere, and whether there are uncommitted changes. */
export function worktreeBadges(where: BranchState): HTMLSpanElement[] {
  if (where.worktree === undefined) {
    return [];
  }
  const badges = [badge("muted", "worktree", `Checked out in ${where.worktree}`)];
  const changes = where.changes;
  if (changes !== undefined) {
    const total = changes.changed + changes.untracked;
    badges.push(
      total === 0
        ? badge("good dot", "●", "No uncommitted changes in its work tree.")
        : badge(
            "bad dot",
            "●",
            `Uncommitted: ${String(changes.changed)} changed, ${String(changes.untracked)} untracked files. Not part of the comparison: only commits are compared.`,
          ),
    );
  }
  return badges;
}
