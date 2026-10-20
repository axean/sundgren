import * as assert from "node:assert/strict";

import {
  headline,
  isSameFunction,
  lineRanges,
  notSkippedProblem,
  type LostFunction,
  parseReport,
} from "../../report.js";

function lost(line: number, base: number): unknown {
  return { line, base_line: base, category: "lost", hits: [1, 0] };
}

function dark(name: string, start: number, end: number): unknown {
  return { name, start, end, status: "went_dark", lines: {} };
}

// A Rust report (patched-tree lines; the `#[ignore]` went in at line 11). Plucking the test at
// patched line 12 loses `divide` (2-3) and `double` (20-21, with its async block); the test's own
// body is in its closure (13-14), since llvm-cov ends a function where its first closure starts.
function rustReport(testName: string): unknown {
  return {
    schema: "sundgren.delta/1",
    summary: {
      lines: { lost: 6 },
      branches: { lost: 1 },
      coverage: { base_pct: 100, patched_pct: 50 },
    },
    files: [
      {
        path: "src/lib.rs",
        lines: [
          lost(2, 2),
          lost(3, 3),
          lost(13, 12),
          lost(14, 13),
          lost(20, 19),
          lost(21, 20),
          { line: 4, base_line: 4, category: "covered", hits: [1, 1] },
        ],
        branches: [
          { line: 14, base_line: 13, block: 0, branch: 0, category: "branch_lost", taken: [1, 0] },
        ],
        functions: [
          dark("mycrate::divide", 1, 4),
          dark(`mycrate::tests::${testName}`, 12, 12),
          dark(`mycrate::tests::${testName}::{closure#0}`, 13, 15),
          dark("mycrate::double", 20, 20),
          dark("mycrate::double::{async_block#0}", 20, 22),
        ],
      },
    ],
  };
}

describe("isSameFunction", () => {
  it("matches plain and qualified names", () => {
    assert.ok(isSameFunction("test_divide", "test_divide"));
    assert.ok(isSameFunction("TestCalc.test_divide", "test_divide"));
    assert.ok(isSameFunction("mycrate::tests::test_divide", "test_divide"));
  });

  it("doesn't match other functions sharing a prefix or suffix", () => {
    assert.ok(!isSameFunction("test_divide_by_zero", "test_divide"));
    assert.ok(!isSameFunction("mycrate::tests::my_test_divide", "test_divide"));
  });
});

describe("parseReport", () => {
  const exclude = { path: "src/lib.rs", name: "test_divide", line: 12 };

  it("leaves out the plucked test's own lines, branches, function and closures", () => {
    const r = parseReport(rustReport("test_divide"), exclude);
    assert.ok(r);
    assert.equal(r.linesLost, 4);
    assert.equal(r.branchesLost, 0);
    assert.deepEqual(r.lostLines.get("src/lib.rs"), [2, 3, 19, 20]);
    assert.deepEqual(
      r.functions.map((f) => f.name),
      ["mycrate::divide", "mycrate::double"],
    );
    assert.equal(r.basePct, undefined);
    assert.equal(r.patchedPct, undefined);
    assert.equal(
      headline("test_divide", r),
      "Plucking test_divide: 4 lines lost, mycrate::divide and mycrate::double are no longer run.",
    );
  });

  it("matches by location, so a test named like the function it tests keeps that loss", () => {
    const r = parseReport(rustReport("divide"), { ...exclude, name: "divide" });
    assert.ok(r);
    assert.equal(r.linesLost, 4);
    assert.ok(r.functions.some((f) => f.name === "mycrate::divide"));
    assert.ok(r.functions.every((f) => f.name !== "mycrate::tests::divide"));
  });

  it("excludes nothing in another file", () => {
    const r = parseReport(rustReport("test_divide"), { ...exclude, path: "tests/other.rs" });
    assert.ok(r);
    assert.equal(r.linesLost, 6);
    assert.equal(r.branchesLost, 1);
    assert.equal(r.basePct, 100);
    assert.deepEqual(r.lostLines.get("src/lib.rs"), [2, 3, 12, 13, 19, 20]);
  });

  it("excludes nothing when no record of that name holds the test's line", () => {
    const r = parseReport(rustReport("test_divide"), { ...exclude, line: 30 });
    assert.ok(r);
    assert.equal(r.linesLost, 6);
    assert.equal(r.patchedPct, 50);
  });

  it("rejects a malformed function record", () => {
    const raw = rustReport("test_divide") as { files: { functions: unknown[] }[] };
    raw.files[0]?.functions.push(3);
    assert.equal(parseReport(raw), undefined);
  });
});

function withOutcomes(outcomes: [string, string][]): Parameters<typeof notSkippedProblem>[0] {
  const r = parseReport(rustReport("test_divide"));
  assert.ok(r);
  return { ...r, outcomes: new Map(outcomes) };
}

describe("notSkippedProblem", () => {
  it("accepts a test skipped in the patched run, also per parameter", () => {
    const path = "tests/test_calc.py";
    const r = withOutcomes([
      ["tests.test_calc::test_add", "passed"],
      ["tests.test_calc::test_divide", "skipped"],
      ["tests.test_calc.TestCalc::test_split[1]", "skipped"],
      ["tests.test_calc.TestCalc::test_split[2]", "skipped"],
    ]);
    assert.equal(notSkippedProblem(r, path, "test_divide"), undefined);
    assert.equal(notSkippedProblem(r, path, "TestCalc::test_split"), undefined);
  });

  it("refuses a test that still ran, or isn't in the results", () => {
    const path = "tests/test_calc.py";
    const r = withOutcomes([
      ["tests.test_calc::test_divide", "passed"],
      ["tests.test_calc::test_divide_by_zero", "skipped"],
    ]);
    assert.match(notSkippedProblem(r, path, "test_divide") ?? "", /still ran/u);
    assert.match(notSkippedProblem(r, path, "test_add") ?? "", /isn't among the test results/u);
  });
});

describe("lineRanges", () => {
  it("joins consecutive lines into ranges", () => {
    assert.equal(lineRanges([11, 6, 7, 8]), "6–8, 11");
  });

  it("handles one line, duplicates and none", () => {
    assert.equal(lineRanges([3]), "3");
    assert.equal(lineRanges([3, 3, 4]), "3–4");
    assert.equal(lineRanges([]), "");
  });
});

function noLongerRun(name: string): LostFunction {
  return { path: "a.py", name, status: "went_dark", start: 1 };
}

describe("headline", () => {
  const base = {
    linesLost: 4,
    branchesLost: 0,
    basePct: undefined,
    patchedPct: undefined,
    lostLines: new Map<string, readonly number[]>(),
    outcomes: new Map<string, string>(),
  };

  it("names the functions no longer run", () => {
    assert.equal(
      headline("test_x", { ...base, functions: [noLongerRun("f")] }),
      "Plucking test_x: 4 lines lost, f is no longer run.",
    );
    assert.equal(
      headline("test_x", {
        ...base,
        functions: [noLongerRun("f"), noLongerRun("g"), noLongerRun("h")],
      }),
      "Plucking test_x: 4 lines lost, f, g and h are no longer run.",
    );
  });

  it("leaves partial losses out of the sentence", () => {
    const partial = { path: "a.py", name: "f", status: "partial_loss", start: 1 } as const;
    assert.equal(
      headline("test_x", { ...base, functions: [partial] }),
      "Plucking test_x: 4 lines lost.",
    );
  });
});
