<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/axean/sundgren/main/docs/_static/brand/sundgren-logo-dark.svg">
  <img alt="sundgren" src="https://raw.githubusercontent.com/axean/sundgren/main/docs/_static/brand/sundgren-logo.svg" width="420">
</picture>

[![CI](https://github.com/axean/sundgren/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/axean/sundgren/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/axean/sundgren/badges/coverage.json)](https://github.com/axean/sundgren/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/sundgren)](https://pypi.org/project/sundgren/)
[![License](https://img.shields.io/pypi/l/sundgren)](https://github.com/axean/sundgren/blob/main/LICENSE)
[![Docs](https://img.shields.io/badge/docs-axean.github.io%2Fsundgren-blue)](https://axean.github.io/sundgren/)

*Sundgren* is Swedish for "sound branch". Given a repo, a base ref and a patch (usually one that
changes tests), it answers: **what happened to source coverage because of this patch?** It runs
your coverage command on the tree before and after the patch, and reports the lines, branches,
functions and test outcomes that changed.

## Install

```sh
uv tool install sundgren      # or: pipx install sundgren
```

Linux and macOS (x86_64 and arm64). The wheels hold a self-contained binary, so running it needs
no Python. Windows is not supported yet.

## Docs

- [Examples](https://axean.github.io/sundgren/): a Python and a Rust project, the command and
  the report.
- [How it works](https://axean.github.io/sundgren/how-it-works.html)
- [Reference](https://axean.github.io/sundgren/reference.html): every option, preset key, exit
  code and warning.
- [Changelog](https://github.com/axean/sundgren/blob/main/CHANGELOG.md)

## Development

One Rust crate (Rust 1.88+); Python is only the test harness and the docs build.

```sh
cargo test && cargo clippy --all-targets -- -D warnings
uv sync && uv run pytest      # e2e and docs tests, run the built binary
```

To release, set the version in `pyproject.toml` and `Cargo.toml`, date its `CHANGELOG.md`
section, check with `uv run tools/release.py check vX.Y.Z`, and push the `vX.Y.Z` tag. The tag
publishes to PyPI and updates the docs site. Each pull request gets a docs preview at
`https://axean.github.io/sundgren/pr-preview/pr-<N>/` until it closes.

## License

Apache-2.0. See [LICENSE](https://github.com/axean/sundgren/blob/main/LICENSE).
