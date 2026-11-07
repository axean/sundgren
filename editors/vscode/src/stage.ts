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

/** How a mark is coloured: by how much it asks for attention. */
export type Tone = "muted" | "text" | "good" | "warn" | "bad";

/** What a lane's Stage column, or one slot of its State column, shows: a symbol and its hover. */
export interface Mark {
  readonly text: string;
  readonly help: string;
  readonly tone: Tone;
}

/** Each stage's name, symbol, colour, and what it typically means, for the legend. */
export const STAGE_MARKS: Readonly<
  Record<
    Stage,
    { readonly name: string; readonly mark: string; readonly tone: Tone; readonly summary: string }
  >
> = {
  draft: { name: "Draft", mark: "○", tone: "muted", summary: "Not merged, never pushed." },
  loose: {
    name: "Loose",
    mark: "⊘",
    tone: "warn",
    summary: "Not merged, remote branch deleted: usually a squash merge.",
  },
  steady: {
    name: "Steady",
    mark: "═",
    tone: "text",
    summary: "Not merged, the same as its remote branch.",
  },
  budding: {
    name: "Budding",
    mark: "▲",
    tone: "warn",
    summary: "Has commits its remote branch doesn't.",
  },
  lagging: {
    name: "Lagging",
    mark: "▼",
    tone: "warn",
    summary: "Its remote branch has commits it doesn't.",
  },
  tangled: {
    name: "Tangled",
    mark: "⋈",
    tone: "bad",
    summary: "It and its remote branch each have commits the other doesn't.",
  },
  remote: {
    name: "Remote",
    mark: "◇",
    tone: "muted",
    summary: "Only on the remote, or the remote's copy of a local branch drawn beside it.",
  },
  done: {
    name: "Done",
    mark: "◆",
    tone: "good",
    summary: "Merged into the base, and its remote branch is gone or the same.",
  },
};

const BASE: Mark = {
  text: "◉",
  help: "Base: every other lane is placed against it.",
  tone: "muted",
};
const UNCLEAR: Mark = { text: "?", help: "Unclear: no branch data for this lane.", tone: "muted" };

/** The State column's first slot: merged into the base or not. */
const MERGED: Readonly<Record<"merged" | "open" | "base", Mark>> = {
  merged: { text: "∈", help: "Merged into the base.", tone: "muted" },
  open: { text: "∉", help: "Not merged into the base.", tone: "muted" },
  base: { text: "·", help: "The base itself.", tone: "muted" },
};

/** The State column's second slot: the remote branch, by how the branch tracks it. */
const REMOTE: Readonly<Record<Tracking, Mark>> = {
  none: { text: "∅", help: "Never pushed: no remote branch.", tone: "muted" },
  gone: { text: "×", help: "Remote branch deleted.", tone: "muted" },
  level: { text: "=", help: "The same as its remote branch.", tone: "muted" },
  ahead: { text: "+", help: "Has commits its remote branch doesn't.", tone: "muted" },
  behind: { text: "−", help: "Its remote branch has commits it doesn't.", tone: "muted" },
  diverged: {
    text: "±",
    help: "It and its remote branch each have commits the other doesn't.",
    tone: "muted",
  },
  "remote-only": { text: "∄", help: "Only on the remote: no local branch.", tone: "muted" },
  "remote-copy": { text: "≈", help: "The server's copy of a local branch.", tone: "muted" },
};
const NO_REMOTE_DATA: Mark = { text: "·", help: "No branch data.", tone: "muted" };
/** REMOTE by tracking: the record keeps every tracking covered, the map looks them up. */
const REMOTE_MARKS = new Map(Object.entries(REMOTE) as [Tracking, Mark][]);

/** The State column's third slot: checked out in a work tree, and whether it has changes. */
const WORKTREE = {
  none: { text: "·", help: "Not checked out.", tone: "muted" },
  unchanged: { text: "□", help: "", tone: "muted" },
  changed: { text: "■", help: "", tone: "warn" },
} as const satisfies Record<string, Mark>;

/** A line of the legend: a symbol, its name, what it means, and its colour. */
export interface LegendRow {
  readonly text: string;
  readonly name: string;
  readonly summary: string;
  readonly tone: Tone;
}

