# History map (design in progress)

The History panel today draws one branch's commits as a lane graph. It becomes the view of where
all branches stand against the main line.

## Decided

- The reference line is a parameter, and everything is relative to it. No branch is special to the
  tool: main is canonical only because everyone merges into it, not by birth. With a feature
  branch as reference, main is an offshoot like any other.
- The map replaces today's per-commit view.
- A branch that really forked from another branch hangs off that branch's lane. Containment
  measures content, which can betray a fork; History shows the forks that happened.
- Small for now: from the reference tip, about 3 branches nearby (within about 5 commits). A
  smarter filter comes later; similarity alone won't do (a branch that mostly deletes code scores
  as very dissimilar).
- Two lanes per branch, local and remote, kept next to each other (layout A of the merge and
  rebase sketch). Vias where needed. Extra rows (vertical space) are fine as long as the Unicode
  rendering can show them.
- One colourmap, used consistently, in `style/brand.css` (light, dark and high-contrast). Every
  brand colour's hex comes from the logo (`docs/_static/brand/sundgren-logo.svg`, the embedded
  image). The layout carries a colour role per lane, never a hex value;
  only the graphical renderer turns roles into colours. The Unicode rendering has no colour.
- Protected branches get their own colour. A legend says what each colour means.
- **Show History** on branch X: X is the reference. The view is centred on X's tip with its
  offshoots (branches ahead of X, merged into X, or left behind on X).
- **From Containment:** the reference is the base, and each branch hangs off it at
  `merge-base(base, branch)`, the fork Containment already measures from.
- The window starts around the reference tip and grows on demand ("show older" at the left
  edge). Offshoots that forked before the window come in through an off-board connector,
  labelled with how far back they forked.

- This work upgrades the History panel; it is not a separate panel or a strip in Containment.
- Every branch is placed against one reference line by two numbers: **ahead** (its own commits)
  and **behind** (reference commits since its fork).
- All four cases are drawn:
  - ahead only: a lane leaving the reference tip;
  - ahead and behind (diverged): a lane leaving an older commit;
  - behind only (left behind): the tip is already in the reference history, either as a label on
    the line itself (fast-forwarded or never committed to) or as a lane that rejoins it (merged);
  - neither: a label at the tip.

```
          stale                         main   origin/main
            ▼                           ▼      ▼
main ●──────●──────●──────●──────●──────●──────●
     │             │      │      │      │      ╰──○──○──○   fix-x       3 ahead · 0 behind
     │             │      │      │      ╰──○──○──○──○       feature-y   4 ahead · 1 behind
     │             │      │      ╰──○──○──○──○──○──○        spike       6 ahead · 2 behind
     │             ╰─○──○─╯                                 old-fix     merged
     ╰──○──○──○──○──○──○──○ ⋯ ○                             dev       140 ahead · 6 behind
```

- Layout is routed like a circuit board: planar where possible. Where a crossing can't be
  avoided, a "via" marks it, and what a via means must be clear in the drawing.

- Every graph has a Unicode rendering, like the sketches in this note. The extension doesn't have
  to show it. The layout is computed once, on a character grid, and the Unicode and graphical
  renderers both draw from it.

Topology alone is mostly planar:

- Forks only (a tree): order rows by fork position, the latest fork nearest the line. No
  crossings.
- Merged lanes nest if their spans nest (inner span nearer the line). Two spans that overlap
  without nesting cross on one side; put one above the line and one below. A via is needed only
  when the overlap pattern can't be split over two sides.
- Branches off branches hang off their parent lane, on the side away from the line.
- Content links between branches are an arbitrary graph and won't be planar in general.

## Open

1. Horizontal distance: commit counts or dates; how to squeeze long branches and long stretches
   of the reference with no forks.
2. Content links from containment (e.g. "fix-x is 100 % in dev") on the map, or topology only?
3. Clicks on a fork commit and on a branch lane.
4. What a via looks like: a bridge (one line passes under another, two rings, no connection) or
   an off-board connector (the line ends in a labelled ring and comes back elsewhere with the
   same label), or both.
5. How literal the board look is: just the routing rules, or also the visual style (straight
   traces with 45° bends, commits as pads, vias as rings).
6. Where protection comes from: git doesn't know it, the host does (GitHub, GitLab).
7. Remote-only branches drawn differently (dashed)?
8. Worktree badge and clean/dirty dot from Containment on the lane tips?
9. Tags: labels on the map, or left out?
10. What colour encodes: branch identity (local and remote twins share it), state (merged, left
    behind, protected), or identity by hue and state by line style.
11. Colour stability: same branch, same colour in Containment and History, and across sessions?
12. Colour in terminal output (ANSI codes added at print time when stdout is a terminal, like
    git's `--color=auto`), or never.
13. The current c1 (blue), c2 (teal), c3 (purple) and c7 (red) are not in the logo, which is warm:
    oranges, roses, moss greens, browns, sage. About six lane colours from the logo can be told
    apart: orange #DF7924, rose #C45865, moss #718456, brown #954812, pink #E58F92,
    sage #A2AB96 (pixel values from the logo). Close pairs to avoid: orange/amber, moss/olive.
    Replace c0–c7 everywhere (Outline item kinds use them too), or a separate lane palette?
    Method: decode the base64 JPEG in the logo SVG's `<image>`, bucket pixels by hue and
    lightness, then snap each pick to the nearest real pixel so the hex occurs in the logo.
