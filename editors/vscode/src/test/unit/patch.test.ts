import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { hasPytestImport, insertionPatch, pluckInsertions, SKIP_REASON } from "../../patch.js";
import { findTests, type Language } from "../../tests.js";

const SKIP = `@pytest.mark.skip(reason="${SKIP_REASON}")`;

function git(cwd: string, ...arguments_: string[]): string {
  return execFileSync("git", arguments_, { cwd, encoding: "utf8", stdio: "pipe" });
}

// Commits `text` as `file` in a new repo, plucks the test named `name`, checks that `git apply`
// accepts the patch, and returns the patch and the patched file.
function pluck(
  language: Language,
  file: string,
  text: string,
  name: string,
): { patch: string; result: string } {
  const test = findTests(language, text).find((candidate) => candidate.name === name);
  assert.ok(test, `no test ${name}`);
  const patch = insertionPatch(file, text, pluckInsertions(test, text));
  const repository = mkdtempSync(path.join(tmpdir(), "sundgren-patch-"));
  try {
    git(repository, "init", "-q");
    mkdirSync(path.dirname(path.join(repository, file)), { recursive: true });
    writeFileSync(path.join(repository, file), text);
    git(repository, "add", file);
    git(repository, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "base");
    writeFileSync(path.join(repository, "pluck.patch"), patch);
    git(repository, "apply", "--check", "pluck.patch");
    git(repository, "apply", "pluck.patch");
    return { patch, result: readFileSync(path.join(repository, file), "utf8") };
  } finally {
    rmSync(repository, { recursive: true, force: true });
  }
}

function hunks(patch: string): number {
  return patch.split("\n").filter((line) => line.startsWith("@@")).length;
}

function text(...lines: string[]): string {
  return lines.map((line) => `${line}\n`).join("");
}

describe("hasPytestImport", () => {
  for (const source of [
    "import pytest",
    "import os, pytest",
    "import pytest  # x",
    "x = 1\nimport pytest, os",
  ]) {
    it(`true for ${JSON.stringify(source)}`, () => {
      assert.equal(hasPytestImport(source), true);
    });
  }
  for (const source of [
    "from pytest import mark",
    "import pytest_mock",
    "import pytest as pt",
    "    import pytest",
    "# import pytest",
  ]) {
    it(`false for ${JSON.stringify(source)}`, () => {
      assert.equal(hasPytestImport(source), false);
    });
  }
});

describe("pluck patch", () => {
  it("ignores a Rust test", () => {
    const source = text(
      "pub fn add(a: i32, b: i32) -> i32 {",
      "    a + b",
      "}",
      "",
      "#[cfg(test)]",
      "mod tests {",
      "    #[test]",
      "    fn adds() {",
      "        assert_eq!(super::add(1, 2), 3);",
      "    }",
      "}",
    );
    const { patch, result } = pluck("rust", "src/lib.rs", source, "adds");
    assert.equal(hunks(patch), 1);
    assert.equal(
      result,
      text(
        "pub fn add(a: i32, b: i32) -> i32 {",
        "    a + b",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    #[test]",
        "    #[ignore]",
        "    fn adds() {",
        "        assert_eq!(super::add(1, 2), 3);",
        "    }",
        "}",
      ),
    );
  });

  it("skips a Python test with one hunk when pytest is imported", () => {
    const source = text(
      "import pytest",
      "",
      "",
      "def helper():",
      "    return 1",
      "",
      "",
      "def test_a():",
      "    assert helper() == 1",
    );
    const { patch, result } = pluck("python", "test_a.py", source, "test_a");
    assert.equal(hunks(patch), 1);
    assert.equal(
      result,
      text(
        "import pytest",
        "",
        "",
        "def helper():",
        "    return 1",
        "",
        "",
        SKIP,
        "def test_a():",
        "    assert helper() == 1",
      ),
    );
  });

  it("imports pytest above a module-level test", () => {
    const source = text("def test_a():", "    assert True");
    const { result } = pluck("python", "test_a.py", source, "test_a");
    assert.equal(result, text("import pytest", SKIP, "def test_a():", "    assert True"));
  });

  it("imports pytest above the decorators of a module-level test", () => {
    const source = text(
      "import os",
      "",
      "",
      "@decorate(",
      "    1,",
      ")",
      "def test_a():",
      "    assert os",
    );
    const { patch, result } = pluck("python", "test_a.py", source, "test_a");
    assert.equal(hunks(patch), 1);
    assert.equal(
      result,
      text(
        "import os",
        "",
        "",
        "import pytest",
        "@decorate(",
        "    1,",
        ")",
        SKIP,
        "def test_a():",
        "    assert os",
      ),
    );
  });

  it("imports pytest above the class of a method, in two hunks when far apart", () => {
    const source = text(
      "class TestCalc:",
      "    def test_one(self):",
      "        a = 1",
      "        b = 2",
      "        c = 3",
      "        d = 4",
      "        e = 5",
      "        assert a + b + c + d + e",
      "",
      "    def test_two(self):",
      "        assert True",
    );
    const { patch, result } = pluck("python", "test_calc.py", source, "test_two");
    assert.equal(hunks(patch), 2);
    assert.equal(
      result,
      text(
        "import pytest",
        "class TestCalc:",
        "    def test_one(self):",
        "        a = 1",
        "        b = 2",
        "        c = 3",
        "        d = 4",
        "        e = 5",
        "        assert a + b + c + d + e",
        "",
        `    ${SKIP}`,
        "    def test_two(self):",
        "        assert True",
      ),
    );
  });

  it("merges the import and the skip into one hunk when close", () => {
    const source = text("class TestCalc:", "", "    def test_one(self):", "        assert True");
    const { patch, result } = pluck("python", "test_calc.py", source, "test_one");
    assert.equal(hunks(patch), 1);
    assert.equal(
      result,
      text(
        "import pytest",
        "class TestCalc:",
        "",
        `    ${SKIP}`,
        "    def test_one(self):",
        "        assert True",
      ),
    );
  });

  it("handles a file without a final newline", () => {
    const source = "import pytest\n\ndef test_a():\n    assert True";
    const { result } = pluck("python", "test_a.py", source, "test_a");
    assert.equal(result, `import pytest\n\n${SKIP}\ndef test_a():\n    assert True`);
  });

  it("handles a Rust test on the last lines without a final newline", () => {
    const source = "#[test]\nfn a() {}";
    const { result } = pluck("rust", "src/lib.rs", source, "a");
    assert.equal(result, "#[test]\n#[ignore]\nfn a() {}");
  });

  it("handles a test at the start of a long file", () => {
    const filler = Array.from({ length: 20 }, (_, index) => `X${String(index)} = ${String(index)}`);
    const source = text("import pytest", "def test_a():", "    assert True", ...filler);
    const { result } = pluck("python", "test_a.py", source, "test_a");
    assert.equal(
      result,
      text("import pytest", SKIP, "def test_a():", "    assert True", ...filler),
    );
  });

  it("handles a test at the end of a long file", () => {
    const filler = Array.from({ length: 20 }, (_, index) => `X${String(index)} = ${String(index)}`);
    const source = text(...filler, "def test_z():", "    assert True");
    const { patch, result } = pluck("python", "test_z.py", source, "test_z");
    assert.equal(hunks(patch), 1);
    assert.equal(
      result,
      text(...filler, "import pytest", SKIP, "def test_z():", "    assert True"),
    );
  });

  it("refuses CR line endings", () => {
    assert.throws(() =>
      insertionPatch("a.py", "def test_a():\r\n    pass\r\n", [{ before: 0, lines: ["x"] }]),
    );
  });
});
