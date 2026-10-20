// The git facts the preconditions need. Plain `git` subprocesses with a timeout; no VS Code.

import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { join, relative, sep } from "node:path";

const TIMEOUT_MS = 10_000;

export interface GitResult {
  readonly code: number;
  readonly stdout: string;
  readonly stderr: string;
}

export async function git(cwd: string, args: readonly string[]): Promise<GitResult> {
  return new Promise((resolve) => {
    execFile(
      "git",
      [...args],
      { cwd, timeout: TIMEOUT_MS, maxBuffer: 64 * 1024 * 1024, env: { ...process.env, LC_ALL: "C" } },
      (error, stdout, stderr) => {
        const code = error === null ? 0 : typeof error.code === "number" ? error.code : 1;
        resolve({ code, stdout, stderr });
      },
    );
  });
}

/** The repository's top-level directory, or undefined if `dir` is not in a work tree. */
export async function repoRoot(dir: string): Promise<string | undefined> {
  const r = await git(dir, ["rev-parse", "--show-toplevel"]);
  return r.code === 0 ? r.stdout.trim() : undefined;
}

/** The full sha of HEAD, or undefined in a repository without commits. */
export async function headSha(repo: string): Promise<string | undefined> {
  const r = await git(repo, ["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]);
  return r.code === 0 ? r.stdout.trim() : undefined;
}

/** A merge, rebase, cherry-pick, revert or bisect in progress, if any. */
export async function operationInProgress(repo: string): Promise<string | undefined> {
  const r = await git(repo, ["rev-parse", "--git-dir"]);
  if (r.code !== 0) {
    return undefined;
  }
  const gitDir = join(repo, r.stdout.trim());
  const markers: readonly (readonly [string, string])[] = [
    ["MERGE_HEAD", "a merge"],
    ["rebase-merge", "a rebase"],
    ["rebase-apply", "a rebase"],
    ["CHERRY_PICK_HEAD", "a cherry-pick"],
    ["REVERT_HEAD", "a revert"],
    ["BISECT_LOG", "a bisect"],
  ];
  return markers.find(([file]) => existsSync(join(gitDir, file)))?.[1];
}

/** Tracked files with uncommitted changes (staged or not), like sundgren's `dirty_tree`. */
export async function changedTrackedFiles(repo: string): Promise<string[] | undefined> {
  const r = await git(repo, ["status", "--porcelain", "--untracked-files=no", "-z"]);
  if (r.code !== 0) {
    return undefined;
  }
  return r.stdout
    .split("\0")
    .filter((entry) => entry.length > 3)
    .map((entry) => entry.slice(3));
}

/** The path of `file` relative to `repo`, with `/` separators, if it lies inside it. */
export function repoPath(repo: string, file: string): string | undefined {
  const rel = relative(repo, file);
  if (rel === "" || rel.startsWith("..") || rel.startsWith(sep)) {
    return undefined;
  }
  return rel.split(sep).join("/");
}

/** Whether `path` (repo-relative) is tracked. */
export async function isTracked(repo: string, path: string): Promise<boolean> {
  const r = await git(repo, ["ls-files", "--error-unmatch", "--", path]);
  return r.code === 0;
}

/** The text of `path` at HEAD, or undefined if it isn't in HEAD or isn't valid UTF-8. */
export async function headText(repo: string, path: string): Promise<string | undefined> {
  const r = await git(repo, ["show", `HEAD:${path}`]);
  return r.code === 0 ? r.stdout : undefined;
}
