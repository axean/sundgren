import * as assert from "node:assert/strict";

import { findTests, isPythonTestFile } from "../../tests.js";

/** [qualifiedName, line] of each test found. */
function found(language: "python" | "rust", lines: readonly string[]): [string, number][] {
  return findTests(language, lines.join("\n")).map((test) => [test.qualifiedName, test.line]);
}

describe("isPythonTestFile", () => {
  for (const name of ["test_calc.py", "calc_test.py", "test_.py"]) {
    it(`accepts ${name}`, () => {
      assert.equal(isPythonTestFile(name), true);
    });
  }
  for (const name of ["calc.py", "testcalc.py", "test_calc.pyi", "conftest.py", "test_calc.txt"]) {
    it(`rejects ${name}`, () => {
      assert.equal(isPythonTestFile(name), false);
    });
  }
});

describe("findTests: Python", () => {
  it("finds module-level and async tests", () => {
    assert.deepEqual(
      found("python", ["def test_a():", "    pass", "", "async def test_b():", "    pass"]),
      [
        ["test_a", 0],
        ["test_b", 3],
      ],
    );
  });

  it("ignores functions not named test*", () => {
    assert.deepEqual(found("python", ["def helper():", "    pass"]), []);
  });

  it("finds methods of a module-level Test class", () => {
    const tests = findTests(
      "python",
      ["class TestCalc:", "    def test_add(self):", "        pass"].join("\n"),
    );
    assert.equal(tests.length, 1);
    assert.deepEqual(
      { ...tests[0] },
      {
        language: "python",
        name: "test_add",
        qualifiedName: "TestCalc::test_add",
        line: 1,
        indent: " ".repeat(4),
        topLevelLine: 0,
        lensLine: 1,
      },
    );
  });

  it("finds methods after a class docstring and decorated methods", () => {
    assert.deepEqual(
      found("python", [
        "class TestCalc(Base):",
        '    """Doc."""',
        "",
        "    @pytest.mark.slow",
        "    def test_add(self):",
        "        pass",
      ]),
      [["TestCalc::test_add", 4]],
    );
  });

  it("skips a function nested in a method", () => {
    assert.deepEqual(
      found("python", [
        "class TestCalc:",
        "    def test_add(self):",
        "        def test_inner():",
        "            pass",
      ]),
      [["TestCalc::test_add", 1]],
    );
  });

  it("skips a function nested in a module-level test", () => {
    assert.deepEqual(found("python", ["def test_a():", "    def test_inner():", "        pass"]), [
      ["test_a", 0],
    ]);
  });

  it("skips methods of a class not named Test*", () => {
    assert.deepEqual(
      found("python", ["class Helper:", "    def test_add(self):", "        pass"]),
      [],
    );
  });

  it("forgets the Test class at the next top-level statement", () => {
    assert.deepEqual(
      found("python", [
        "class TestCalc:",
        "    pass",
        "",
        "if True:",
        "    def test_hidden():",
        "        pass",
      ]),
      [],
    );
  });

  it("skips a def inside a triple-quoted string", () => {
    assert.deepEqual(
      found("python", [
        'SOURCE = """',
        "def test_fake():",
        "    pass",
        '"""',
        "",
        "TEMPLATE = '''",
        "def test_fake2():",
        "'''",
        "",
        "def test_real():",
        "    pass",
      ]),
      [["test_real", 9]],
    );
  });

  it("puts topLevelLine at the first line of a multi-line decorator", () => {
    const tests = findTests(
      "python",
      [
        "import os",
        "",
        "@pytest.mark.parametrize(",
        '    "x",',
        "    [1, 2],",
        ")",
        "def test_x(x):",
        "    pass",
      ].join("\n"),
    );
    assert.deepEqual(
      tests.map((test) => [test.line, test.topLevelLine]),
      [[6, 2]],
    );
  });

  it("puts topLevelLine of a method at its class's decorators", () => {
    const tests = findTests(
      "python",
      ["", "@decorate", "class TestCalc:", "    def test_add(self):", "        pass"].join("\n"),
    );
    assert.deepEqual(
      tests.map((test) => [test.line, test.topLevelLine]),
      [[3, 1]],
    );
  });

  it("handles CRLF line endings", () => {
    assert.deepEqual(findTests("python", "def test_a():\r\n    pass\r\n").length, 1);
  });
});

function rust(...lines: string[]): [string, number][] {
  return found("rust", lines);
}

