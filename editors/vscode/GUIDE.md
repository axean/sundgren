# Guide draft: the sundgren VS Code extension

Lasting, user-facing behaviour, to become a page on the docs site. Only what users see and do.

## What it does

Above each test in a Rust file or a pytest test file (`test_*.py`, `*_test.py`), the extension
shows **sg: pluck this test**. Clicking it answers: what happens to coverage if this test is
skipped? sundgren checks out the last commit twice, skips the test in one copy, runs the whole
test suite in both, and compares the coverage. Your files are never changed.

## Install

Install **sundgren** from the Extensions view. The extension contains the sundgren program for
your platform (Linux and macOS, x86_64 and arm64), so nothing else is needed for sundgren itself.
The project needs its own test tools:

- Python: pytest and coverage.py in the interpreter the project uses.
- Rust: cargo-llvm-cov.

Windows isn't supported yet.

## Pluck a test

1. Commit your work. sundgren compares against the last commit, so it refuses while there are
   uncommitted changes to tracked files.
2. Click **sg: pluck this test** above the test.
3. The first time in a project, the extension shows exactly what it will run (preset, Python
   interpreter, commands) and asks you to confirm. It asks again if any of that changes.
4. A notification shows the progress. When the run is done it says what was lost, for example
   "Plucking test_divide: 3 lines lost, 2 branches lost, divide went dark (coverage 100% → 75%)".
   The lost lines are highlighted in the editor, and the details are in the **sundgren** output
   panel.

Which Python is used: the interpreter selected in Microsoft's Python extension, else the
project's `.venv`, else `python` on `PATH`.

## Cancel

While a pluck runs, cancel it with the notification's **Cancel** button, or by clicking the line
above the test, which then reads **sg: plucking… (click to cancel)**. sundgren stops the test
processes and removes its temporary checkouts, usually within a second.

## When it refuses, and why

The extension checks everything before it runs anything, and stops at the first problem with one
sentence saying what to do. It refuses when:

- the workspace isn't trusted (running a test runs the project's code);
- the file has unsaved changes;
- the file isn't in a git repository, the repository has no commits, or a merge, rebase,
  cherry-pick, revert or bisect is in progress;
- there are uncommitted changes to tracked files;
- the file isn't committed, or differs from the last commit;
- the project isn't at the repository root (a `pyproject.toml`, `setup.cfg` or `pytest.ini` for
  Python, a `Cargo.toml` for Rust), or VS Code's test settings point elsewhere
  (`python.testing.cwd`, options in `python.testing.pytestArgs`, `rust-analyzer.linkedProjects`);
- sundgren's own check fails: the patch doesn't apply, or a test tool is missing;
- the sundgren it found (through `sundgren.path`, the project's `.venv` or `PATH`) is too old for
  the extension.

## Settings

- `sundgren.path` (user settings only, not per project): a sundgren program to use instead of the
  bundled one.
