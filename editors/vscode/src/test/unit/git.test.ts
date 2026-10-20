import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { changedTrackedFiles, porcelainPaths } from "../../git.js";

function git(cwd: string, ...arguments_: string[]): void {
  execFileSync("git", arguments_, { cwd, stdio: "pipe" });
}

describe("porcelainPaths", () => {
  it("reads modified files and skips a rename's old path", () => {
    assert.deepEqual(porcelainPaths(" M a.py\0R  new.py\0old.py\0 M b.py\0"), [
      "a.py",
      "new.py",
      "b.py",
    ]);
  });

  it("lists the new path of a staged rename in a real repository", async () => {
    const directory = mkdtempSync(path.join(tmpdir(), "sundgren-git-"));
    try {
      git(directory, "init", "-q");
      writeFileSync(path.join(directory, "old_name.py"), "x = 1\n");
      git(directory, "add", ".");
      git(directory, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init");
      git(directory, "mv", "old_name.py", "new_name.py");
      assert.deepEqual(await changedTrackedFiles(directory), ["new_name.py"]);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});
