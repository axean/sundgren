import * as assert from "node:assert/strict";

import {
  headline,
  isSameFunction,
  lineRanges,
  type LostFunction,
  parseReport,
} from "../../report.js";

function lost(line: number, base: number): unknown {
  return { line, base_line: base, category: "lost", hits: [1, 0] };
}

// A Rust report where plucking test_divide loses divide's body (lines 2-3) and the test's own
// body (patched lines 12-14, base lines 11-13; the `#[ignore]` went in above it).
function rustReport(testName: string): unknown {
  return {
    schema: "sundgren.delta/1",
    summary: {
      lines: { lost: 5 },
      branches: { lost: 1 },
      coverage: { base_pct: 100, patched_pct: 50 },
    },
    files: [
      {
        path: "src/lib.rs",
        lines: [
          lost(2, 2),
          lost(3, 3),
          lost(12, 11),
          lost(13, 12),
          lost(14, 13),
          { line: 4, base_line: 4, category: "covered", hits: [1, 1] },
        ],
        branches: [
          { line: 13, base_line: 12, block: 0, branch: 0, category: "branch_lost", taken: [1, 0] },
        ],
        functions: [
          { name: "mycrate::divide", start: 1, end: 4, status: "went_dark", lines: {} },
          { name: testName, start: 11, end: 15, status: "went_dark", lines: {} },
        ],
      },
    ],
  };
}

describe("isSameFunction", () => {
  it("matches plain, qualified and mangled names", () => {
    assert.ok(isSameFunction("test_divide", "test_divide"));
    assert.ok(isSameFunction("TestCalc.test_divide", "test_divide"));
    assert.ok(isSameFunction("mycrate::tests::test_divide", "test_divide"));
    assert.ok(isSameFunction("_RNvNtCs1a_7mycrate5tests11test_divide", "test_divide"));
  });

  it("doesn't match other functions sharing a prefix or suffix", () => {
    assert.ok(!isSameFunction("test_divide_by_zero", "test_divide"));
    assert.ok(!isSameFunction("mycrate::tests::my_test_divide", "test_divide"));
    assert.ok(!isSameFunction("_RNvCs1a_7mycrate18test_divide_by_zero", "test_divide"));
  });
});

describe("parseReport", () => {
  it("leaves out the plucked test's own lines, branches and function", () => {
    const r = parseReport(rustReport("mycrate::tests::test_divide"), {
      path: "src/lib.rs",
      name: "test_divide",
    });
    assert.ok(r);
    assert.equal(r.linesLost, 2);
    assert.equal(r.branchesLost, 0);
    assert.deepEqual(r.lostLines.get("src/lib.rs"), [2, 3]);
    assert.deepEqual(
      r.functions.map((f) => f.name),
      ["mycrate::divide"],
    );
    assert.equal(r.basePct, undefined);
    assert.equal(r.patchedPct, undefined);
    assert.equal(
      headline("test_divide", r),
      "Plucking test_divide: 2 lines lost, mycrate::divide is no longer run.",
    );
  });

  it("excludes nothing in another file", () => {
    const r = parseReport(rustReport("mycrate::tests::test_divide"), {
      path: "tests/other.rs",
      name: "test_divide",
    });
    assert.ok(r);
    assert.equal(r.linesLost, 5);
    assert.equal(r.branchesLost, 1);
    assert.equal(r.basePct, 100);
    assert.deepEqual(r.lostLines.get("src/lib.rs"), [2, 3, 11, 12, 13]);
  });

  it("excludes nothing without a matching function record", () => {
    const r = parseReport(rustReport("mycrate::tests::test_other"), {
      path: "src/lib.rs",
      name: "test_divide",
    });
    assert.ok(r);
    assert.equal(r.linesLost, 5);
    assert.equal(r.patchedPct, 50);
  });

  it("rejects a malformed function record", () => {
    const raw = rustReport("test_divide") as { files: { functions: unknown[] }[] };
    raw.files[0]?.functions.push(3);
    assert.equal(parseReport(raw), undefined);
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

function dark(name: string): LostFunction {
  return { path: "a.py", name, status: "went_dark", start: 1 };
}

describe("headline", () => {
  const base = {
    linesLost: 4,
    branchesLost: 0,
    basePct: undefined,
    patchedPct: undefined,
    lostLines: new Map<string, readonly number[]>(),
  };

  it("names the functions no longer run", () => {
    assert.equal(
      headline("test_x", { ...base, functions: [dark("f")] }),
      "Plucking test_x: 4 lines lost, f is no longer run.",
    );
    assert.equal(
      headline("test_x", { ...base, functions: [dark("f"), dark("g"), dark("h")] }),
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
