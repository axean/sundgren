---
orphan: true
---

# VS Code

The sundgren extension for VS Code answers one question from the editor: **what happens to
coverage if I skip this test?**

**Contents**

- [What it does](#what-it-does)
- [Install](#install)
- [Pluck a test](#pluck-a-test)
- [Cancel](#cancel)
- [When it refuses](#when-it-refuses)
- [Settings](#settings)

## What it does

Above each test in a Rust file or a pytest test file (`test_*.py`, `*_test.py`), the extension
shows **sg: pluck this test**. Clicking it skips that one test in a patch (`#[ignore]` in Rust,
`@pytest.mark.skip` in Python) and runs sundgren's `run` with it: the whole test suite runs with
coverage on the last commit and with the test skipped, in temporary checkouts, and the extension
shows what changed. Your files are never changed.

## Install

Each [release on GitHub](https://github.com/axean/sundgren/releases) has a `.vsix` file per
platform, with the sundgren program for that platform inside: `linux-x64`, `linux-arm64`,
`darwin-x64` and `darwin-arm64`. Download the one for your machine, then in VS Code open the
Extensions view, choose **Install from VSIX…** from its `…` menu and pick the file. Windows isn't
supported yet.

The project needs its own test tools:

- Python: pytest and coverage.py in the interpreter the project uses.
- Rust: cargo-llvm-cov.

## Pluck a test

1. Commit your work. sundgren compares against the last commit, so the extension refuses while
   tracked files have uncommitted changes.
2. Click **sg: pluck this test** above the test. The test fades while its pluck runs.
3. The first time in a project, the extension shows exactly what it will run (the preset, the
   Python interpreter and the commands) and asks you to confirm. It asks again if any of that
   changes.
4. A notification shows each step: the check, the tests on the last commit, the tests without the
   plucked test, and the comparison. When the run is done it says what was lost, for example
   "Plucking test_divide: 3 lines lost, 2 branches lost, divide is no longer run (coverage 100% →
   75%)". The lost lines are highlighted in the editor, and the details are in the **sundgren**
   output panel.

The plucked test's own lines don't count: a skipped test trivially loses its own body, and in
Rust coverage includes test code. They're left out of the count and the highlights, and the
coverage percentage isn't shown then, since it would include them.

The Python interpreter is the one selected in Microsoft's Python extension, else the project's
`.venv`, else `python` on `PATH`. A Rust test runs with the `cargo-llvm-cov` preset, a Python test
with the `pytest` preset and the whole suite.

## Cancel

While a pluck runs, cancel it with the notification's **Cancel** button, or by clicking the line
above the test, which then reads **sg: plucking… (click to cancel)**. sundgren stops the test
processes and removes its temporary checkouts, usually within a second.

## When it refuses

The extension checks everything before it runs anything, and stops at the first problem with one
sentence saying what to do. It refuses when:

- the workspace isn't trusted, since running a test runs the project's code;
- the file has unsaved changes;
- the file isn't in a git repository, the repository has no commits, or a merge, rebase,
  cherry-pick, revert or bisect is in progress;
- tracked files have uncommitted changes;
- the file isn't committed, or differs from the last commit;
- the project isn't at the repository root (a `pyproject.toml`, `setup.cfg` or `pytest.ini` for
  Python, a `Cargo.toml` for Rust), or VS Code's test settings point elsewhere
  (`python.testing.cwd`, options in `python.testing.pytestArgs`, `rust-analyzer.linkedProjects`);
- sundgren's own [check](reference.md#checking-before-a-run) fails: the patch doesn't apply, a
  test tool is missing, or the tests would import the project from your checkout instead of the
  temporary one. The last happens with a `src/` layout and an editable install; set pytest's
  `pythonpath` option to `src` under `[tool.pytest.ini_options]`;
- the plucked Python test still ran with the skip marker, for example because a decorator wraps
  it without `functools.wraps` and hides the marker from pytest, so the result would be
  meaningless;
- the sundgren it found (through `sundgren.path`, the project's `.venv` or `PATH`) is too old for
  the extension.

## Settings

- `sundgren.path`, in user settings only, not per project: a sundgren program to use instead of
  the bundled one.
