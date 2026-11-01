---
orphan: true
---

# History map

sundgren draws where branches stand against a base branch as a vertical map, newest on top, routed
like a circuit board. This is that panel, on a made-up repository:

<iframe class="sg-demo" src="_static/demo/index.html" title="The sundgren history panel, on a made-up repository" loading="lazy"></iframe>

## In VS Code

Run **sundgren: Show History** from the Command Palette, or from the button in the Source Control
title bar, and pick a base (the same default as Containment). From the Containment panel,
**History** opens it with Containment's base, showing the branches most like its target; the
**History** button under an open branch also pins that branch.

- The base is the reference line. Every other lane leaves it where its branch forked, and rejoins
  it where it was merged. A remote lane holds only the commits its local branch doesn't have.
- Each commit is a row: the time on the axis at the left (the day where one starts), the graph,
  the branch labels, the short hash, the subject and the author, in fixed columns. Hover a row for
  the whole commit message. A label gives the branch's ahead (↑) and behind (↓) counts against the base, local or
  remote (`= origin` when the remote is at the same commit), and the same badges as Containment.
- **Show** picks the branches: local first, recent activity, mine (commits by your
  `user.email`), or like a branch (most contained in it, as in Containment). The first 12 are
  shown; **Branches** lists them all in that order: tick or untick any, and the map follows.
  All, First 12, Local and Remote tick a whole group (or untick it, if it is all ticked); Reset
  goes back to the first 12.
- **Authors** lists who wrote the commits on the page, by email. Untick someone and their
  commits shrink to thin rows, the lines still running through them.
- **Dates** picks the days: the last 7, 30 or 90, or click a first and a last day in the
  calendar (a dot marks days with commits on the page). Any time goes back to the newest
  commits.
- A run of commits on one lane with nothing else happening is folded into one row; click ×N to
  unfold it, and again to fold it.
- **Load older** at the bottom reads more of the base's history (`sundgren.historyPage` commits,
  50 by default) and keeps the scroll position.
- Each branch has its own colour (12, then dotted). Remote lanes are thinner and dashed. Protected
  branches (the `sundgren.protected` setting, default `main` and `master`) are drawn in the text
  colour. Merged branches are faded.
- A lane broken around a horizontal line passes under it, it doesn't join it. A pad at the
  bottom: the lane goes on in older history; the line under the map says how many commits.
- The legend under the map lists the markings in use.

## From the command line

The extension runs the `sundgren` binary bundled with it:

```console
$ sundgren graph --repo . --base main --show local --format text
```

`--format json` prints a `sundgren.graph/3` document
([schema](https://github.com/axean/sundgren/blob/main/schema/graph.schema.json)): the lanes, the
commits, the folds, and the drawing as a grid of cells, one character each, which the panel draws
as is.
