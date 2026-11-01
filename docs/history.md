---
orphan: true
---

# History map

sundgren draws where branches stand against a base branch, around its tip, as a map routed like a
circuit board. This is that panel, on a made-up repository:

<iframe class="sg-demo" src="_static/demo/index.html" title="The sundgren history panel, on a made-up repository" loading="lazy"></iframe>

## In VS Code

Run **sundgren: Show History** from the Command Palette, or from the button in the Source Control
title bar, and pick a base (the same default as Containment). The target is the checked-out branch.
From the Containment panel, **History** opens it with Containment's base and target; the
**History** button under an open branch also pins that branch.

- The base is the reference line. Every other lane leaves it where its branch forked, and rejoins
  it where it was merged. A branch's local and remote lanes run side by side.
- Shown are the target and the branches whose work is most found in it (containment, as in the
  Containment panel), among those visible in the frame. **Branches** sets how many (1 to 7).
- The frame is as wide as the panel. ◀ and ▶ step back and forward in time, overlapping.
- Each branch has its own colour; its remote lane is the same colour turned in hue. Protected
  branches (the `sundgren.protected` setting, default `main` and `master`) are drawn in the text
  colour. Merged branches are faded, branches only on the remote dashed.
- A lane that breaks around a vertical line passes under it, it doesn't join it. A ringed dot at an
  edge: the lane goes on outside the frame, with the number of its earlier commits.
- The legend under the map lists the colours and markings in use.
- Each lane's label gives its ahead and behind counts against the base and the same badges as
  Containment: work tree, uncommitted changes, sync with its upstream.

## From the command line

The extension runs the `sundgren` binary bundled with it:

```console
$ sundgren graph --repo . --base main --target feat-b --format text
```

`--format json` prints a `sundgren.graph/2` document
([schema](https://github.com/axean/sundgren/blob/main/schema/graph.schema.json)): the lanes, the
commits, and the drawing as a grid of cells, one character each, which the panel draws as is.
