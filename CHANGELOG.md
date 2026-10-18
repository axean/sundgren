# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Logo: the README shows it instead of the title. The docs site shows it at the top of the page,
  with the round icon in the header and as the favicon. Light and dark variants.
- Changelog page on the docs site, linked from the header.
- Line coverage of sundgren's own code in CI: measured in-process over everything outside
  `tests/` and `docs/examples/`, with a summary table, an HTML report artifact and a Codecov
  badge in the README.

### Changed

- The docs site uses the logo's colours: an orange accent (darker in light mode for contrast) and
  warm greys, in light and dark mode. Note callouts use the accent instead of blue.

## [0.1.1] - 2026-10-18

### Added

- Docs site at https://axean.github.io/sundgren/ with a Python and a Rust example: skipping
  `test_divide`, or marking it `#[ignore]`, makes `divide` go dark. Each shows its project, patch
  and command, a graphic drawn from the recorded JSON report, and the text and JSON reports.
- Docs examples in `docs/examples/`: a project, a patch, the exact commands and their recorded
  output. The docs tests rerun them in CI (the Python example on 3.11–3.14, the Rust one with the
  `cargo-tarpaulin` preset on rustc 1.99.1 and cargo-tarpaulin 0.37.5) and fail on any difference;
  `tools/docs_examples.py --update` re-records.
- A docs test that fails if a docs page has a code block that isn't an include of a tested example
  file.
- Docs workflow: the docs tests and a strict Sphinx build run on every push and PR.
- This changelog, and a versioning policy in the README: before 1.0, a breaking change to the CLI,
  the report schema, the preset format or the built-in preset names bumps the minor version.
- README badges for CI, PyPI, Python versions, license, docs, Ruff, Black style and mypy.

### Changed

- The version is set in `pyproject.toml` (`uv version --bump patch`); `sundgren.__version__` reads
  the installed package metadata.
- Stricter release checks (`tools/release.py`): the tag must be `vX.Y.Z` and equal the
  `pyproject.toml` version, and the changelog must have a section for it and an empty
  `## Unreleased`. The GitHub release notes are that section.
- The PyPI publish waits for the docs tests and a strict docs build; the site deploys to GitHub
  Pages after the publish.

## [0.1.0] - 2026-10-18

> [!NOTE]
> First release. This entry was written after the release.

### Added

- `sundgren run`: runs a coverage command on the base tree and on the patched tree (git worktrees),
  diffs the two lcov files with line numbers mapped through the patch, and reports what the patch
  did to lines, branches, functions and test outcomes.
- `sundgren diff` for two existing lcov files, `sundgren ingest` (a normalized lcov snapshot as
  JSON) and `sundgren presets`.
- Built-in presets `pytest` (coverage.py + pytest, selecting the test files the patch touches),
  `cargo-llvm-cov` and `cargo-tarpaulin`. Custom presets go in `sundgren.toml` or
  `[tool.sundgren.presets]`; any tool that writes lcov works through `--cmd` and `--lcov`.
- JSON report with schema `sundgren.delta/1`: line categories (`lost`, `gained`, `new_hit`, ...),
  branch categories, function status (`went_dark`, `lit_up`, ...), JUnit test outcomes, a
  per-directory `tree` rollup and warnings. `--format text` prints a summary.
- `--repeat N` reports lines whose status varies across runs as `flaky`. `--full` keeps unchanged
  entries, `--counts` adds hit-count changes.
- Exit codes: 0 ok, 1 tool or run failure, 2 usage error, 3 patch doesn't apply.

[0.1.0]: https://github.com/axean/sundgren/releases/tag/v0.1.0
[0.1.1]: https://github.com/axean/sundgren/releases/tag/v0.1.1
