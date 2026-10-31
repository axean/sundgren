---
orphan: true
---

# Containment

> [!NOTE]
> A prototype. How it decides what counts as the same work is still open.

Git records where commits came from, not what is in them. Once work is rebased, squashed or
extracted from a dev branch into its own merge request, git no longer knows it is the same work.
sundgren compares branches by their content instead: for two branches, how much of the work each
added since it left the base is found in the other. This is the panel, on a made-up project
(below) compared with its `dev` branch. It works as in VS Code: pick another **Target**, count
**Comments** or not, click a branch, and **Inspect** it line by line. The results are sundgren's
own, computed when these docs were built.

<iframe class="sg-demo" src="_static/demo/containment.html" title="The sundgren containment panel, on a made-up repository" loading="lazy"></iframe>

## Reading it

- **Lines**: what a branch added since it left the base, at the commit shown. Lines that are only
  a comment, in Python, Rust and C files, are left out (toolbar: *Comments*).
- **Thick bar**: how much of the branch's work is in the target. Solid is lines found as they
  are; light is edited lines, found as a line at least 90 % similar.
- **Thin bar**: how much of the target's work is in the branch.
- **Label**: *in target* when the branch is (almost) all in the target but not the other way round,
  as for an extraction; *same work* when both are; *overlap*, *unrelated*, or *nothing ahead*
  for a branch that is already merged.
- **Dashed line**: the median of all branches' bars, roughly what lines unrelated branches share
  anyway reach.
- **Badges** (in VS Code): local or remote only, in sync with its upstream or ahead and behind, and
  checked out in a worktree, with a dot for uncommitted changes. Uncommitted changes are not
  compared.
- Click a branch to see its files. In Python, Rust and C files, sundgren also reads the code:
  each file lists its functions and classes with how much of each is in the target, and the item
  there most of it was found in (*→ name*), tagged *moved* or *renamed* when that is in another
  file or under another name. **Inspect** shows every line of the branch next to the line of the
  target it was found as.

## The demo repository

sensorlink, telemetry for a lab's sensor boards: a C ring buffer (`c/`), a Rust frame parser
(`rust/`) and Python log analysis (`python/telemetry/`). Every source file starts with the same
license header. The branches, compared with `dev` from `main`:

| Branch | Expected | What it is |
| --- | --- | --- |
| `dev` | target | Ring buffer overwrite mode and peek (C), a CRC-16 after each frame (Rust), smoothing filters and CSV export (Python), and two refactors. |
| `frame-crc` | in target | dev's CRC check, extracted as is: all exact matches. |
| `ringbuf-overwrite` | in target | dev's ring buffer work, extracted and edited in review: the edited lines are fuzzy matches. |
| `restructure-filters-and-crc` | in target | dev's filters and CRC check, reorganized: `rolling_mean` in `smoothing.py` is dev's `moving_average` in `stats.py`; the method `Frame::verify_checksum` is dev's function `check_crc`. |
| `docs-and-license-headers` | unrelated | Doc comments and missing license headers, one line of code. Comment lines are ignored, so it is near 0 %. |
| `dev-rebased` | same work | dev rebased onto a newer main. |
| `csv-export-and-plots` | overlap | Half of dev's CSV export, plus plots. |
| `serial-capture`, `channel-config`, `framedump-cli`, `tick-clock` | unrelated | Other work in the same project. |
| `fix-readme-typo` | nothing ahead | Already merged into main. |

`uv run tools/demo_containment.py --repo DIR` builds the repository in `DIR` to try by hand, with
a remote (branches in sync, ahead, behind, only on the remote, local only) and a worktree with
uncommitted changes.

## In VS Code

Run **sundgren: Show Containment** and pick the target branch. A branch's work is what it added
since it left the canonical branch: the repository's default branch (what `origin/HEAD` points
at, else `main` or `master`), or the one the `sundgren.base` setting names. Every other branch is
compared with the target, a few at a time, and takes its place in the list as soon as it is
done.

## From the command line

```console
$ sundgren contain feature dev --base main --format text
```

compares two branches and prints `sundgren.containment/0` JSON, or a summary with
`--format text`. `--fuzzy T` sets how similar an edited line must be (default 0.9; 1 for exact
matches only), `--comments count` counts comment lines as work, and `--lines` lists every line
and where it was found.
