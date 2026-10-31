---
orphan: true
---

# History graph

sundgren draws the history of a branch as a graph, in a VS Code panel. This is that panel, on a
made-up repository:

<iframe class="sg-demo" src="_static/demo/index.html" title="The sundgren history panel, on a made-up repository" loading="lazy"></iframe>

## In VS Code

Run **sundgren: Show History** from the Command Palette, or from the button in the Source Control
title bar, and pick a branch.

- Each row is a commit: its branches and tags, subject, author, date and short sha. Hover a row for
  the full sha and both dates; click it to select it.
- Each branch line has its own colour. A hollow node is a merge.
- The current branch is filled in orange.
- `sundgren.maxCommits` (default 2000) limits how many commits are shown.

## From the command line

The extension runs the `sundgren` binary bundled with it:

```console
$ sundgren graph main --repo . -n 1000
```

It prints the history as `sundgren.graph/1` JSON
([schema](https://github.com/axean/sundgren/blob/main/schema/graph.schema.json)): the commits,
children before parents, each with its column and colour, and the lines from the row above.
