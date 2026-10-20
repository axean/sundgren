# sundgren

<p class="sg-hero"><img class="sg-hero-light" src="_static/brand/sundgren-logo.svg" alt="" width="420"><img class="sg-hero-dark" src="_static/brand/sundgren-logo-dark.svg" alt="" width="420"></p>

Given a repo, a base ref and a patch, sundgren answers: **what happened to source coverage
because of this patch?** It runs your tests with coverage on the tree before the patch and on the
tree after it, then reports the lines, branches, functions and test outcomes that changed.

Use it to check that a test you skip, delete or rewrite doesn't leave code untested, or that a
source change comes with tests for its new lines.

## What it looks like

Skipping `test_divide`, the only test that calls `divide`, makes `divide` go dark:

```{sundgren-graphic} examples/skip_divide
```

The [examples](examples.md) show the project, the patch and the command behind this, and more
cases in Python and Rust.

## Install

Install the `sundgren` package from [PyPI](https://pypi.org/project/sundgren/) as a tool, with
uv or pipx. It is a single program for Linux and macOS and needs no Python to run. sundgren works
with Python projects (coverage.py and pytest), Rust projects (cargo-llvm-cov or cargo-tarpaulin)
and any tool that writes lcov.

## Where next

- [Examples](examples.md): common cases, from a patch that only touches tests to your own commands
  with placeholders.
- [How it works](how-it-works.md): the worktrees, which tests run, and how the two runs are
  compared.
- [Reference](reference.md): every option, preset key, placeholder, exit code and warning.
- [VS Code](vscode.md): pluck a test from the editor and see what it does to coverage.
- [Changelog](changelog.md)
