# sundgren

**A similarity check between git branches, from their common ancestor: how much of each branch's
work is in another.**

Once work is rebased, squashed, or extracted from a dev branch into its own merge request, git no
longer knows it is the same work. sundgren takes what each branch added since it left a common
base and compares it line by line, so you can see which branch already holds what, what was moved
or renamed on the way, and what is left.

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

**sundgren: Show History** draws the branches around a base branch as a vertical map, newest on
top: where each forked and merged, next to the commits in fixed columns. Show local branches
first, recent activity, your own, or those most like a branch, then tick or untick any in the
Branches list. Filter by author and by date (a calendar). Quiet runs of commits are folded
(click ×N). **History** in the Containment panel opens it too.

![The history panel](https://raw.githubusercontent.com/axean/sundgren/main/editors/vscode/images/history.png)

## Outline

**sundgren: Show Outline** shows the file in the editor as sundgren reads it: its functions and
classes, and its comments and strings.

## Languages

- **Any text file** is compared line by line.
- **Python** (`.py`, `.pyi`), **Rust** (`.rs`) and **C** (`.c`, `.h`) are also read as code:
  containment per function, class or other item, with where each went in the other branch (tagged
  _moved_ or _renamed_), and comment lines (license headers, doc comments) left out.
  - Python: `def`, `class`.
  - Rust: `fn`, `struct`, `enum`, `union`, `trait`, `impl`, `mod`, `const`, `static`, `type`,
    `macro_rules!`.
  - C: functions, `struct`, `union`, `enum`, `#define`.

## Settings

| Setting                |                                                                                               |
| ---------------------- | --------------------------------------------------------------------------------------------- |
| `sundgren.base`        | The canonical branch a branch's work is counted from. Empty: the repository's default branch. |
| `sundgren.protected`   | Branch name patterns the history draws in the protected colour (`main`, `master`).            |
| `sundgren.historyPage` | Commits of the base the history reads first, and adds with each **Load older** (50).          |
| `sundgren.path`        | A `sundgren` binary to use instead of the bundled one.                                        |

## Platforms

Linux (x64, arm64) and macOS (Intel, Apple silicon); the `sundgren` binary is bundled. sundgren is
also a command line tool: `pip install sundgren`.

More: [documentation](https://axean.github.io/sundgren/), with live demos ·
[source](https://github.com/axean/sundgren) · [issues](https://github.com/axean/sundgren/issues).
Apache-2.0.
