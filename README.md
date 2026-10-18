# sundgren

[![CI](https://github.com/axean/sundgren/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/axean/sundgren/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/sundgren)](https://pypi.org/project/sundgren/)
[![Python](https://img.shields.io/pypi/pyversions/sundgren)](https://pypi.org/project/sundgren/)
[![License](https://img.shields.io/pypi/l/sundgren)](https://github.com/axean/sundgren/blob/main/LICENSE)
[![Docs](https://img.shields.io/badge/docs-axean.github.io%2Fsundgren-blue)](https://axean.github.io/sundgren/)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)
[![Checked with mypy](https://www.mypy-lang.org/static/mypy_badge.svg)](https://mypy-lang.org/)

*Sundgren* is Swedish for "sound branch". Given a repo, a base ref and a patch (usually one that
changes tests), it answers: **what happened to source coverage because of this patch?**

It runs a coverage command on the base tree and on the patched tree (git worktrees), diffs the two
lcov files with line numbers mapped through the patch, and writes a JSON report
(`sundgren.delta/1`) or a text summary.

## Install

```sh
uv tool install sundgren      # or: pipx install sundgren
```

Sundgren never imports coverage or pytest itself. They run as commands in your project's
environment: `{python}` is the `python` on your `PATH` (or `python3`), override with `--python`.

## Python

```sh
sundgren run --patch skip_divide.patch                 # preset "pytest" is picked from pyproject.toml
sundgren run --patch p.patch --base main --format text
```

The `pytest` preset runs `coverage run --branch -m pytest <selected tests>` then `coverage lcov`.
It only runs the test files the patch touches; if the patch touches `conftest.py`, any other `.py`
file or pytest config, it runs everything. Override with `--tests tests/test_x.py ...`.

- Set `[run] source = <your package>` in `.coveragerc`, so never-imported files are reported.
- Editable installs measure your main checkout, not the worktree. Sundgren detects this and exits 1;
  fix with `--env PYTHONPATH={worktree}/src`.

## Rust

```sh
sundgren run --patch ignore_divide.patch                          # cargo-llvm-cov
sundgren run --patch ignore_divide.patch --preset cargo-tarpaulin
sundgren run --patch p.patch --tests divide                       # name filters, passed after --
```

Each run uses its own `CARGO_TARGET_DIR`, so both sides build from scratch. Function names are
stored mangled.

## Any language: `--cmd`

Anything that writes lcov works:

```sh
sundgren run --patch p.patch \
  --cmd "npx jest --coverage --coverageReporters=lcov --coverageDirectory={out}" \
  --lcov "{out}/lcov.info"
```

Placeholders: `{python}`, `{out}` (per-run output dir), `{worktree}`, `{lcov}`, `{junit}`,
`{tests}`. A token that is exactly `{tests}` expands to one argv entry per selected test (none if
nothing is selected). `--shell` runs commands through `/bin/sh -c`. `--junit TEMPLATE` adds test
outcomes to the report. `--env K=V` sets (templated) environment variables.

Custom presets go in `sundgren.toml` (`[presets.<name>]`) or `[tool.sundgren.presets.<name>]` in
`pyproject.toml`, with the same keys as `src/sundgren/lang/python/presets.toml`. `sundgren presets`
lists them.

## Other commands

```sh
sundgren diff base.lcov patched.lcov --patch p.patch --base-root A --patched-root B
sundgren ingest coverage.lcov --root .        # normalized snapshot as JSON
```

Options shared by `run` and `diff`: `--format json|text`, `-o OUT`, `--full` (include unchanged
lines, functions and tests), `--counts` (hit-count changes on lines that stayed hit), `--repeat N`
(`run` only; lines whose status varies across runs are reported as `flaky`).

Exit codes: 0 ok, 1 tool or run failure, 2 usage error, 3 patch doesn't apply.

## Report

`files[].lines[].category` is one of `lost`, `gained`, `new_hit`, `new_miss`, `deleted_hit`,
`deleted_miss`, `kept_hit`, `kept_miss`, `unmeasured` (genhtml: LBC, GBC, GNC, UNC, DCB, DUB, CBC,
UBC). Functions are `went_dark`, `lit_up`, `partial_loss`, `partial_gain`, `mixed`, `unchanged`,
`added` or `removed`. `tree` rolls the line stats up per directory. Schema:
`src/sundgren/core/report/delta.schema.json`.

## Versioning

Sundgren follows [SemVer](https://semver.org/). The public API is the CLI (commands, flags, exit
codes), the report schema, the preset / `sundgren.toml` format and the built-in preset names. The
Python modules are internal.

Before 1.0, a breaking change bumps the minor version and anything else the patch version. The
report schema has its own version, `sundgren.delta/N`; N goes up on a breaking schema change.
Changes are listed in `CHANGELOG.md`.

## Development

```sh
uv sync                       # dev env in .venv
uv run pytest
uv run ruff check . && uv run ruff format --check .
uv run mypy
```

To release, run `uv version --bump patch` (or `minor`), move the `Unreleased` entries in
`CHANGELOG.md` to a new `## [X.Y.Z] - YYYY-MM-DD` section with its link, check with
`uv run tools/release.py check vX.Y.Z`, then push a `vX.Y.Z` tag.

Layers: `core` (no languages, no subprocesses) ← `run` (worktrees, command templates) ← `lang`
(presets, selectors, symbolizers). Only `cli.py` imports `lang`. `tests/unit/test_layering.py`
enforces this. See `DECISIONS.md` for choices the spec left open.

## License

Apache-2.0. See `LICENSE`.
