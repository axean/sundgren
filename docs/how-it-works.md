---
orphan: true
---

# How it works

`sundgren` runs your tests with coverage on the tree before the patch and on the tree after it,
then compares the two lcov files line by line, with line numbers mapped through the patch.

The `run` command does all of this. The `diff` command does only the comparison, for lcov files
you already have. Every option is listed in the [reference](reference.md).

## 1. Two worktrees

`sundgren` resolves the base ref (`--base`, default `HEAD`) to a commit and checks it out twice as
git worktrees, `base/` and `patched/`, in a temporary directory (or `--workdir`). It applies the
patch to `patched/` with git apply. Both worktrees are removed afterwards, unless
`--keep-worktrees` is given.

- Uncommitted changes in your checkout are not part of the base. If there are any, the report gets
  a `dirty_tree` warning.
- The line maps come from the diff git actually applied, not from the patch file. A hunk that git
  applied at an offset is mapped correctly, with a `patch_offset` warning.
- A patch that doesn't apply exits with code 3.

## 2. Picking the tests

The tests to run come from, in order:

1. `--tests`, if given (see [running only some tests](examples.md#running-only-some-tests));
2. the preset's selector;
3. otherwise, all tests.

The `pytest` preset's selector runs only the test files the patch touches. If the patch touches any
other Python file or a pytest config file (`pyproject.toml`, `setup.cfg`, `tox.ini`,
`pytest.ini`), or adds, deletes or renames a test file, it runs everything.

The same selection runs on both sides. With path-style tests, a test file missing on one side is
dropped for that side. If none is left, that side counts as empty coverage, with a
`nothing_to_run` warning.

## 3. Running the commands

The preset's commands run in each worktree in turn, with the worktree as the working directory and
no stdin. Placeholders such as `{out}` and `{lcov}` are filled in first (see
[placeholders](reference.md#placeholders)). Each run writes to its own directory with a log file.

- A command's exit code is only logged: failing tests are data, not an error.
- A side fails only when its lcov file is missing afterwards. `sundgren` then exits 1 and prints the
  end of the log.
- Rust presets give each side its own `CARGO_TARGET_DIR`. That's slower, but the two trees never
  share build output.
- With `--repeat`, the commands run N times per side. Lines whose status differs between runs are
  reported as `flaky` instead of as a change.

**Wrong-tree check.** If the lcov file names source files in your main checkout instead of the
worktree, the run measured the wrong code. This usually happens with an editable install. `sundgren`
stops with exit code 1 and lists the files. For Python, pass `--env` with
`PYTHONPATH={worktree}/src`, or set pytest's `pythonpath` option to `src`. With pytest,
`--check` finds this before anything runs (problem `wrong_tree`).

## 4. Comparing

**Paths.** lcov files name sources by absolute path. `sundgren` strips the worktree root (or the
`--base-root`/`--patched-root` and `--strip-prefix` values in `diff` mode) to get repo-relative
paths. Files are paired through the patch, so a renamed file is compared with its old self.

**Lines.** Every executable line gets a category:

| Category | Meaning |
| --- | --- |
| `lost` | hit before the patch, missed after |
| `gained` | missed before, hit after |
| `kept_hit`, `kept_miss` | unchanged line, same status on both sides |
| `new_hit`, `new_miss` | line added by the patch |
| `deleted_hit`, `deleted_miss` | line removed by the patch |
| `unmeasured` | executable on one side only, or the file wasn't measured |

A changed line counts as one deleted line plus one new line. A file missing from one side's lcov
but present in its tree is *inferred*: its lines count as executable and missed, with an
`inferred_file` warning. This is how never-imported files show up. For Python, it needs coverage.py
to know your package: set `source` in the `[run]` section of `.coveragerc`.

**Branches** match on line, block and branch id, with line numbers mapped through the patch. Their
categories are `branch_kept_hit`, `branch_kept_miss`, `branch_lost`, `branch_gained`, `branch_new`,
`branch_deleted`, `branch_flaky` and `branch_unmeasured`.

**Functions** come from the lcov function records (`FN`). A function is judged by its body only,
because its signature line runs at import or entry even when the body never does:

| Status | Meaning |
| --- | --- |
| `went_dark` | body hit before, nothing hit after |
| `lit_up` | body not hit before, hit after |
| `partial_loss`, `partial_gain` | some body lines lost, or gained |
| `mixed` | both lost and gained lines |
| `added`, `removed` | the function exists on one side only |
| `unchanged` | no change |

**Tests.** With JUnit output on both sides (the `pytest` preset writes it), the report lists each
test's outcome before and after, for example `passed` to `skipped`.

**Tree.** Line stats are rolled up per directory, for every file on either side.

## 5. The report

The default output is a JSON report (`sundgren.delta/1`, schema in
[schema/delta.schema.json](https://github.com/axean/sundgren/blob/main/schema/delta.schema.json)).
It lists only what changed: files with a non-kept line, branch or function, and changed test
outcomes. `--full` lists everything. `--format` with the value text prints a terminal summary instead. The summary
and the directory tree are always complete.

## Cancelling

SIGINT (Ctrl-C) or SIGTERM stops `run`, with or without `--check`, at any point, from creating the
worktrees to comparing:

- Each command, git included, runs in its own process group. `sundgren` sends the group SIGTERM,
  and SIGKILL after one second if anything is still running.
- The worktrees and the temporary directory are removed, also with `--keep-worktrees`. They are
  first renamed to a `sundgren-trash-*` directory next to them and deleted in the background, so
  a large build directory doesn't hold `sundgren` up.
- `sundgren` exits with 130 after SIGINT and 143 after SIGTERM, usually well within two seconds.

If `sundgren` itself is killed with SIGKILL, it can't clean up. On Linux, the commands it started
directly are killed with it, but the processes those started are not.

## Limitations

- Functions with the same name in one lcov file are merged into one record, because lcov keys
  functions by name.
- A one-line function can't be `went_dark`: its only line is the signature, which runs at import.
- Rust closures have function records with no end line, so the enclosing function is cut at the
  closure's start.
- A function record without an end line is taken to run until the next function starts.
