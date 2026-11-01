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
- Lane colours may be offshoots of the logo colours (near in hue and vibrancy); a larger N
  matters more than exact logo pixels. The palette has 7 families, each with a remote twin, one
  set per theme (VS Code's body class picks it, as in `brand.css`). Swatches in
  `style/lane-palette.png`. All at least 3:1 on their background.

  | family | light local / remote  | dark local / remote   | logo colour   |
  | ------ | --------------------- | --------------------- | ------------- |
  | orange | `#C26200` / `#9C5E05` | `#F48C3D` / `#F7AC59` | orange        |
  | rose   | `#C45865` / `#A85048` | `#F5848F` / `#FFA196` | rose          |
  | forest | `#4B5C30` / `#344D2D` | `#7B8E60` / `#86A27E` | moss, darker  |
  | wine   | `#922A3C` / `#782420` | `#CB5F6B` / `#D77A70` | rose, darker  |
  | sage   | `#899971` / `#71886A` | `#BACCA2` / `#9FBA9A` | sage          |
  | brown  | `#7E441B` / `#633A0A` | `#B4754D` / `#BC8E62` | brown         |
  | pink   | `#D17D80` / `#B67166` | `#FEB4B6` / `#E79EBD` | pink          |

  Method: OKLCH hue and chroma from the logo colour, lightness per theme until contrast holds;
  the twin turns the hue 12° and moves lightness one step. Gold, olive, moss and peach came out
  too close to others. Closest families: rose, wine and pink (OKLab 0.06–0.07). Above 7 branches,
  line style (dotted, double).
  On dark backgrounds the pale families (sage, pink) have darker remote twins, and pink's turns
  towards magenta, away from rose: lighter twins came out near the protected colour (the text
  colour). Protected lanes are also drawn heavier. Families come from the branch name (a hash;
  the next free one if taken), so a branch keeps its colour from frame to frame. The panel orders
  them orange, forest, wine, sage, rose, brown, pink, so neighbours differ.
- Branch selection: the same controls as Containment. Base = the reference line, viewed around
  its tip; target = a chosen branch, pinned; both are dropdowns. Candidates are the branches
  visible in the frame (a fork, merge or tip in it, or a lane through it), which can be
  thousands. They are ranked by containment against the target (the fuzzy matching math), and
  the top N are shown (N from a dropdown). Walking in time changes the candidates. Speed comes later.
- Default target: the checked-out branch.
- No result cache for now: slow is acceptable in the prototype, stale caches are not.
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

## Built (prototype)

`sundgren graph` (`src/map.rs`) prints the map as `sundgren.graph/2` JSON
(`schema/graph.schema.json`) or as Unicode (`--format text`):

```console
$ sundgren graph --base main --target feat-b --branches 3 --columns 40 --format text
```

- Lanes: each ref's first-parent history down to a commit another lane already has. Longer
  branches are placed first, so a branch off a branch hangs off it. Twins share columns.
- Frame: `--offset` base commits back from the tip; as many base commits as fit `--columns`.
  `frame.older`/`frame.newer` step by about 3/4 of the frame, so frames overlap.
- A merge leaves its lane at the merge commit's column when the merged commit is the lane's
  last one before it. Otherwise it gets a row of its own next to its lane (a grid row with no
  lane), from the merged commit to the merge commit's column.
- Rows: branches placed one by one (latest fork first), above or below the reference, at the
  position with the fewest vias; ties go below and farther out.
- Ranking runs `contain` for every candidate against the target, every time (no cache).
- Tests: the merge/rebase sketch as a real repository, with Unicode snapshots
  (`src/map/tests.rs`); e2e JSON validated against the schema (`tests/e2e/test_graph.py`).

Known limits:

- On a real repository (about 50 branches, 2k commits), the map without a target takes 0.06 s;
  ranking 10 candidates against a big dev branch took 68 s (fuzzy matching).
- A merge commit shared by twins connects to one twin only.
- Time order uses committer dates; a lane whose commits are out of date order can look odd.

## Resolved by convention (veto welcome)

- Bridge glyph for a crossing: `──╴│╶──`, the lane breaks and the vertical runs through.
- The graphics follow the Unicode geometry: orthogonal traces, rounded corners, same grid.
- Protected branches come from a setting with name patterns, default `main` and `master`. No
  host API calls.
- Lane tips carry the same badges as Containment rows: worktree, clean/dirty, sync state.
- Tags are labels on their commits, like branch names.
- No colour codes in terminal output for now; the colour role in the layout leaves the door open.
- `style/logo-lane-swatches.png` is left out of the .vsix (it is a reference, no page uses it).

## Parked

- Brand palette: augmented with the lane palette as new tokens; c0–c7 stay as they are. Exact-pixel twin pairs from the logo (real pixel values; swatches in
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

## Feedback round 1 (2026-11-01), on the horizontal map

Axel's notes after trying it on the sketch repo and bragi-quantum:

1. A lot of vertical space is unused.
2. The branch labels are good, but they jump left and right when stepping through history.
3. Wants a fixed space on the right for the labels, with the details (commits, remote, ...) in
   tab-aligned columns.
4. Showing origin on the same graph is OK, but local and remote are confused visually far more
   than wanted; branches changing as you step makes it worse.
5. Stepping through history jumps too much; no feeling of continuity. A setting for the step,
   small by default.
6. Needs many more than 7 branches. Brand cohesion can go for lanes: a palette that tells many
   branches apart.
7. Choosing a target doesn't make sense: in practice base = target and step through history.
8. The "most in main ⓘ" note suggests there are other choices, but there are none.
9. Other ways to choose which branches are shown would be good.
10. Runs of commits repeat the same information: fold them, like `─○─ (×7)`.
11. Mostly origin branches show up, but local branches are usually more interesting.
12. Flip it: a vertical graph (guitar hero style), vertical scrolling is fine.

Decision: flip it vertical first (folding is then free: fewer rows, other lanes just get
shorter; scrolling replaces frames). Branch `history-vertical`, off `history-map`.

### Choices for the vertical map (veto welcome)

- Newest commit at the top, one row per commit, one column per lane; it replaces the horizontal
  map. Same lane and routing engine, turned; the Unicode reads like `git log --graph`.
- Right side: fixed table columns per row: refs and lane labels at the lane's tip, subject,
  author, date. Lane details aligned in columns.
- A page of history (150 commits by default); "Load older" grows it and keeps the scroll
  position. No ◀ ▶.
- Folding: a run of 3 or more commits on one lane, with nothing else happening in those rows, is
  one row (`○ ×7`); click to unfold. The Unicode prints it folded.
- Local and remote: in sync = one lane with a sync mark; otherwise the remote lane is thinner
  and dashed. Local branches come before remote ones.
- A Show dropdown replaces Target: Local first (default), Recent activity, Mine, Like a branch…
  (the containment ranking). Up to 30 branches, 12 by default. The ⓘ note goes.
- Lane palette: about 12 well-separated hues per theme, line styles for repeats; protected stays
  the text colour.
