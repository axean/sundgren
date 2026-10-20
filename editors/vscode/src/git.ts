// The git facts the preconditions need. Plain `git` subprocesses with a timeout; no VS Code.
/* eslint-disable security/detect-non-literal-fs-filename -- paths come from the open workspace */

import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";

const TIMEOUT_MS = 10_000;

export interface GitResult {
  readonly code: number;
  readonly stdout: string;
  readonly stderr: string;
}

function exitCode(error: (Error & { code?: unknown }) | null): number {
  if (error === null) {
    return 0;
  }
  return typeof error.code === "number" ? error.code : 1;
}

export async function git(cwd: string, gitArguments: readonly string[]): Promise<GitResult> {
  return new Promise((resolve) => {
    execFile(
      // eslint-disable-next-line sonarjs/no-os-command-from-path -- the user's own git, as VS Code uses it
      "git",
      [...gitArguments],
      {
        cwd,
        timeout: TIMEOUT_MS,
        maxBuffer: 64 * 1024 * 1024,
        env: { ...process.env, LC_ALL: "C" },
      },
      (error, stdout, stderr) => {
        resolve({ code: exitCode(error), stdout, stderr });
      },
    );
  });
}

/** The repository's top-level directory, or undefined if `directory` is not in a work tree. */
export async function repositoryRoot(directory: string): Promise<string | undefined> {
  const r = await git(directory, ["rev-parse", "--show-toplevel"]);
  return r.code === 0 ? r.stdout.trim() : undefined;
}

/** The full sha of HEAD, or undefined in a repository without commits. */
export async function headSha(repository: string): Promise<string | undefined> {
  const r = await git(repository, ["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]);
  return r.code === 0 ? r.stdout.trim() : undefined;
}

const OPERATIONS: readonly (readonly [string, string])[] = [
  ["MERGE_HEAD", "a merge"],
  ["rebase-merge", "a rebase"],
  ["rebase-apply", "a rebase"],
  ["CHERRY_PICK_HEAD", "a cherry-pick"],
  ["REVERT_HEAD", "a revert"],
  ["BISECT_LOG", "a bisect"],
];

/** A merge, rebase, cherry-pick, revert or bisect in progress, if any. */
export async function operationInProgress(repository: string): Promise<string | undefined> {
  const r = await git(repository, ["rev-parse", "--absolute-git-dir"]);
  if (r.code !== 0) {
    return undefined;
  }
  const gitDirectory = r.stdout.trim();
  return OPERATIONS.find(([file]) => existsSync(path.join(gitDirectory, file)))?.[1];
}

/** Tracked files with uncommitted changes (staged or not), like sundgren's `dirty_tree`. */
export async function changedTrackedFiles(repository: string): Promise<string[] | undefined> {
  const r = await git(repository, ["status", "--porcelain", "--untracked-files=no", "-z"]);
  return r.code === 0
    ? r.stdout
        .split("\0")
        .filter((entry) => entry.length > 3)
        .map((entry) => entry.slice(3))
    : undefined;
}

/** The path of `file` relative to `repository`, with `/` separators, if it lies inside it. */
export function repositoryPath(repository: string, file: string): string | undefined {
  const relativePath = path.relative(repository, file);
  return relativePath === "" || relativePath.startsWith("..") || path.isAbsolute(relativePath)
    ? undefined
    : relativePath.split(path.sep).join("/");
}

/** Whether `file` (repository-relative) is tracked. */
export async function isTracked(repository: string, file: string): Promise<boolean> {
  const r = await git(repository, ["ls-files", "--error-unmatch", "--", file]);
  return r.code === 0;
}

/** The text of `file` at HEAD, or undefined if it isn't in HEAD. */
export async function headText(repository: string, file: string): Promise<string | undefined> {
  const r = await git(repository, ["show", `HEAD:${file}`]);
  return r.code === 0 ? r.stdout : undefined;
}
