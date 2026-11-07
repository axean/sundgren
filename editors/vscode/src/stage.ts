// Branch stages: where a branch is in its life, for the History map's labels. No VS Code, no DOM.
//
// A stage comes from two facts: whether the lane is merged into the base (by hash), and how the
// branch stands against its upstream. Every pair of the two has exactly one row in STAGES, so a
// branch is always in exactly one stage; the type checker finds a pair missing or written twice.

import type { BranchState } from "./git.js";
import type { Lane } from "./graph.js";

export type Stage =
  "draft" | "loose" | "steady" | "budding" | "lagging" | "tangled" | "remote" | "done";

/** How a branch stands against its upstream, or what kind of remote lane it is. */
export type Tracking =
  "none" | "gone" | "level" | "ahead" | "behind" | "diverged" | "remote-only" | "remote-copy";

type Key = `${"merged" | "open"} ${Tracking}`;

/** A row: the stage, and what this row means, for hover text. */
interface Row {
  readonly stage: Stage;
  readonly help: string;
}

const COPY_HELP = "the server's copy of your branch, with commits your branch doesn't have.";

/** Every pair of merged and tracking, once, with its stage. */
export const STAGES: Readonly<Record<Key, Row>> = {
  "open none": { stage: "draft", help: "Not merged, never pushed." },
  "open gone": {
    stage: "loose",
    help: "Not merged by hash (maybe not merged into the base at all), remote branch deleted. Usually a squash merge.",
  },
  "open level": { stage: "steady", help: "Not merged; the same as its remote branch." },
  "open ahead": { stage: "budding", help: "Not merged; has commits its remote branch doesn't." },
  "open behind": {
    stage: "lagging",
    help: "Not merged; its remote branch has commits it doesn't.",
  },
  "open diverged": {
    stage: "tangled",
    help: "Not merged; it and its remote branch each have commits the other doesn't.",
  },
  "open remote-only": { stage: "remote", help: "Not merged; only on the remote, no local branch." },
  "open remote-copy": { stage: "remote", help: `Not merged; ${COPY_HELP}` },
  "merged none": { stage: "done", help: "Merged into the base; never pushed." },
  "merged gone": { stage: "done", help: "Merged into the base; remote branch deleted." },
  "merged level": { stage: "done", help: "Merged into the base; remote branch still there." },
  "merged ahead": {
    stage: "budding",
    help: "Merged into the base; has commits its remote branch doesn't.",
  },
  "merged behind": {
    stage: "lagging",
    help: "Merged into the base; its remote branch has newer commits that aren't.",
  },
  "merged diverged": {
    stage: "tangled",
    help: "Merged into the base; it and its remote branch each have commits the other doesn't, and the remote's aren't in the base.",
  },
  "merged remote-only": {
    stage: "done",
    help: "Merged into the base; only on the remote, no local branch.",
  },
  "merged remote-copy": { stage: "remote", help: `Merged into the base; ${COPY_HELP}` },
};

/** Each stage's name, mark (an emoji), and what it typically means, for the legend. */
export const STAGE_MARKS: Readonly<
  Record<Stage, { readonly name: string; readonly mark: string; readonly summary: string }>
> = {
  draft: { name: "Draft", mark: "📝", summary: "Not merged, never pushed." },
  loose: {
    name: "Loose",
    mark: "🍂",
    summary: "Not merged, remote branch deleted: usually a squash merge.",
  },
  steady: { name: "Steady", mark: "🌳", summary: "Not merged, the same as its remote branch." },
  budding: { name: "Budding", mark: "🌱", summary: "Has commits its remote branch doesn't." },
  lagging: { name: "Lagging", mark: "🐢", summary: "Its remote branch has commits it doesn't." },
  tangled: {
    name: "Tangled",
    mark: "🧶",
    summary: "It and its remote branch each have commits the other doesn't.",
  },
  remote: {
    name: "Remote",
    mark: "📡",
    summary: "Only on the remote, or the remote's copy of a local branch drawn beside it.",
  },
  done: {
    name: "Done",
    mark: "✅",
    summary: "Merged into the base, and its remote branch is gone or the same.",
  },
};

/** What a lane's Stage or State column shows: an emoji, and its hover text. */
export interface Mark {
  readonly text: string;
  readonly help: string;
}

const BASE: Mark = { text: "🪵", help: "Base: every other lane is placed against it." };
const UNCLEAR: Mark = { text: "❓", help: "Unclear: no branch data for this lane." };

/** A line of the legend: a mark of the Stage or State column, its name, what it means. */
export interface LegendRow {
  readonly text: string;
  readonly name: string;
  readonly summary: string;
}

/** Every mark of the Stage and State columns. */
export const LEGEND: readonly LegendRow[] = [
  ...Object.values(STAGE_MARKS).map(({ name, mark, summary }) => ({ text: mark, name, summary })),
  { text: UNCLEAR.text, name: "Unclear", summary: "No branch data for the lane." },
  { text: BASE.text, name: "Base", summary: "The base: every other lane is placed against it." },
  { text: "📂", name: "Unchanged", summary: "State: checked out, no uncommitted changes." },
  { text: "✏️", name: "Changed", summary: "State: checked out, with uncommitted changes." },
];

/**
How the branch stands against its upstream. A remote lane is a remote-only branch, or the copy of
a local branch drawn beside it. A local lane needs its branch's state; undefined without one.
*/
export function tracking(
  lane: Pick<Lane, "kind" | "only">,
  state: BranchState | undefined,
): Tracking | undefined {
  if (lane.kind === "remote") {
    return lane.only === "remote" ? "remote-only" : "remote-copy";
  }
  if (state === undefined || lane.kind !== "local" || state.isRemote) {
    return undefined;
  }
  if (state.upstream === undefined) {
    return "none";
  }
  if (state.isGone === true) {
    return "gone";
  }
  const isAhead = (state.ahead ?? 0) > 0;
  const isBehind = (state.behind ?? 0) > 0;
  if (isAhead && isBehind) {
    return "diverged";
  }
  if (isAhead) {
    return "ahead";
  }
  return isBehind ? "behind" : "level";
}

/** The lane's stage, as its mark and hover text: ✅ and "Done: Merged into the base; …". */
export function stageMark(
  lane: Pick<Lane, "kind" | "only" | "merged">,
  state: BranchState | undefined,
): Mark {
  if (lane.kind === "reference") {
    return BASE;
  }
  const how = tracking(lane, state);
  if (how === undefined) {
    return UNCLEAR;
  }
  const row = STAGES[`${lane.merged ? "merged" : "open"} ${how}`];
  const { name, mark } = STAGE_MARKS[row.stage];
  return { text: mark, help: `${name}: ${row.help}` };
}

/**
The State column: a work tree the branch is checked out in, unchanged (📂) or with uncommitted
changes (✏️). Undefined when it isn't checked out. This is the branch's state, not its stage.
*/
export function worktreeMark(state: BranchState | undefined): Mark | undefined {
  const worktree = state?.worktree;
  if (worktree === undefined) {
    return undefined;
  }
  const changes = state?.changes;
  if (changes === undefined) {
    return { text: "📂", help: `Checked out in ${worktree}.` };
  }
  return changes.changed + changes.untracked === 0
    ? { text: "📂", help: `Checked out in ${worktree}; no uncommitted changes.` }
    : {
        text: "✏️",
        help: `Checked out in ${worktree}; uncommitted: ${String(changes.changed)} changed, ${String(changes.untracked)} untracked files.`,
      };
}