describe("findTests: Rust", () => {
  it("finds #[test] and #[tokio::test] functions", () => {
    assert.deepEqual(
      rust(
        "#[test]",
        "fn a() {}",
        "",
        "#[tokio::test]",
        "async fn b() {}",
        "",
        "    #[test]",
        "    pub fn c() {}",
        "#[test]",
        "pub(crate) fn d() {}",
      ),
      [
        ["a", 1],
        ["b", 4],
        ["c", 7],
        ["d", 9],
      ],
    );
  });

  it("reports the indent of the fn line", () => {
    const tests = findTests(
      "rust",
      ["mod tests {", "    #[test]", "    fn a() {}", "}"].join("\n"),
    );
    assert.deepEqual(
      tests.map((test) => [test.line, test.indent, test.topLevelLine]),
      [[2, " ".repeat(4), 2]],
    );
  });

  it("skips attributes and comments between #[test] and the fn", () => {
    assert.deepEqual(
      rust(
        "#[test] // the main case",
        "#[should_panic]",
        "// a comment",
        '#[cfg_attr(miri, ignore = "slow")]',
        "fn a() {}",
      ),
      [["a", 4]],
    );
  });

  it("skips tests already #[ignore]d after #[test]", () => {
    assert.deepEqual(rust("#[test]", "#[ignore]", "fn a() {}"), []);
    assert.deepEqual(rust("#[test]", '#[ignore = "slow"]', "fn a() {}"), []);
  });

  it("skips tests #[ignore]d before #[test]", () => {
    assert.deepEqual(rust("#[ignore]", "#[test]", "fn a() {}"), []);
  });

  it("skips an #[ignore] with a trailing comment", () => {
    assert.deepEqual(rust("#[test]", "#[ignore] // flaky", "fn a() {}"), []);
  });

  it("skips a commented-out #[test]", () => {
    assert.deepEqual(rust("// #[test]", "fn a() {}"), []);
  });

  it("skips #[test] not followed by a fn", () => {
    assert.deepEqual(rust("#[test]", "mod m {}", "fn later() {}"), []);
  });

  it("ignores other attributes on their own", () => {
    assert.deepEqual(rust("#[inline]", "fn a() {}"), []);
  });
});

/** [line, lensLine] of each test found. */
function lensLines(language: "python" | "rust", lines: readonly string[]): [number, number][] {
  return findTests(language, lines.join("\n")).map((test) => [test.line, test.lensLine]);
}

describe("findTests: lens line", () => {
  it("is the def line without decorators", () => {
    assert.deepEqual(lensLines("python", ["def test_a():", "    pass"]), [[0, 0]]);
  });

  it("is the first line of a multi-line decorator block", () => {
    assert.deepEqual(
      lensLines("python", [
        "x = 1",
        "@pytest.mark.slow",
        "@pytest.mark.parametrize(",
        '    "x",',
        "    [1, 2],",
        ")",
        "def test_x(x):",
        "    pass",
      ]),
      [[6, 1]],
    );
  });

  it("is a method's own first decorator, not the class's or a previous method's", () => {
    assert.deepEqual(
      lensLines("python", [
        "@decorate",
        "class TestCalc:",
        "    @pytest.mark.slow",
        "    def test_a(self):",
        "        pass",
        "    def test_b(self):",
        "        pass",
        "",
        "    @pytest.mark.parametrize(",
        "        'x', [1]",
        "    )",
        "    def test_c(self, x):",
        "        pass",
      ]),
      [
        [3, 2],
        [5, 5],
        [11, 8],
      ],
    );
  });

  it("doesn't reach past a blank line or another function's body", () => {
    assert.deepEqual(
      lensLines("python", [
        "@decorate",
        "def helper():",
        "    return 1",
        "def test_a():",
        "    pass",
        "@decorate",
        "",
        "def test_b():",
        "    pass",
      ]),
      [
        [3, 3],
        [7, 7],
      ],
    );
  });

  it("is the first attribute or doc comment line in Rust", () => {
    assert.deepEqual(
      lensLines("rust", [
        "mod tests {",
        "    /// Doubles.",
        "    #[test]",
        "    #[should_panic]",
        "    fn a() {}",
        "",
        "    // not a doc comment",
        '    #[cfg_attr(miri, ignore = "slow")]',
        "    #[tokio::test]",
        "    async fn b() {}",
        "}",
      ]),
      [
        [4, 1],
        [9, 7],
      ],
    );
  });
});
