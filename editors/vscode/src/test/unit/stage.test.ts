import * as assert from "node:assert/strict";

import type { BranchState } from "../../git.js";
import type { Lane } from "../../graph.js";
import { STAGE_MARKS, STAGES, stageMark, tracking, worktreeMark } from "../../stage.js";

const LOCAL = { kind: "local" } as const;

/** A local branch's state with `fields`. */
function local(fields: Partial<BranchState> = {}): BranchState {
  return { name: "feature", isRemote: false, ...fields };
}

const UPSTREAM = { upstream: "origin/feature", ahead: 0, behind: 0 };

function sorted(keys: Iterable<string>): string[] {
  return [...keys].toSorted((a, b) => a.localeCompare(b));
}

describe("tracking", () => {
  it("reads a local branch's upstream", () => {
    const cases: [BranchState, string][] = [
      [local(), "none"],
      [local({ upstream: "origin/feature", isGone: true }), "gone"],
      [local(UPSTREAM), "level"],
      [local({ ...UPSTREAM, ahead: 2 }), "ahead"],
      [local({ ...UPSTREAM, behind: 3 }), "behind"],
      [local({ ...UPSTREAM, ahead: 2, behind: 3 }), "diverged"],
    ];
    for (const [state, expected] of cases) {
      assert.equal(tracking(LOCAL, state), expected, JSON.stringify(state));
    }
  });

  it("tells a remote-only branch from the copy of a local one", () => {
    assert.equal(tracking({ kind: "remote", only: "remote" }, undefined), "remote-only");
    assert.equal(tracking({ kind: "remote" }, undefined), "remote-copy");
  });

  it("has none for a local lane without its branch's state, or the base", () => {
    assert.equal(tracking(LOCAL, undefined), undefined);
    assert.equal(tracking({ kind: "reference" }, local(UPSTREAM)), undefined);
  });
});

describe("stageMark", () => {
  // Every kind of lane, merged or not, with every kind of state there is.
  const lanes: Pick<Lane, "kind" | "only">[] = [
    { kind: "local" },
    { kind: "local", only: "local" },
    { kind: "remote" },
    { kind: "remote", only: "remote" },
  ];
  const states: (BranchState | undefined)[] = [
    undefined,
    local(),
    local({ upstream: "origin/feature", isGone: true }),
    local(UPSTREAM),
    local({ ...UPSTREAM, ahead: 1 }),
    local({ ...UPSTREAM, behind: 1 }),
    local({ ...UPSTREAM, ahead: 1, behind: 1 }),
  ];

  it("reaches every row of the table, and only runes it knows", () => {
    const runes = new Set([...Object.values(STAGE_MARKS).map((mark) => mark.rune), "?"]);
    const reached = new Set<string>();
    for (const lane of lanes) {
      for (const isMerged of [false, true]) {
        for (const state of states) {
          const how = tracking(lane, state);
          if (how !== undefined) {
            reached.add(`${isMerged ? "merged" : "open"} ${how}`);
          }
          assert.ok(runes.has(stageMark({ ...lane, merged: isMerged }, state).text));
        }
      }
    }
    assert.deepEqual(sorted(reached), sorted(Object.keys(STAGES)));
  });

  it("says the stage and the row's meaning", () => {
    const mark = stageMark(
      { kind: "local", merged: true },
      local({ upstream: "o/f", isGone: true }),
    );
    assert.deepEqual(mark, {
      text: "ᛅ",
      help: "Done: Merged into the base; remote branch deleted.",
    });
    assert.equal(stageMark({ kind: "local", merged: false }, local()).text, "ᛜ");
    assert.equal(stageMark({ kind: "remote", merged: true }, undefined).text, "ᚱ");
  });

  it("marks the base, and a lane it knows nothing about", () => {
    assert.equal(stageMark({ kind: "reference", merged: false }, undefined).text, "ᛒ");
    assert.equal(stageMark({ kind: "local", merged: false }, undefined).text, "?");
  });
});

describe("worktreeMark", () => {
  it("marks a checked-out branch clean or with uncommitted changes", () => {
    assert.equal(worktreeMark(local()), undefined);
    assert.equal(worktreeMark(undefined), undefined);
    const at = { worktree: "/w" };
    assert.equal(worktreeMark(local({ ...at, changes: { changed: 0, untracked: 0 } }))?.text, "ᛟ");
    assert.equal(worktreeMark(local(at))?.text, "ᛟ");
    const dirty = worktreeMark(local({ ...at, changes: { changed: 2, untracked: 1 } }));
    assert.deepEqual(dirty, {
      text: "ᚺ",
      help: "Checked out in /w; uncommitted: 2 changed, 1 untracked files.",
    });
  });
});
