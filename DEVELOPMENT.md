# Developing sundgren

One Rust crate (Rust 1.88+); Python is only the test harness and the docs build.

```sh
uv sync                       # dev env in .venv, with the binary built (pyright resolves imports here)
uv tool install pre-commit    # once per machine
pre-commit install            # once per clone: the lint hooks run on every commit, as in CI
cargo test
uv run pytest                 # e2e and docs tests, run the built binary
```

Install pre-commit as a tool, not with `uvx`: the git hook remembers the Python it was installed
with, and `uvx` runs from uv's cache, which can be cleaned.

To release, set the version in `pyproject.toml` and `Cargo.toml`, date its `CHANGELOG.md`
section, check with `uv run tools/release.py check vX.Y.Z`, and push the `vX.Y.Z` tag. The tag
publishes to PyPI and updates the docs site. Each pull request gets a docs preview at
`https://axean.github.io/sundgren/pr-preview/pr-<N>/` until it closes, and `main`, with changes
not released yet, is at `https://axean.github.io/sundgren/main/`.

The VS Code extension has its own notes:
[editors/vscode/DEVELOPMENT.md](editors/vscode/DEVELOPMENT.md).
