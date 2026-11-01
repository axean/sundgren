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
- Colour is branch identity. The remote twin is a different hue of it. Merged is faded,
  remote-only is dashed. Other markings are free as long as the legend explains them.
- Containment results stay off the map in the first version, but the layout model has room for
  other edge kinds (content links, patch-id matches) later.
- No click actions yet, but commits and lanes are clickable elements (hover, focus, an event to
  the extension).
- **Show History** on branch X: X is the reference. The view is centred on X's tip with its
  offshoots (branches ahead of X, merged into X, or left behind on X).
- **From Containment:** the reference is the base, and each branch hangs off it at
  `merge-base(base, branch)`, the fork Containment already measures from.
- The frame is as wide as the panel: as many commits as fit, never horizontal scrolling, no
  folding of quiet stretches (unfolding them would break the space budget). Buttons walk
  backwards and forwards in time by overlapping steps (like 1900–1950, then 1940–1990), so the
  connection between frames stays visible. This replaces the earlier "show older" growing.
  Offshoots that forked before the frame come in through an off-board connector, labelled with
  how far back they forked.

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

## Resolved by convention (veto welcome)

- Bridge glyph for a crossing: `──╴│╶──`, the lane breaks and the vertical runs through.
- The graphics follow the Unicode geometry: orthogonal traces, rounded corners, same grid.
- Protected branches come from a setting with name patterns, default `main` and `master`. No
  host API calls.
- Lane tips carry the same badges as Containment rows: worktree, clean/dirty, sync state.
- Tags are labels on their commits, like branch names.
- No colour codes in terminal output for now; the colour role in the layout leaves the door open.
- `style/logo-lane-swatches.png` is left out of the .vsix (it is a reference, no page uses it).

## Open

1. Branch selection, one rule for both entry points. The branch set follows the frame: when you
   walk in time, it is picked again from what is in the frame. How many: a number from a
   dropdown (more branches, less space), default 3. Proposed rule: pinned branches first (from
   Containment: the target and the selected row; from Show History: none), then fill up to N
   with the branches that have the most events in the frame (fork, merge, tip), most recent
   first.

## Parked

- The palette. Local/remote twin pairs from the logo (real pixel values; swatches in
  `style/logo-lane-pairs.png`):

  | pair          | local     | remote    | note                                   |
  | ------------- | --------- | --------- | -------------------------------------- |
  | orange/amber  | `#DF7924` | `#E99231` |                                        |
  | rose/pink     | `#C45865` | `#E58F92` |                                        |
  | moss/olive    | `#718456` | `#959056` |                                        |
  | brown/umber   | `#874C24` | `#AA673D` |                                        |
  | peach/tan     | `#D6997A` | `#CDA780` | weak on white (contrast about 2.3:1)   |

  Pairs differ from each other by ΔE ≥ 19, twins within a pair by 11–20. The logo has no blue,
  purple or teal, so five is about the limit; beyond that, line style (dotted, double) or
  repeats. Red-green colour blindness: rose/pink and moss/olive become nearly the same (ΔE 4
  simulated); names in the legend and labels carry it. Replace c0–c7 with logo colours everywhere, or a separate lane palette. About six
  logo colours can be told apart: orange #DF7924, rose #C45865, moss #718456, brown #954812,
  pink #E58F92, sage #A2AB96 (pixel values; swatches in `style/logo-lane-swatches.png`). Avoid
  orange/amber #E99231 and moss/olive #959056. c1 blue, c2 teal, c3 purple, c7 red are not in
  the logo. Method: decode the base64 JPEG in the logo SVG's `<image>`, bucket pixels by hue
  and lightness, snap each pick to the nearest real pixel. Axel will raise it.
