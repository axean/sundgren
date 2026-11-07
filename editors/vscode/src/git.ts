// What the extension asks git directly: which repository, which branches. No VS Code.

import { execFile } from "node:child_process";

import { findOnPath } from "./sundgren.js";
import {
  parseBranchStates,
  parseChanges,
  parseNumstats,
  parseRefNames,
  STATE_FORMAT,
  type BranchState,
  type CommitStat,
} from "./git-output.js";

export * from "./git-output.js";

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

/** The checked-out submodules of `repository`, nested ones too, as absolute paths. */
export async function submodules(repository: string): Promise<string[]> {
  const r = await git(repository, ["submodule", "foreach", "--quiet", "--recursive", "pwd"]);
  return r.code === 0 ? r.stdout.split("\n").filter((line) => line !== "") : [];
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

/** Tags, newest first. */
export async function tags(repository: string): Promise<string[]> {
  const r = await git(repository, [
    "for-each-ref",
    "--sort=-creatordate",
    "--format=%(refname:short)",
    "refs/tags",
  ]);
  return r.code === 0 ? r.stdout.split("\n").filter((name) => name !== "") : [];
}

/** The text of `file` at commit `oid` (`git cat-file`), or undefined if it isn't a file there. */
export async function fileAt(
  repository: string,
  oid: string,
  file: string,
): Promise<string | undefined> {
  const r = await git(repository, ["cat-file", "blob", `${oid}:${file}`]);
  return r.code === 0 ? r.stdout : undefined;
}

/**
The default base for containment: the branch origin/HEAD points at (`origin/main`), else the
first of main, master, origin/main and origin/master that exists.
*/
export function chooseBase(
  originHead: string | undefined,
  names: readonly string[],
): string | undefined {
  const head = originHead ?? "";
  return head === ""
    ? ["main", "master", "origin/main", "origin/master"].find((name) => names.includes(name))
    : head;
}

/** The repository's default branch, as `chooseBase` picks it; undefined if it has none. */
export async function defaultBranch(repository: string): Promise<string | undefined> {
  const head = await git(repository, [
    "symbolic-ref",
    "--quiet",
    "--short",
    "refs/remotes/origin/HEAD",
  ]);
  const found = await branches(repository);
  return chooseBase(
    head.code === 0 ? head.stdout.trim() : undefined,
    found.map((branch) => branch.name),
  );
}

/** Where a repository's commits open on the web: `prefix` + a commit id. */
export interface CommitLinks {
  /** The web host, e.g. `github.com`. */
  readonly host: string;
  readonly prefix: string;
}

const SSH_SCHEMES = new Set(["ssh", "git+ssh", "ssh+git", "git"]);

/** `host` from `user@host`. */
function withoutUser(authority: string): string {
  return authority.slice(authority.lastIndexOf("@") + 1);
}

/** The web scheme, host and path of a remote URL; undefined for a local path or an unknown scheme. */
function webLocation(remote: string): { scheme: string; host: string; path: string } | undefined {
  const separator = remote.indexOf("://");
  if (separator === -1) {
    // scp-like: [user@]host:path
    const colon = remote.indexOf(":");
    const slash = remote.indexOf("/");
    if (colon <= 0 || (slash !== -1 && slash < colon)) {
      return undefined;
    }
    return {
      scheme: "https",
      host: withoutUser(remote.slice(0, colon)),
      path: remote.slice(colon + 1),
    };
  }
  const scheme = remote.slice(0, separator).toLowerCase();
  const rest = remote.slice(separator + 3);
  const slash = rest.indexOf("/");
  if (slash === -1) {
    return undefined;
  }
  const host = withoutUser(rest.slice(0, slash));
  const path = rest.slice(slash + 1);
  if (scheme === "https" || scheme === "http") {
    return { scheme, host, path };
  }
  // The ssh port isn't the web one.
  const port = host.lastIndexOf(":");
  return SSH_SCHEMES.has(scheme)
    ? { scheme: "https", host: port === -1 ? host : host.slice(0, port), path }
    : undefined;
}

/** `path` without slashes at either end, and without `.git`. */
function repositoryPath(path: string): string {
  let trimmed = path;
  while (trimmed.startsWith("/")) {
    trimmed = trimmed.slice(1);
  }
  while (trimmed.endsWith("/")) {
    trimmed = trimmed.slice(0, -1);
  }
  return trimmed.endsWith(".git") ? trimmed.slice(0, -".git".length) : trimmed;
}

/**
Where commits of the repository at `remote` (an ssh, scp-like or https URL) open: GitHub's
`/commit/` on github.com, GitLab's `/-/commit/` on any other host.
*/
export function commitLinks(remote: string): CommitLinks | undefined {
  const where = webLocation(remote.trim());
  if (where === undefined) {
    return undefined;
  }
  const path = repositoryPath(where.path);
  const host = where.host.toLowerCase();
  if (path === "" || host === "") {
    return undefined;
  }
  return host === "github.com" || host === "ssh.github.com"
    ? { host: "github.com", prefix: `https://github.com/${path}/commit/` }
    : { host, prefix: `${where.scheme}://${host}/${path}/-/commit/` };
}

/** The URL of `origin`, else of the remote `base` tracks; undefined if neither is set. */
export async function remoteUrl(repository: string, base: string): Promise<string | undefined> {
  const origin = await git(repository, ["remote", "get-url", "origin"]);
  if (origin.code === 0) {
    return origin.stdout.trim();
  }
  const remote = await git(repository, ["config", "--get", `branch.${base}.remote`]);
  const name = remote.stdout.trim();
  if (name === "" || name === "." || remote.code !== 0) {
    return undefined;
  }
  const url = await git(repository, ["remote", "get-url", "--", name]);
  return url.code === 0 ? url.stdout.trim() : undefined;
}

/** Every branch's state, with the uncommitted changes of the work trees branches are in. */
export async function branchStates(repository: string): Promise<BranchState[]> {
  const r = await git(repository, ["for-each-ref", STATE_FORMAT, "refs/heads", "refs/remotes"]);
  if (r.code !== 0) {
    return [];
  }
  return Promise.all(
    parseBranchStates(r.stdout).map(async (branch) => {
      if (branch.worktree === undefined) {
        return branch;
      }
      const status = await git(branch.worktree, ["status", "--porcelain"]);
      return status.code === 0 ? { ...branch, changes: parseChanges(status.stdout) } : branch;
    }),
  );
}

/** The stats of `oids`, read by up to `parallel` git processes at once. */
export async function commitStats(
  repository: string,
  oids: readonly string[],
  parallel: number,
): Promise<Map<string, CommitStat>> {
  const size = Math.ceil(oids.length / Math.max(1, parallel));
  const chunks: string[][] = [];
  for (let start = 0; start < oids.length; start += size) {
    chunks.push(oids.slice(start, start + size));
  }
  const results = await Promise.all(
    chunks.map((chunk) =>
      git(repository, [
        "log",
        "--no-walk=unsorted",
        "--format=%x1e%H",
        "--raw",
        "--numstat",
        "--diff-merges=first-parent",
        ...chunk,
      ]),
    ),
  );
  return new Map(
    results.flatMap((result) => (result.code === 0 ? [...parseNumstats(result.stdout)] : [])),
  );
}

/**
What the history map is drawn from: every ref and where it points, and HEAD. Commit ids cover
their whole history, so the map can only change when this does. Undefined if git fails.
*/
export async function refsFingerprint(repository: string): Promise<string | undefined> {
  const [refs, head] = await Promise.all([
    git(repository, ["for-each-ref", "--format=%(objectname) %(refname) %(symref)"]),
    // Its commit, and the branch it is on (or HEAD when detached).
    git(repository, ["rev-parse", "HEAD", "--symbolic-full-name", "HEAD"]),
  ]);
  return refs.code === 0 && head.code === 0 ? `${head.stdout}${refs.stdout}` : undefined;
}

/** The branches and remote branches merged into `base` by hash: their tips are in its history. */
export async function mergedInto(repository: string, base: string): Promise<string[]> {
  const r = await git(repository, [
    "for-each-ref",
    "--format=%(refname)",
    `--merged=${base}`,
    "refs/heads",
    "refs/remotes",
  ]);
  return r.code === 0 ? parseRefNames(r.stdout) : [];
}
