// The sundgren.outline/0 document (`sundgren outline --tokens`) and the outline page's messages.
// Shared by the extension and the webview; no VS Code, no DOM.

export type Language = "python" | "rust" | "c";
export type TokenKind = "ident" | "number" | "string" | "comment" | "directive" | "punct";

export interface Item {
  readonly kind: string;
  readonly name: string;
  /** First and last line, 1-based, inclusive. */
  readonly start: number;
  readonly end: number;
  readonly children?: readonly Item[];
}

export interface Token {
  readonly kind: TokenKind;
  readonly line: number;
  readonly end_line: number;
  readonly text: string;
}

export interface FileOutline {
  readonly path: string;
  /** Left out if sundgren doesn't read the file's language. */
  readonly language?: Language;
  readonly items: readonly Item[];
  readonly tokens?: readonly Token[];
}

export interface Outline {
  readonly schema: "sundgren.outline/0";
  readonly files: readonly FileOutline[];
}

/** From the extension to the outline page. */
export type ToOutlinePage =
  | {
      readonly type: "outline";
      readonly path: string;
      readonly source: string;
      readonly file: FileOutline;
    }
  | { readonly type: "error"; readonly path: string; readonly message: string };

/** The item kinds sundgren reports in `language` (see src/syntax/ at the repo root). */
export function itemKinds(language: Language): readonly string[] {
  switch (language) {
    case "python": {
      return ["function", "class"];
    }
    case "rust": {
      return [
        "function",
        "struct",
        "enum",
        "union",
        "trait",
        "impl",
        "module",
        "const",
        "static",
        "type",
        "macro",
      ];
    }
    case "c": {
      return ["function", "struct", "union", "enum", "macro"];
    }
  }
}

/** The token kinds worth a colour in `language`: not identifiers and punctuation. */
export function tokenKinds(language: Language): readonly TokenKind[] {
  return language === "c"
    ? ["comment", "string", "number", "directive"]
    : ["comment", "string", "number"];
}

const LANGUAGES = new Set<string>(["python", "rust", "c"]);
const KINDS = new Set<string>(["ident", "number", "string", "comment", "directive", "punct"]);

type Fields = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is Fields {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isLine(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 1;
}

function isItem(value: unknown): value is Item {
  return (
    isObject(value) &&
    typeof value["kind"] === "string" &&
    typeof value["name"] === "string" &&
    isLine(value["start"]) &&
    isLine(value["end"]) &&
    (value["children"] === undefined ||
      (Array.isArray(value["children"]) && value["children"].every((child) => isItem(child))))
  );
}

function isToken(value: unknown): value is Token {
  return (
    isObject(value) &&
    typeof value["kind"] === "string" &&
    KINDS.has(value["kind"]) &&
    isLine(value["line"]) &&
    isLine(value["end_line"]) &&
    typeof value["text"] === "string"
  );
}

function isFile(value: unknown): value is FileOutline {
  return (
    isObject(value) &&
    typeof value["path"] === "string" &&
    (value["language"] === undefined ||
      (typeof value["language"] === "string" && LANGUAGES.has(value["language"]))) &&
    Array.isArray(value["items"]) &&
    value["items"].every((item) => isItem(item)) &&
    (value["tokens"] === undefined ||
      (Array.isArray(value["tokens"]) && value["tokens"].every((token) => isToken(token))))
  );
}

/** `value` as an outline document, or undefined if it isn't one. */
export function parseOutline(value: unknown): Outline | undefined {
  return isObject(value) &&
    value["schema"] === "sundgren.outline/0" &&
    Array.isArray(value["files"]) &&
    value["files"].every((file) => isFile(file))
    ? (value as unknown as Outline)
    : undefined;
}

/** An item with how deep it is nested: 0 for top-level items. */
export interface Placed {
  readonly item: Item;
  readonly depth: number;
}

/** Every item, parents before their children. */
export function flatten(items: readonly Item[], depth = 0): Placed[] {
  return items.flatMap((item) => [{ item, depth }, ...flatten(item.children ?? [], depth + 1)]);
}

/** A stretch of one line: plain text between tokens, or (part of) a token of `kind`. */
export interface Segment {
  readonly text: string;
  readonly kind: TokenKind | undefined;
}

/** The source as tokens and the text between them, in order. */
function pieces(source: string, tokens: readonly Token[]): Segment[] {
  const out: Segment[] = [];
  let position = 0;
  for (const token of tokens) {
    const at = token.text === "" ? -1 : source.indexOf(token.text, position);
    if (at === -1) {
      continue;
    }
    if (at > position) {
      out.push({ text: source.slice(position, at), kind: undefined });
    }
    out.push({ text: token.text, kind: token.kind });
    position = at + token.text.length;
  }
  if (position < source.length) {
    out.push({ text: source.slice(position), kind: undefined });
  }
  return out;
}

/**
The source split into lines, each a list of segments: the tokens, in order, and the text between
them. A token over several lines is split at the line ends. Tokens are found by their text, from
where the last one ended; one that can't be found is skipped.
*/
export function segmentLines(source: string, tokens: readonly Token[]): Segment[][] {
  const lines: Segment[][] = [[]];
  for (const piece of pieces(source, tokens)) {
    for (const [index, part] of piece.text.split("\n").entries()) {
      if (index > 0) {
        lines.push([]);
      }
      if (part !== "") {
        lines.at(-1)?.push({ text: part, kind: piece.kind });
      }
    }
  }
  // A final newline doesn't start another line.
  if (source.endsWith("\n") && lines.length > 1 && lines.at(-1)?.length === 0) {
    lines.pop();
  }
  return lines;
}
