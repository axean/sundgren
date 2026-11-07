---
orphan: true
---

# History map

sundgren draws where branches stand against a base branch as a vertical map, newest on top, routed
like a circuit board. This is that panel, on a made-up repository:

<iframe class="sg-demo" src="_static/demo/index.html" title="The sundgren history panel, on a made-up repository" loading="lazy"></iframe>

## In VS Code

Run **sundgren: Show History** from the Command Palette, the tree in the status bar,
or the button in the Source Control title bar, and pick a base (the same default as Containment). From the Containment panel,
**History** opens it with Containment's base, showing the branches most like its target; the
**History** button under an open branch also pins that branch. `sundgren.show.*` in Settings
hides the status bar tree and the Source Control button and menu.

With several repositories (open folders, repositories in folders below them, their submodules,
the active file's), it asks which one
once and remembers it; **Repository** in the toolbar picks another, and right-clicking a
repository in the Source Control view opens History there.

- The base is the reference line. Every other lane leaves it where its branch forked, and rejoins
  it where it was merged. A remote lane holds only the commits its local branch doesn't have.
- Each commit is a row: the time on the axis at the left (the day where one starts), the graph,
  the branch labels, the files changed and lines added and removed, the short hash, the subject and
  the author, in fixed columns named in the header. The stats come in just after the map. Hover a row for
  the whole commit message. Click the hash to open the commit on its host: GitHub on github.com,
  GitLab's commit page on any other host, from the `origin` remote (else the base's remote). A label gives the branch's ahead (↑) and behind (↓) counts against the base, then its stage
  and its state as symbols (below); hover one for what it means.
- **Show** picks the branches: local first, recent activity, mine (commits by your
  `user.email`), or like a branch (most contained in it, as in Containment). The first 12 are
  shown; **Branches** lists them all in that order: tick or untick any, and the map follows.
  All, First 12, Local and Remote tick a whole group (or untick it, if it is all ticked); Reset
  goes back to the first 12.
- **Authors** lists who wrote the commits on the page, by email, as the repository's `.mailmap`
  maps them. Untick someone and their commits shrink to thin rows, the lines still running
  through them.
- **Dates** picks the days: the last 7, 30 or 90, or click a first and a last day in the
  calendar (a dot marks days with commits on the page). Any time goes back to the newest
  commits.
- A run of commits on one lane with nothing else happening is folded into one row; click ×N to
  unfold it, and again to fold it.
- **Load older** at the bottom reads more of the base's history (`sundgren.historyPage` commits,
  50 by default) and keeps the scroll position.
- The map redraws by itself when a branch, tag or HEAD moves, from VS Code or a terminal: the
  panel checks every `sundgren.historyRefresh` seconds (5 by default, 0 turns it off) while it
  shows. Uncommitted changes don't redraw it.
- Each branch has its own colour (12, then dotted). Remote lanes are thinner and dashed. Protected
  branches (the `sundgren.protected` setting, default `main` and `master`) are drawn in the text
  colour. Merged branches are faded.
- A lane broken around a horizontal line passes under it, it doesn't join it. A pad at the
  bottom: the lane goes on in older history; the line under the map says how many commits.
- The legend under the map lists the markings in use.

### Stages and state

A branch's **stage** is where it is in its life. It comes from two things only: whether it is
merged into the base (by commit, so a squash merge doesn't count), and how it stands against its
remote branch. Every combination has exactly one stage; the hover text says which combination it
is.

| Symbol | Stage   | Typically                                                                   |
| ------ | ------- | --------------------------------------------------------------------------- |
| ○      | Draft   | Not merged, never pushed.                                                   |
| ⊘      | Loose   | Not merged, remote branch deleted: usually a squash merge.                  |
| ═      | Steady  | Not merged, the same as its remote branch.                                  |
| ▲      | Budding | Has commits its remote branch doesn't.                                      |
| ▼      | Lagging | Its remote branch has commits it doesn't.                                   |
| ⋈      | Tangled | It and its remote branch each have commits the other doesn't.               |
| ◇      | Remote  | Only on the remote, or the remote's copy of a local branch drawn beside it. |
| ◆      | Done    | Merged into the base, and its remote branch is gone or the same.            |
| ?      | Unclear | No branch data for the lane.                                                |

The base is marked ◉. The colour says how much a stage asks for attention: green for Done, amber
for Loose, Budding and Lagging, red for Tangled.

The **state** column gives the facts a stage is made from, always in three slots:

| Slot          | Symbols                                                                                                                 |
| ------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Merged        | ∈ merged into the base, ∉ not                                                                                           |
| Remote branch | ∅ never pushed, × deleted, = the same, + ahead, − behind, ± both, ∄ no local branch, ≈ the server's copy of a local one |
| Work tree     | · not checked out, □ checked out and unchanged, ■ with uncommitted changes                                              |

So the ways to be Done read apart: `∈ ×` remote deleted, `∈ ∅` never pushed, `∈ =` remote still
there. The circled ? next to the column names shows both tables.

## From the command line

The extension runs the `sundgren` binary bundled with it:

```console
$ sundgren graph --repo . --base main --show local --format text
```

`--format json` prints a `sundgren.graph/3` document
([schema](https://github.com/axean/sundgren/blob/main/schema/graph.schema.json)): the lanes, the
commits, the folds, and the drawing as a grid of cells, one character each, which the panel draws
as is.
