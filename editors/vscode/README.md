# sundgren

**See how your git branches relate: not by their ancestry, but by what is in them.**

Once work is rebased, squashed, or extracted from a dev branch into its own merge request, git no
longer knows it is the same work. sundgren compares branches by their content, so you can see
which branch already holds what, what was moved or renamed on the way, and what is left.

> **Preview.** How sundgren decides what counts as the same work is still open, and may change.

## Containment

**sundgren: Show Containment** compares every branch with a target branch you pick: how much of
each branch's work is already in the target, and how much of the target's is in it. A branch's
work is what it added since it left the canonical branch (`origin/HEAD`, else `main`).

![The containment panel: each branch with how much of its work is in dev](https://raw.githubusercontent.com/axean/sundgren/main/editors/vscode/images/containment.png)

- Each branch gets a label: _in target_ for an extraction, _same work_, _overlap_, _unrelated_.
- Edited lines count too: a line at least 90 % similar to one in the target is found.
- Comment lines (license headers, doc comments) are left out, in Python, Rust and C; a toggle
  counts them.
- In Python, Rust and C, sundgren reads the code: each function and class shows where it went in
  the target, tagged _moved_ or _renamed_.
- Badges show whether a branch is local or remote, in sync, ahead or behind, checked out in a
  worktree, and has uncommitted changes.

## Inspect

**Inspect** on a branch shows what is different: every line it added that the target doesn't have,
or has edited, next to the target's line, with the changed characters marked. The rest is folded.

![The inspect panel: a branch's edited lines next to the target's](https://raw.githubusercontent.com/axean/sundgren/main/editors/vscode/images/inspect.png)

## History

**sundgren: Show History** draws a branch's history as a graph: one lane per line of work, with
refs, subjects, authors and dates.

![The history panel](https://raw.githubusercontent.com/axean/sundgren/main/editors/vscode/images/history.png)

## Outline

**sundgren: Show Outline** shows the file in the editor as sundgren reads it: its functions and
classes, and its comments and strings.

## Settings

| Setting               |                                                                                               |
| --------------------- | --------------------------------------------------------------------------------------------- |
| `sundgren.base`       | The canonical branch a branch's work is counted from. Empty: the repository's default branch. |
| `sundgren.maxCommits` | The most commits the history shows (2000).                                                    |
| `sundgren.path`       | A `sundgren` binary to use instead of the bundled one.                                        |

## Platforms

Linux (x64, arm64) and macOS (Intel, Apple silicon); the `sundgren` binary is bundled. sundgren is
also a command line tool: `pip install sundgren`.

More: [documentation](https://axean.github.io/sundgren/), with live demos ·
[source](https://github.com/axean/sundgren) · [issues](https://github.com/axean/sundgren/issues).
Apache-2.0.
