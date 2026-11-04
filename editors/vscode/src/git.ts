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

/** Uncommitted changes in a work tree. */
export interface Changes {
  /** Tracked files changed, staged or not. */
  readonly changed: number;
  readonly untracked: number;
}

/** Where a branch lives: local, with its upstream if it has one, or only on a remote. */
export interface BranchState {
  readonly name: string;
  readonly isRemote: boolean;
  /** The upstream of a local branch, e.g. `origin/main`. */
  readonly upstream?: string;
  /** Commits the branch has that the upstream doesn't, and the other way round. */
  readonly ahead?: number;
  readonly behind?: number;
  /** The upstream is configured but no longer exists. */
  readonly isGone?: boolean;
  /** The work tree the branch is checked out in, if any. */
  readonly worktree?: string;
  /** That work tree's uncommitted changes. */
  readonly changes?: Changes;
}

const STATE_FORMAT =
  "--format=%(refname)%00%(upstream:short)%00%(upstream:track,nobracket)%00%(worktreepath)";

/** `ahead 2, behind 1`, `ahead 2`, `gone` or nothing, from `%(upstream:track,nobracket)`. */
function parseTrack(track: string): Pick<BranchState, "ahead" | "behind" | "isGone"> {
  if (track === "gone") {
    return { isGone: true };
  }
  const ahead = /ahead (\d+)/u.exec(track)?.[1];
  const behind = /behind (\d+)/u.exec(track)?.[1];
  return {
    ahead: ahead === undefined ? 0 : Number(ahead),
    behind: behind === undefined ? 0 : Number(behind),
  };
}

/**
Branches from `git for-each-ref` with STATE_FORMAT over refs/heads and refs/remotes: local ones
with their upstream and work tree, then remote ones that are no local branch's upstream.
*/
export function parseBranchStates(output: string): BranchState[] {
  const local: BranchState[] = [];
  const remote: string[] = [];
  for (const line of output.split("\n")) {
    const [ref = "", upstream = "", track = "", worktree = ""] = line.split("\0", 4);
    if (ref.startsWith("refs/heads/")) {
      local.push({
        name: ref.slice("refs/heads/".length),
        isRemote: false,
        ...(upstream !== "" && { upstream, ...parseTrack(track) }),
        ...(worktree !== "" && { worktree }),
      });
    } else if (ref.startsWith("refs/remotes/") && !ref.endsWith("/HEAD")) {
      remote.push(ref.slice("refs/remotes/".length));
    }
  }
  const tracked = new Set(local.map((branch) => branch.upstream));
  return [
    ...local,
    ...remote
      .filter((name) => !tracked.has(name))
      .map((name) => ({ name, isRemote: true }) satisfies BranchState),
  ];
}

/** Counts from `git status --porcelain`: `??` lines are untracked, the rest changed. */
export function parseChanges(output: string): Changes {
  const lines = output.split("\n").filter((line) => line !== "");
  const untracked = lines.filter((line) => line.startsWith("??")).length;
  return { changed: lines.length - untracked, untracked };
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

/** A commit's change in short; a merge's is against its first parent, as GitLab shows it. */
export interface CommitStat {
  readonly files: number;
  readonly added: number;
  readonly removed: number;
}

/** Stats from `git log --format=%x1e%H --numstat`; binary files count as files, not lines. */
export function parseNumstats(output: string): Map<string, CommitStat> {
  const stats = new Map<string, CommitStat>();
  for (const record of output.split("\u{1E}").slice(1)) {
    const [oid = "", ...lines] = record.split("\n");
    let files = 0;
    let added = 0;
    let removed = 0;
    for (const line of lines) {
      const match = /^(\d+|-)\t(\d+|-)\t/u.exec(line);
      if (match === null) {
        continue;
      }
      files += 1;
      added += match[1] === "-" ? 0 : Number(match[1]);
      removed += match[2] === "-" ? 0 : Number(match[2]);
    }
    stats.set(oid.trim(), { files, added, removed });
  }
  return stats;
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
