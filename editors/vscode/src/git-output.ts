// What git's output says, parsed: branch states, work tree changes, commit stats, ref names. No
// VS Code and no Node, so the docs demos parse the git output recorded with them the same way.

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

export const STATE_FORMAT =
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

/** A commit's change in short; a merge's is against its first parent, as GitLab shows it. */
export interface CommitStat {
  readonly files: number;
  readonly added: number;
  readonly removed: number;
  /** The files by kind of change; renames and type changes count as modified, copies as added. */
  readonly filesAdded: number;
  readonly filesModified: number;
  readonly filesDeleted: number;
}

type FileKindKey = "filesAdded" | "filesModified" | "filesDeleted";

/** A `--raw` line's kind of change, from its status letter. */
function rawKind(line: string): FileKindKey | undefined {
  const status = /^:\S+ \S+ \S+ \S+ ([A-Z])/u.exec(line)?.[1];
  if (status === undefined) {
    return undefined;
  }
  if (status === "A" || status === "C") {
    return "filesAdded";
  }
  return status === "D" ? "filesDeleted" : "filesModified";
}

/**
Stats from `git log --format=%x1e%H --raw --numstat`: the kinds from the raw lines' status
letters, the lines from numstat; binary files count as files, not lines.
*/
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
    const kinds = lines.map((line) => rawKind(line));
    const count = (kind: FileKindKey): number => kinds.filter((each) => each === kind).length;
    stats.set(oid.trim(), {
      files,
      added,
      removed,
      filesAdded: count("filesAdded"),
      filesModified: count("filesModified"),
      filesDeleted: count("filesDeleted"),
    });
  }
  return stats;
}

/**
Branches and remote branches from `git for-each-ref --format=%(refname)` output, by short name
(`main`, `origin/main`); remote HEADs left out.
*/
export function parseRefNames(output: string): string[] {
  const names: string[] = [];
  for (const ref of output.split("\n")) {
    if (ref.startsWith("refs/heads/")) {
      names.push(ref.slice("refs/heads/".length));
    } else if (ref.startsWith("refs/remotes/") && !ref.endsWith("/HEAD")) {
      names.push(ref.slice("refs/remotes/".length));
    }
  }
  return names;
}
