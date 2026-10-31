// What the extension asks git directly: which repository, which branches. No VS Code.

import { execFile } from "node:child_process";

import { findOnPath } from "./sundgren.js";

const TIMEOUT_MS = 10_000;

export interface GitResult {
  readonly code: number;
  readonly stdout: string;
  readonly stderr: string;
}

/** Runs the first `git` on PATH, as VS Code's own git support finds it. */
export async function git(cwd: string, gitArguments: readonly string[]): Promise<GitResult> {
  const executable = findOnPath("git", process.env["PATH"]);
  if (executable === undefined) {
    return { code: 127, stdout: "", stderr: "git is not on PATH" };
  }
  return new Promise((resolve) => {
    execFile(
      executable,
      [...gitArguments],
      {
        cwd,
        timeout: TIMEOUT_MS,
        maxBuffer: 64 * 1024 * 1024,
        env: { ...process.env, LC_ALL: "C" },
      },
      (error, stdout, stderr) => {
        let code = 0;
        if (error !== null) {
          code = typeof error.code === "number" ? error.code : 1;
        }
        resolve({ code, stdout, stderr });
      },
    );
  });
}

/** The repository's top-level directory, or undefined if `directory` is not in a work tree. */
export async function repositoryRoot(directory: string): Promise<string | undefined> {
  const r = await git(directory, ["rev-parse", "--show-toplevel"]);
  return r.code === 0 ? r.stdout.trim() : undefined;
}

export interface Branch {
  /** Short name: `main`, `origin/main`. */
  readonly name: string;
  readonly isRemote: boolean;
  readonly isCurrent: boolean;
}

/** Branches from `git for-each-ref --format=%(refname)%00%(HEAD)` output. */
export function parseBranches(output: string): Branch[] {
  const branches: Branch[] = [];
  for (const line of output.split("\n")) {
    const [ref = "", head = ""] = line.split("\0", 2);
    const local = ref.startsWith("refs/heads/") ? ref.slice("refs/heads/".length) : undefined;
    const remote = ref.startsWith("refs/remotes/") ? ref.slice("refs/remotes/".length) : undefined;
    const name = local ?? remote;
    // origin/HEAD is an alias, not a branch.
    if (name === undefined || remote?.endsWith("/HEAD") === true) {
      continue;
    }
    branches.push({ name, isRemote: remote !== undefined, isCurrent: head === "*" });
  }
  // The current branch, then local, then remote branches; each in git's order.
  const rank = (b: Branch): number => {
    if (b.isCurrent) {
      return 0;
    }
    return b.isRemote ? 2 : 1;
  };
  return branches.toSorted((a, b) => rank(a) - rank(b));
}

export async function branches(repository: string): Promise<Branch[]> {
  const r = await git(repository, [
    "for-each-ref",
    "--format=%(refname)%00%(HEAD)",
    "refs/heads",
    "refs/remotes",
  ]);
  return r.code === 0 ? parseBranches(r.stdout) : [];
}
