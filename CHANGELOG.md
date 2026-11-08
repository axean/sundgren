# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- C++, JavaScript and TypeScript are read: outline, containment per function and class, comment
  lines left out.
- `utils/`: tools for working on sundgren: `screenshot.py`, and `outline_check.py`, which checks
  code reading on a corpus against each language's own parser.

### Changed

- Code is read with tree-sitter: one engine for every language, each language a grammar and a
  query. Items are found as the language's own parser finds them (checked against Python's `ast`,
  rust-analyzer and the TypeScript compiler on system files).
- Containment: a line found in several places of the other branch is matched to the nearest, in
  the same file and item first, then the same file, then an item of the same name elsewhere. A
  copy of the code in a file that sorts first no longer shows the original as *moved*. Totals are
  unchanged.

## [2.1.1] - 2026-11-07

### Changed

- The README, the Marketplace page and the package descriptions say what sundgren is for: viewing,
  understanding and measuring what git repositories contain and how they compare. New screenshots
  ([#53]).
- No more preview or prototype labels: not on the Marketplace, in the docs, or in `sundgren --help`
  ([#53]).
- The docs' History and Containment demos show each branch's stage and state, and History its
  commits' files and lines changed, from git output recorded with the demo ([#53]).

## [2.1.0] - 2026-11-07

### Added

- VS Code: History and Containment work in submodules and other nested repositories. The
  repository is picked once and remembered per workspace; **Repository** in the toolbar picks
  another, and the Source Control view's repository menu opens either panel there ([#36]).
- VS Code: a sundgren tree in the status bar, a menu of Show History, Show Containment and Show
  Outline ([#37]).
- VS Code: the repository picker also lists repositories in folders below the open folders, down
  to `sundgren.repositoryDepth` levels (3 by default). Worktrees and submodules are not searched
  for there: a folder needs its own `.git` folder ([#37]).
- VS Code History panel: click a commit's hash to open it on GitHub, or GitLab on any other host,
  from the repository's remote ([#39]).
- VS Code History panel: each commit's files changed and lines added and removed, left of its
  hash (a merge's against its first parent) ([#39]).
- VS Code History panel: column names in the header: Time, Branch, Changes, Commit, Message,
  Author ([#39]).
- `sundgren relate-repos OTHER`: how two clones of related repositories relate, from git history
  alone: shared commits, where each base left the other and when the copy can have been made, the
  last sync, the newest shared commit, and the merges that synced one into the other. Text, or
  `sundgren.relate-repos/0` JSON. Neither clone is written to ([#40]).
- `sundgren relate-repos` also lists, per repository, the repositories its commit messages name
  (`See merge request PATH!N`) and the hosts they link to ([#42]).
- `sundgren relate-repos` also lists the authors of the commits on one base only, per side, with
  `.mailmap` from both clones ([#44]).
- VS Code History panel: redraws by itself when a branch, tag or HEAD moves. It checks the refs
  every `sundgren.historyRefresh` seconds (5; 0 turns it off) while it shows, and runs
  `sundgren graph` again only when they changed. The scroll position and selection stay ([#50]).
- VS Code History panel: lane labels show the branch's stage and state as symbols, in named
  columns: Branch, Counts, Stage, State. The stage (Draft, Loose, Steady, Budding, Lagging,
  Tangled, Remote, Done) comes from whether the branch is merged into the base and how it stands
  against its remote branch, coloured by how much it asks for attention. The state shows those
  facts in three slots: merged or not, the remote branch, the work tree. They replace the label's
  local/remote column and its badges. A circled ? next to the column names explains each symbol.
  The Containment panel shows the same stage and state for each branch, instead of its badges
  ([#51]).
- `sundgren graph --compare-repo DIR`: the history map with another clone's branches as remote lanes
  (`NAME/BRANCH`), its base pinned, and the commits where the two split and synced marked ([#46]).
- VS Code: **sundgren: Show History With Another Repository…** opens that map; the marks show
  as badges in the label column ([#46]).
- `sundgren origin FILE... --compare-repo DIR --at REV,...`: for each file, the file of another
  repository its lines are found in at each commit given, its share, and its lead over the next
  file, as `sundgren.origin/0` JSON or text ([#43]).
- VS Code: **sundgren: Show Origin** shows that for the file in the editor, a row per commit
  picked, and inspects it against a commit's best file line by line ([#45]).

### Changed

- VS Code: everything the extension adds to VS Code's own UI says "sundgren:": buttons, menu
  entries, tooltips, pickers and panel tabs (**sundgren: History (main)**) ([#37]).
- VS Code: the status bar tree, the Source Control title button and the Source Control
  repository menu each have a `sundgren.show.*` setting that hides them ([#37]).
- VS Code History panel: the files changed are split by kind, added/modified/deleted, in green,
  amber and red (`2/3/1`), showing only the kinds there are. Renames and type changes count as
  modified, copies as added ([#49]).
- `sundgren contain` (and so `graph --show like` and `origin`) matches fuzzy lines on every core
  ([#48]).

### Fixed

- `sundgren graph` follows `.mailmap`: commit authors and emails, branch authors, and the
  `user.email` that `--show mine` looks for. The History panel's Authors list merges a person's
  emails with it ([#38]).
- `sundgren graph` gives the same JSON on every run: where lines of two lanes share cells, which
  lane they belong to no longer changed from run to run (it also changed their colour in the
  panel and the docs demo) ([#47]).

## [2.0.0] - 2026-11-01

### Changed

- `sundgren graph` draws where branches stand against a base branch: a vertical map, newest on
  top, routed like a circuit board, with the base as the reference line and local and remote refs
  as twin lanes. It prints `sundgren.graph/3` JSON, or Unicode with `--format text`. It replaces
  `sundgren.graph/1` and the `REV` and `-n` arguments ([#34]).
- VS Code: **sundgren: Show History** shows that map, and opens from Containment with its base and
  target ([#34]).

### Added

- `sundgren graph` picks branches with `--show local|recent|mine|like`, `--branches`, `--pin` and
  `--hide`, and the page with `--commits`, `--since` and `--until` ([#34]).
- VS Code History panel: a time axis, short hashes, whole commit messages on hover, a branch list
  to tick, author and date (calendar) filters, folded runs of commits, and **Load older** ([#34]).

## [1.1.1] - 2026-10-31

### Added

- The VS Code extension is on the Marketplace, as a preview ([#29]).

### Fixed

- Code in the panels and the docs shows as typed: the fonts' ligatures no longer draw `->` as an
  arrow or `>=` as ≥ ([#29]).

## [1.1.0] - 2026-10-31

### Added

- `sundgren contain A B --base REV` (prototype) measures how much of the work each branch added
  since it left the base is in the other, by content: exact and edited (fuzzy) lines. Lines that
  are only a comment are left out (`--comments count` keeps them); `--lines` lists every line and
  where it was found ([#29]).
- In Python, Rust and C files, sundgren reads the code: containment per function and class, with
  where each went in the other branch. `sundgren outline FILE` shows what it reads ([#29]).
- VS Code: **sundgren: Show Containment** compares every branch with a target, with sync,
  worktree and uncommitted-changes badges; **Inspect** shows a branch's edited and missing lines
  next to the target's; **sundgren: Show Outline** shows a file as sundgren reads it ([#29]).
- Docs: Features and Containment pages; the demos work like the panels, on data the docs build
  makes with the current sundgren ([#29]).

### Changed

- One look for the panels and the docs site (`style/`), with the Inter and JetBrains Mono fonts
  ([#29]).

## [1.0.0] - 2026-10-31

### Changed

- sundgren is now a git history visualizer. The coverage delta tool (`sundgren run`, `diff`,
  `presets`, `ingest`) and the extension's "pluck a test" are removed ([#23]).

### Added

- `sundgren graph [REV]` prints the history of a revision laid out as a graph, as
  `sundgren.graph/1` JSON ([#23]).
- VS Code: **sundgren: Show History** shows a branch's history as a graph in a panel ([#23]).
- Docs: a History graph page with a live demo, and a Development menu listing the previews of
  `main` and of every open pull request ([#23]).

[2.1.1]: https://github.com/axean/sundgren/releases/tag/v2.1.1
[2.1.0]: https://github.com/axean/sundgren/releases/tag/v2.1.0
[2.0.0]: https://github.com/axean/sundgren/releases/tag/v2.0.0
[1.1.1]: https://github.com/axean/sundgren/releases/tag/v1.1.1
[1.1.0]: https://github.com/axean/sundgren/releases/tag/v1.1.0
[1.0.0]: https://github.com/axean/sundgren/releases/tag/v1.0.0
[#23]: https://github.com/axean/sundgren/pull/23
[#29]: https://github.com/axean/sundgren/pull/29
[#34]: https://github.com/axean/sundgren/pull/34
[#36]: https://github.com/axean/sundgren/pull/36
[#37]: https://github.com/axean/sundgren/pull/37
[#38]: https://github.com/axean/sundgren/pull/38
[#39]: https://github.com/axean/sundgren/pull/39
[#40]: https://github.com/axean/sundgren/pull/40
[#42]: https://github.com/axean/sundgren/pull/42
[#43]: https://github.com/axean/sundgren/pull/43
[#44]: https://github.com/axean/sundgren/pull/44
[#45]: https://github.com/axean/sundgren/pull/45
[#46]: https://github.com/axean/sundgren/pull/46
[#47]: https://github.com/axean/sundgren/pull/47
[#48]: https://github.com/axean/sundgren/pull/48
[#49]: https://github.com/axean/sundgren/pull/49
[#50]: https://github.com/axean/sundgren/pull/50
[#51]: https://github.com/axean/sundgren/pull/51
[#53]: https://github.com/axean/sundgren/pull/53
