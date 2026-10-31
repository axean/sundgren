// The inspect page's messages and grouping: one branch's lines next to where they were found in
// the target. Shared by the extension and the webview; no VS Code, no DOM.

import type { Comments, Containment, LineFound, Meta } from "./containment.js";

/** From the extension to the inspect page. */
export type ToInspectPage =
  | {
      readonly type: "inspect";
      readonly branch: string;
      readonly target: string;
      readonly base: string;
      readonly comments: Comments;
      readonly threshold: number;
      /** `sundgren contain --lines branch target`. */
      readonly result: Containment;
      /** Where to scroll to: a file, and a line in it. */
      readonly focus?: { readonly path: string; readonly line?: number };
      readonly meta?: Meta;
    }
  | { readonly type: "error"; readonly branch: string; readonly message: string };

/** Lines of one item (or outside items: no `item`), in file order. */
export interface ItemGroup {
  readonly item: string | undefined;
  readonly lines: readonly LineFound[];
}

/** One file's lines, by item in order of first line. */
export interface FileGroup {
  readonly path: string;
  readonly items: readonly ItemGroup[];
}

/** `lines` (in file order, as `--lines` gives them) by file, then by item. */
export function groupLines(lines: readonly LineFound[]): FileGroup[] {
  const files: { path: string; items: Map<string, LineFound[]> }[] = [];
  for (const line of lines) {
    let file = files.at(-1);
    if (file?.path !== line.path) {
      file = { path: line.path, items: new Map() };
      files.push(file);
    }
    // "" for lines outside items: item names are never empty.
    const key = line.item ?? "";
    const group = file.items.get(key);
    if (group === undefined) {
      file.items.set(key, [line]);
    } else {
      group.push(line);
    }
  }
  return files.map((file) => ({
    path: file.path,
    items: [...file.items].map(([key, grouped]) => ({
      item: key === "" ? undefined : key,
      lines: grouped,
    })),
  }));
}

/** The text of each line of `lines`, by `path:line`: to show the line a match points at. */
export function textAt(lines: readonly LineFound[]): Map<string, string> {
  return new Map(lines.map((line) => [`${line.path}:${String(line.line)}`, line.text]));
}
