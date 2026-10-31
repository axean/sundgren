import * as assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { locateSundgren, runSundgren } from "../../sundgren.js";

function executable(file: string): string {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, '#!/bin/sh\necho "$@"\n');
  chmodSync(file, 0o755);
  return file;
}

describe("locateSundgren", () => {
  const root = mkdtempSync(path.join(tmpdir(), "sundgren-locate-"));
  const setting = executable(path.join(root, "setting", "sundgren"));
  const bundled = executable(path.join(root, "bundled", "sundgren"));
  const repository = path.join(root, "repo");
  const venv = executable(path.join(repository, ".venv", "bin", "sundgren"));
  const onPath = executable(path.join(root, "bin", "sundgren"));
  const missing = path.join(root, "missing", "sundgren");

  it("prefers the setting, then bundled, then .venv, then PATH", () => {
    const options = { setting, bundled, repository, pathEnvironment: path.dirname(onPath) };
    assert.deepEqual(locateSundgren(options), { path: setting, source: "setting" });
    assert.deepEqual(locateSundgren({ ...options, setting: "" }), {
      path: bundled,
      source: "bundled",
    });
    assert.deepEqual(locateSundgren({ ...options, setting: "", bundled: missing }), {
      path: venv,
      source: "venv",
    });
    assert.deepEqual(
      locateSundgren({ ...options, setting: "", bundled: missing, repository: root }),
      { path: onPath, source: "path" },
    );
    assert.equal(
      locateSundgren({ setting: "", bundled: missing, repository: root, pathEnvironment: "" }),
      undefined,
    );
  });

  it("reports a setting that isn't an executable", () => {
    const found = locateSundgren({ setting: missing, bundled, repository, pathEnvironment: "" });
    assert.ok(found !== undefined && "error" in found);
  });
});

describe("runSundgren", () => {
  it("returns the output and exit code", async () => {
    const directory = mkdtempSync(path.join(tmpdir(), "sundgren-run-"));
    const binary = executable(path.join(directory, "s"));
    const result = await runSundgren(binary, ["graph", "main"], tmpdir()).done;
    assert.equal(result.code, 0);
    assert.equal(result.stdout, "graph main\n");
    assert.equal(result.isCancelled, false);
  });
});
