# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `sundgren contain A B --base REV` (prototype) measures how much of the work each branch added
  since it left the base is in the other, by content: exact and edited (fuzzy) lines. Lines that
  are only a comment are left out (`--comments count` keeps them); `--lines` lists every line and
  where it was found.
- In Python, Rust and C files, sundgren reads the code: containment per function and class, with
  where each went in the other branch. `sundgren outline FILE` shows what it reads.
- VS Code: **sundgren: Show Containment** compares every branch with a target, with sync,
  worktree and uncommitted-changes badges; **Inspect** shows a branch's edited and missing lines
  next to the target's; **sundgren: Show Outline** shows a file as sundgren reads it.
- The VS Code extension is published to the Marketplace, as a preview.
- Docs: Features and Containment pages; the demos work like the panels, on data the docs build
  makes with the current sundgren.

### Changed

- One look for the panels and the docs site (`style/`), with the Inter and JetBrains Mono fonts.

## [1.0.0] - 2026-10-31

### Changed

- sundgren is now a git history visualizer. The coverage delta tool (`sundgren run`, `diff`,
  `presets`, `ingest`) and the extension's "pluck a test" are removed.

### Added

- `sundgren graph [REV]` prints the history of a revision laid out as a graph, as
  `sundgren.graph/1` JSON.
- VS Code: **sundgren: Show History** shows a branch's history as a graph in a panel.
- Docs: a History graph page with a live demo, and a Development menu listing the previews of
  `main` and of every open pull request.

[1.0.0]: https://github.com/axean/sundgren/releases/tag/v1.0.0
