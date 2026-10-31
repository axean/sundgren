# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

- sundgren is now a git history visualizer. The coverage delta tool (`sundgren run`, `diff`,
  `presets`, `ingest`) and the extension's "pluck a test" are removed.

### Added

- `sundgren graph [REV]` prints the history of a revision laid out as a graph, as
  `sundgren.graph/1` JSON.
- VS Code: **sundgren: Show History** shows a branch's history as a graph in a panel.
- Docs: a History graph page with a live demo, and a Development menu listing the previews of
  `main` and of every open pull request.