/** The Stage column's symbols. */
export const STAGE_LEGEND: readonly LegendRow[] = [
  ...Object.values(STAGE_MARKS).map(({ name, mark, tone, summary }) => ({
    text: mark,
    name,
    summary,
    tone,
  })),
  { text: UNCLEAR.text, name: "Unclear", summary: "No branch data for the lane.", tone: "muted" },
  {
    text: BASE.text,
    name: "Base",
    summary: "Every other lane is placed against it.",
    tone: "muted",
  },
];

/** The State column's symbols, slot by slot: merged, remote branch, work tree. */
export const STATE_LEGEND: readonly LegendRow[] = [
  { ...MERGED.merged, name: "Merged", summary: MERGED.merged.help },
  { ...MERGED.open, name: "Not merged", summary: MERGED.open.help },
  { ...REMOTE.none, name: "Never pushed", summary: "No remote branch." },
  { ...REMOTE.gone, name: "Deleted", summary: "Its remote branch was deleted." },
  { ...REMOTE.level, name: "Same", summary: REMOTE.level.help },
  { ...REMOTE.ahead, name: "Ahead", summary: REMOTE.ahead.help },
  { ...REMOTE.behind, name: "Behind", summary: REMOTE.behind.help },
  { ...REMOTE.diverged, name: "Both", summary: REMOTE.diverged.help },
  { ...REMOTE["remote-only"], name: "No local", summary: REMOTE["remote-only"].help },
  { ...REMOTE["remote-copy"], name: "Copy", summary: REMOTE["remote-copy"].help },
  { ...WORKTREE.none, name: "Not checked out", summary: "Not checked out in any work tree." },
  { ...WORKTREE.unchanged, name: "Unchanged", summary: "Checked out, no uncommitted changes." },
  { ...WORKTREE.changed, name: "Changed", summary: "Checked out, with uncommitted changes." },
];

/**
How the branch stands against its upstream. A remote lane is a remote-only branch, or the copy of
a local branch drawn beside it. A local lane, or the base, needs its branch's state; undefined
without one.
*/
export function tracking(
  lane: Pick<Lane, "kind" | "only">,
  state: BranchState | undefined,
): Tracking | undefined {
  if (lane.kind === "remote") {
    return lane.only === "remote" ? "remote-only" : "remote-copy";
  }
  if (state === undefined || state.isRemote) {
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

/** The lane's stage, as its symbol and hover text: ◆ and "Done: Merged into the base; …". */
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
  const { name, mark, tone } = STAGE_MARKS[row.stage];
  return { text: mark, help: `${name}: ${row.help}`, tone };
}

/** The work tree slot: not checked out (·), unchanged (□), or with uncommitted changes (■). */
export function worktreeMark(state: BranchState | undefined): Mark {
  const worktree = state?.worktree;
  if (worktree === undefined) {
    return WORKTREE.none;
  }
  const changes = state?.changes;
  if (changes === undefined) {
    return { ...WORKTREE.unchanged, help: `Checked out in ${worktree}.` };
  }
  return changes.changed + changes.untracked === 0
    ? { ...WORKTREE.unchanged, help: `Checked out in ${worktree}; no uncommitted changes.` }
    : {
        ...WORKTREE.changed,
        help: `Checked out in ${worktree}; uncommitted: ${String(changes.changed)} changed, ${String(changes.untracked)} untracked files.`,
      };
}

/**
The State column: three slots, always shown. Merged into the base or not; the remote branch;
the work tree. This is the branch's state, the facts its stage is made from.
*/
export function stateMarks(
  lane: Pick<Lane, "kind" | "only" | "merged">,
  state: BranchState | undefined,
): readonly [Mark, Mark, Mark] {
  let merged: Mark = lane.merged ? MERGED.merged : MERGED.open;
  if (lane.kind === "reference") {
    merged = MERGED.base;
  }
  const how = tracking(lane, state);
  const remote = how === undefined ? undefined : REMOTE_MARKS.get(how);
  return [merged, remote ?? NO_REMOTE_DATA, worktreeMark(state)];
}
