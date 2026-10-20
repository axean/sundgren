---
orphan: true
---

# VS Code

The sundgren extension puts **sg: pluck this test** above each test in a Python (pytest) or Rust
file. Clicking it skips that one test and shows what happens to coverage: which lines, branches
and functions are no longer run without it. Plucking is a simple example of what sundgren can do,
for example finding unit tests that are redundant.

**Contents**

- [Install](#install)
- [Pluck a test](#pluck-a-test)
- [What a pluck does](#what-a-pluck-does)
- [Reading the result](#reading-the-result)
- [Cancel](#cancel)
- [When it refuses](#when-it-refuses)
- [Settings](#settings)
- [Limitations](#limitations)

## Install

Each [release on GitHub](https://github.com/axean/sundgren/releases) has a `.vsix` file per
platform, with the sundgren program for that platform inside: `linux-x64`, `linux-arm64`,
`darwin-x64` and `darwin-arm64`. Download the one for your machine, then in VS Code open the
Extensions view, choose **Install from VSIX…** from its `…` menu and pick the file.

The project needs its own test tools:

- Python: pytest and coverage.py in the interpreter the project uses.
- Rust: cargo-llvm-cov.

> [!NOTE]
> The extension isn't on the VS Code Marketplace or Open VSX yet, and Windows isn't supported.

## Pluck a test

1. Commit your work. sundgren compares against the last commit, so the extension refuses while
   tracked files have uncommitted changes.
2. Click **sg: pluck this test** above the test. The test fades while its pluck runs.
3. The first time in a project, the extension shows what it will run (the preset, the Python
   interpreter and the commands) and asks you to confirm. It asks again if any of that changes.
4. A notification shows each step. When the run is done it says what was lost, for example
   "Plucking test_divide: 3 lines lost, 2 branches lost, divide is no longer run (coverage 100% →
   75%)". The lost lines are highlighted in the editor, and the details are in the **sundgren**
   output panel.

Your files are never changed: the pluck runs in temporary checkouts.

## What a pluck does

1. **Checks.** Before anything runs, the extension checks everything listed under
   [when it refuses](#when-it-refuses), and stops at the first problem.
2. **The patch.** It writes a patch that skips the test, against the last commit: `#[ignore]` in
   Rust, `@pytest.mark.skip` in Python (above any decorators, importing pytest if the file doesn't
   already).
3. **sundgren's check.** It runs sundgren's [check](reference.md#checking-before-a-run) with
   the patch: the patch applies, the test tools are installed, and the tests would import the
   temporary checkout's code. Nothing runs yet.
4. **The run.** It runs sundgren's `run`, pinned to the commit the checks used: the whole test
   suite on the last commit, then again with the test skipped, each in its own git worktree.
5. **The result.** It reads sundgren's JSON report and shows what changed.

A Rust test runs with the `cargo-llvm-cov` preset, a Python test with the `pytest` preset. The
Python interpreter is the one selected in Microsoft's Python extension, else the project's
`.venv`, else `python` on `PATH`.

> [!NOTE]
> The suite runs twice from fresh checkouts, so a pluck takes about twice as long as your test
> suite. In Rust each side also builds from scratch in its own target directory.

## Reading the result

The headline counts the lines and branches lost and names the functions that are no longer run.
The output panel lists the lost lines per file.

> [!NOTE]
> The plucked test's own lines aren't counted. A skipped test trivially loses its own body, and
> in Rust coverage includes test code. Its lines, its closures and its function are left out of
> the counts and the highlights. The coverage percentage isn't shown then, since it would include
> them.

> [!WARNING]
> "Loses no coverage" means every line still runs without the test, not that the test is
> useless. Another test may run the same lines without checking the same results.

For Python, the extension also checks that the test was really skipped in the second run. If a
decorator hides the skip marker from pytest, the test still runs and the result would be
meaningless, so the extension refuses instead.

## Cancel

While a pluck runs, cancel it with the notification's **Cancel** button, or by clicking the line
above the test, which then reads **sg: plucking… (click to cancel)**.

Cancel stops sundgren and every process it started (the test commands, and in Rust cargo and the
build), and removes the temporary checkouts. It usually takes well under a second, also in the
middle of a Rust build. If sundgren hasn't stopped after 5 seconds, the extension kills it.

## When it refuses

The extension refuses rather than guess, with one sentence saying what to do. It refuses when:

- the workspace isn't trusted, since running a test runs the project's code;
- the file has unsaved changes;
- the file isn't in a git repository, the repository has no commits, or a merge, rebase,
  cherry-pick, revert or bisect is in progress;
- tracked files have uncommitted changes;
- the file isn't committed, or differs from the last commit;
- the project isn't at the repository root (a `pyproject.toml`, `setup.cfg` or `pytest.ini` for
  Python, a `Cargo.toml` for Rust), or VS Code's test settings point elsewhere
  (`python.testing.cwd`, options in `python.testing.pytestArgs`, `rust-analyzer.linkedProjects`);
- sundgren's own check fails: the patch doesn't apply, a test tool is missing, or the tests would
  import the project from your checkout instead of the temporary one;
- the plucked Python test still ran with the skip marker;
- the sundgren it found (through `sundgren.path`, the project's `.venv` or `PATH`) is too old for
  the extension.

> [!TIP]
> The import problem happens with a `src/` layout and an editable install: both runs would import
> your checkout and measure the same code. Set pytest's `pythonpath` option to `src` under
> `[tool.pytest.ini_options]`.

## Settings

- `sundgren.path`, in user settings only, not per project: a sundgren program to use instead of
  the bundled one.

## Limitations

- Only files named `test_*.py` or `*_test.py` get a lens. pytest's `python_files` setting isn't
  read.
- Projects in a subfolder of the repository are refused.
- Rust reports have no test outcomes (cargo-llvm-cov doesn't give any), so a Rust pluck can't
  show which tests ran.
- macOS isn't tested in CI. There, if sundgren doesn't stop and has to be killed, the test
  processes it started can keep running. On Linux they're killed with it.
