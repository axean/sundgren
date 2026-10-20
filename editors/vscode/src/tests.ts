// Finding tests in a file's text. A plain scan, so it works without rust-analyzer or the Python
// extension. It only reports tests it is sure about; anything unusual is left out.

export type Language = "python" | "rust";

export interface FoundTest {
  readonly language: Language;
  /** The function name, e.g. `test_divide`. */
  readonly name: string;
  /** For display and pytest node IDs: `TestCalc::test_divide` or `test_divide`. */
  readonly qualifiedName: string;
  /** 0-based line of the `def` or `fn`. */
  readonly line: number;
  /** Leading whitespace of that line. */
  readonly indent: string;
  /** 0-based line where a top-level `import pytest` may go: the start of the enclosing top-level statement. */
  readonly topLevelLine: number;
}

const PYTHON_TEST_FILE = /^(?:test_.*|.*_test)\.py$/u;
const PYTHON_DEF = /^(?<indent>[ \t]*)(?:async[ \t]+)?def[ \t]+(?<name>test\w*)[ \t]*\(/u;
const PYTHON_CLASS = /^(?<indent>[ \t]*)class[ \t]+(?<name>\w+)\b/u;
const RUST_TEST_ATTR = /^[ \t]*#\[(?:\w+::)*test(?:\(.*\))?\][ \t]*(?:\/\/.*)?$/u;
const RUST_ATTR = /^[ \t]*#\[.*\][ \t]*(?:\/\/.*)?$/u;
const RUST_IGNORE_ATTR = /^[ \t]*#\[ignore(?:\s*=.*|\(.*\))?\][ \t]*$/u;
const RUST_FN =
  /^(?<indent>[ \t]*)(?:pub(?:\([^)]*\))?[ \t]+)?(?:async[ \t]+)?(?:unsafe[ \t]+)?fn[ \t]+(?<name>\w+)\b/u;

/** Whether pytest collects tests from a file with this base name. */
export function isPythonTestFile(baseName: string): boolean {
  return PYTHON_TEST_FILE.test(baseName);
}

export function findTests(language: Language, text: string): FoundTest[] {
  const lines = text.split("\n").map((l) => l.replace(/\r$/u, ""));
  return language === "python" ? findPython(lines) : findRust(lines);
}

/** Lines that are inside a triple-quoted string, so a `def test_` there is not code. */
function pythonStringLines(lines: readonly string[]): Set<number> {
  const inside = new Set<number>();
  let open: string | undefined;
  for (const [i, line] of lines.entries()) {
    if (open !== undefined) {
      inside.add(i);
    }
    for (const match of line.matchAll(/"""|'''/gu)) {
      const quote = match[0];
      if (open === undefined) {
        open = quote;
      } else if (open === quote) {
        open = undefined;
      }
    }
  }
  return inside;
}

function findPython(lines: readonly string[]): FoundTest[] {
  const strings = pythonStringLines(lines);
  const found: FoundTest[] = [];
  let testClass: { name: string; indent: string; line: number } | undefined;
  let topLevelStart = 0;
  for (const [i, line] of lines.entries()) {
    if (strings.has(i) || line.trim() === "" || line.trimStart().startsWith("#")) {
      continue;
    }
    const atColumnZero = !/^[ \t]/u.test(line);
    if (atColumnZero && !line.startsWith("@") && !line.startsWith(")")) {
      topLevelStart = firstDecoratorLine(lines, i);
      testClass = undefined;
    }
    const cls = PYTHON_CLASS.exec(line)?.groups;
    if (cls !== undefined && atColumnZero) {
      // pytest collects methods of module-level classes named Test* (without __init__).
      testClass = cls["name"]?.startsWith("Test") === true ? { name: cls["name"], indent: "", line: i } : undefined;
      continue;
    }
    const def = PYTHON_DEF.exec(line)?.groups;
    if (def === undefined) {
      continue;
    }
    const indent = def["indent"] ?? "";
    const name = def["name"] ?? "";
    if (indent === "") {
      found.push({ language: "python", name, qualifiedName: name, line: i, indent, topLevelLine: topLevelStart });
    } else if (testClass !== undefined && isDirectMember(lines, testClass.line, i, indent)) {
      found.push({
        language: "python",
        name,
        qualifiedName: `${testClass.name}::${name}`,
        line: i,
        indent,
        topLevelLine: topLevelStart,
      });
    }
  }
  return found;
}

/** The first line of the decorators directly above a column-0 statement at `line`. */
function firstDecoratorLine(lines: readonly string[], line: number): number {
  let start = line;
  while (start > 0) {
    const previous = lines[start - 1] ?? "";
    const continuesDecorator = previous.startsWith("@") || /^[ \t)\]]/u.test(previous);
    if (previous.trim() === "" || !continuesDecorator) {
      break;
    }
    start -= 1;
  }
  return start;
}

/** Whether the `def` at `line` is a method of the class at `classLine`, not a nested function. */
function isDirectMember(lines: readonly string[], classLine: number, line: number, indent: string): boolean {
  let memberIndent: string | undefined;
  for (let i = classLine + 1; i < line; i += 1) {
    const text = lines[i] ?? "";
    if (text.trim() === "" || text.trimStart().startsWith("#")) {
      continue;
    }
    memberIndent = /^[ \t]*/u.exec(text)?.[0] ?? "";
    break;
  }
  return memberIndent === undefined ? indent.length > 0 : indent === memberIndent;
}

function findRust(lines: readonly string[]): FoundTest[] {
  const found: FoundTest[] = [];
  for (const [i, line] of lines.entries()) {
    if (!RUST_TEST_ATTR.test(line)) {
      continue;
    }
    let ignored = false;
    for (let j = i + 1; j < lines.length; j += 1) {
      const next = lines[j] ?? "";
      if (RUST_IGNORE_ATTR.test(next)) {
        ignored = true;
      }
      if (RUST_ATTR.test(next) || next.trim() === "" || next.trimStart().startsWith("//")) {
        continue;
      }
      const fn = RUST_FN.exec(next)?.groups;
      if (fn !== undefined && !ignored) {
        const name = fn["name"] ?? "";
        found.push({ language: "rust", name, qualifiedName: name, line: j, indent: fn["indent"] ?? "", topLevelLine: j });
      }
      break;
    }
  }
  return found;
}
