// Finding tests in a file's text. A plain scan, so it works without rust-analyzer or the Python
// extension. It only reports tests it is sure about; anything unusual is left out.

export type Language = "python" | "rust";

export interface FoundTest {
  readonly language: Language;
  /** The function name, e.g. `test_divide`. */
  readonly name: string;
  /** For display: `TestCalc::test_divide` or `test_divide`. */
  readonly qualifiedName: string;
  /** 0-based line of the `def` or `fn`. */
  readonly line: number;
  /** Leading whitespace of that line. */
  readonly indent: string;
  /** 0-based line where a module-level `import pytest` may go: the start of the enclosing top-level statement. */
  readonly topLevelLine: number;
}

/* eslint-disable security/detect-unsafe-regex -- anchored, and only ever applied to one source line */
const PYTHON_TEST_FILE = /^(?:test_.*|.*_test)\.py$/u;
const PYTHON_DEF = /^(?<indent>[ \t]*)(?:async[ \t]+)?def[ \t]+(?<name>test\w*)[ \t]*\(/u;
const PYTHON_CLASS = /^class[ \t]+(?<name>\w+)\b/u;
const RUST_TEST_ATTR = /^[ \t]*#\[(?:\w+::)*test(?:\(.*\))?\][ \t]*(?:\/\/.*)?$/u;
const RUST_ATTR = /^[ \t]*#\[.*\][ \t]*(?:\/\/.*)?$/u;
const RUST_IGNORE_ATTR = /^[ \t]*#\[ignore\b.*\][ \t]*$/u;
const RUST_PUB = /^pub(?:\([^)]*\))?[ \t]+/u;
const RUST_FN = /^(?:(?:async|unsafe|const)[ \t]+)*fn[ \t]+(?<name>\w+)\b/u;
/* eslint-enable security/detect-unsafe-regex */

/** Whether pytest collects tests from a file with this base name. */
export function isPythonTestFile(baseName: string): boolean {
  return PYTHON_TEST_FILE.test(baseName);
}

export function findTests(language: Language, text: string): FoundTest[] {
  const lines = text.split("\n").map((l) => l.replace(/\r$/u, ""));
  return language === "python" ? findPython(lines) : findRust(lines);
}

/** Lines inside a triple-quoted string, so a `def test_` there isn't code. */
function pythonStringLines(lines: readonly string[]): Set<number> {
  const inside = new Set<number>();
  let open: string | undefined;
  for (const [index, line] of lines.entries()) {
    if (open !== undefined) {
      inside.add(index);
    }
    for (const match of line.matchAll(/"""|'''/gu)) {
      if (open === undefined) {
        open = match[0];
      } else if (open === match[0]) {
        open = undefined;
      }
    }
  }
  return inside;
}

function isCode(line: string): boolean {
  const trimmed = line.trim();
  return trimmed !== "" && !trimmed.startsWith("#");
}

function findPython(lines: readonly string[]): FoundTest[] {
  const strings = pythonStringLines(lines);
  const found: FoundTest[] = [];
  let testClass: { name: string; line: number } | undefined;
  let topLevelStart = 0;
  for (const [index, line] of lines.entries()) {
    if (strings.has(index) || !isCode(line)) {
      continue;
    }
    if (isTopLevelStatement(line)) {
      topLevelStart = firstDecoratorLine(lines, index);
      testClass = testClassAt(line, index);
      if (testClass !== undefined) {
        continue;
      }
    }
    const test = pythonTest(lines, index, testClass, topLevelStart);
    if (test !== undefined) {
      found.push(test);
    }
  }
  return found;
}

function isTopLevelStatement(line: string): boolean {
  return !/^[ \t)]/u.test(line) && !line.startsWith("@");
}

/** A module-level class pytest collects methods from (named Test*). */
function testClassAt(line: string, index: number): { name: string; line: number } | undefined {
  const name = PYTHON_CLASS.exec(line)?.groups?.["name"];
  return name?.startsWith("Test") === true ? { name, line: index } : undefined;
}

function pythonTest(
  lines: readonly string[],
  index: number,
  testClass: { name: string; line: number } | undefined,
  topLevelLine: number,
): FoundTest | undefined {
  const groups = PYTHON_DEF.exec(lines.at(index) ?? "")?.groups;
  const name = groups?.["name"];
  if (name === undefined) {
    return undefined;
  }
  const indent = groups?.["indent"] ?? "";
  const base = { language: "python", name, line: index, indent, topLevelLine } as const;
  if (indent === "") {
    return { ...base, qualifiedName: name };
  }
  return testClass !== undefined && isDirectMember(lines, testClass.line, index, indent)
    ? { ...base, qualifiedName: `${testClass.name}::${name}` }
    : undefined;
}

/** The first line of the decorators directly above a column-0 statement at `line`. */
function firstDecoratorLine(lines: readonly string[], line: number): number {
  let start = line;
  while (start > 0) {
    const previous = lines.at(start - 1) ?? "";
    const isDecoratorPart = previous.startsWith("@") || /^[ \t)\]]/u.test(previous);
    if (!isDecoratorPart || previous.trim() === "") {
      break;
    }
    start -= 1;
  }
  return start;
}

/** Whether the `def` at `line` is a method of the class at `classLine`, not a nested function. */
function isDirectMember(
  lines: readonly string[],
  classLine: number,
  line: number,
  indent: string,
): boolean {
  const firstMember = lines.slice(classLine + 1, line).find((text) => isCode(text));
  const memberIndent =
    firstMember === undefined ? indent : (/^[ \t]*/u.exec(firstMember)?.[0] ?? "");
  return indent === memberIndent;
}

function findRust(lines: readonly string[]): FoundTest[] {
  const found: FoundTest[] = [];
  for (const [index, line] of lines.entries()) {
    if (!RUST_TEST_ATTR.test(line)) {
      continue;
    }
    const test = rustTestAfter(lines, index + 1);
    if (test !== undefined) {
      found.push(test);
    }
  }
  return found;
}

/** The test function the attributes starting at `start` belong to, unless it's `#[ignore]`d. */
function rustTestAfter(lines: readonly string[], start: number): FoundTest | undefined {
  for (let index = start; index < lines.length; index += 1) {
    const line = lines.at(index) ?? "";
    if (RUST_IGNORE_ATTR.test(line)) {
      return undefined;
    }
    if (RUST_ATTR.test(line) || !isCode(line) || line.trimStart().startsWith("//")) {
      continue;
    }
    const indent = line.slice(0, line.length - line.trimStart().length);
    const name = RUST_FN.exec(line.trimStart().replace(RUST_PUB, ""))?.groups?.["name"];
    if (name === undefined) {
      return undefined;
    }
    return {
      language: "rust",
      name,
      qualifiedName: name,
      line: index,
      indent,
      topLevelLine: index,
    };
  }
  return undefined;
}
