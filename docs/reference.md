---
orphan: true
---

# Reference

sundgren has four commands: `run`, `diff`, `presets` and `ingest`. Long options can be shortened
to any unique prefix, for example `--keep` for `--keep-worktrees`. [How it works](how-it-works.md)
explains what the commands do.

## run

Runs coverage on the base tree and on the patched tree, then compares them. Run it inside the
project's git repository.

| Option | Default | Meaning |
| --- | --- | --- |
| `--patch` FILE | required | The patch to measure. |
| `--preset` NAME | picked from the repo | The preset to run: `pytest` if the repo has a `pyproject.toml`, `setup.cfg` or `pytest.ini`, `cargo-llvm-cov` if it has a `Cargo.toml`. With both or neither, pass `--preset` or `--cmd`. |
| `--cmd` CMD | | An ad-hoc command instead of a preset. Repeat it for several commands. Needs `--lcov`. |
| `--lcov` TEMPLATE | | With `--cmd`: where the commands write the lcov file. |
| `--junit` TEMPLATE | | With `--cmd`: where the commands write JUnit XML, if they do. |
| `--shell` | off | With `--cmd`: run the commands through `/bin/sh` instead of splitting them into arguments. |
| `--repo` DIR | `.` | The repository. Any directory inside it works. |
| `--base` REF | `HEAD` | The commit both trees start from. |
| `--tests` T... | | The tests to run, overriding the preset's selection. Paths for `pytest`, name filters for the Rust presets. |
| `--python` PATH | `python` on `PATH`, else `python3` | The value of `{python}`. |
| `--env` K=V | | Set an environment variable for the commands. Repeatable. The value can use placeholders. |
| `--repeat` N | 1 | Run each side N times. Lines whose status differs between runs are reported as `flaky`. |
| `--workdir` DIR | a temporary directory | Where the worktrees and run outputs go. The outputs are kept. |
| `--keep-worktrees` | off | Keep the two worktrees after the run, for debugging. |

The [output options](#output-options) apply too.

Any tool that writes lcov works with `--cmd`, for example Jest with its lcov reporter: pass the
test command with `--cmd` and where it writes the lcov file with `--lcov`, usually under `{out}`.

## diff

Compares two lcov files you already have. Nothing is run.

| Argument | Meaning |
| --- | --- |
| BASE.lcov, PATCHED.lcov | The two lcov files. |
| `--patch` FILE | The patch between the two trees, for mapping line numbers. Without it, lines are compared by number. |
| `--base-root` DIR, `--patched-root` DIR | The source trees the lcov files were measured in. Their paths are stripped from the lcov paths, and their files are used to find files missing from the coverage. |
| `--strip-prefix` PREFIX... | More path prefixes to strip, for lcov files recorded elsewhere (for example on CI). |
| `--base-junit` FILE, `--patched-junit` FILE | JUnit XML for each side, to compare test outcomes. |

The [output options](#output-options) apply too.

## Output options

For `run` and `diff`.

| Option | Default | Meaning |
| --- | --- | --- |
| `--format` json\|text | json | A JSON report, or a summary for the terminal. |
| `-o`, `--output` OUT | stdout | Write the report to a file. |
| `--full` | off | List unchanged lines, branches, functions and test outcomes too. |
| `--counts` | off | Also report hit-count changes on lines that stayed hit (`files[].count_changed`). |

## presets

Lists the built-in presets and the repo's own, with where each is defined, its test style, its
selector and its commands.

| Option | Default | Meaning |
| --- | --- | --- |
| `--repo` DIR | `.` | The repository whose presets to include. |

## ingest

Prints one lcov file as normalized JSON, as sundgren reads it. For debugging.

| Argument | Meaning |
| --- | --- |
| FILE.lcov | The lcov file. |
| `--root` DIR | A path to strip from the lcov paths. |

## Presets

A preset is a list of commands plus where they write their output.

| Preset | Runs |
| --- | --- |
| `pytest` | coverage.py with branch coverage around pytest, then coverage.py's lcov export. Writes JUnit, and runs only the touched test files. |
| `cargo-llvm-cov` | cargo-llvm-cov, with lines and functions. |
| `cargo-tarpaulin` | cargo-tarpaulin. |

Your own presets go in `sundgren.toml` as `[presets.NAME]` tables, or in `pyproject.toml` as
`[tool.sundgren.presets.NAME]` tables. A preset with a built-in's name replaces it. If both files
define the same name, `sundgren.toml` wins.

| Key | Required | Meaning |
| --- | --- | --- |
| `commands` | yes | The commands to run, in order, in the worktree. A non-empty list of strings. |
| `lcov` | yes | Where the commands write the lcov file. |
| `junit` | | Where the commands write JUnit XML. |
| `env` | | Environment variables for the commands, as a table of strings. |
| `description` | | Shown by the presets command. |
| `selector` | | How to pick tests from the patch. The only one is `python.pytest_files`. Without one, all tests run. |
| `tests_style` | | `args` (the default): tests are paths, and one missing on a side is dropped there. `filter`: tests are name filters, passed as they are. |
| `shell` | | `true` runs each command through `/bin/sh`. Default `false`. |
| `hints` | | Advice printed by the text report when there are warnings, and with the wrong-tree error. |

Paths in `lcov` and `junit` are relative to the worktree. Unknown keys are an error.

## Placeholders

Commands, the `lcov` and `junit` paths and environment values can use these:

| Placeholder | Value |
| --- | --- |
| `{python}` | The Python interpreter (`--python`). |
| `{worktree}` | The worktree the commands run in. |
| `{out}` | This run's output directory. |
| `{lcov}` | The lcov path. |
| `{junit}` | The JUnit path, or empty. |
| `{tests}` | The selected tests, or nothing when all tests run. |

A command argument that is exactly `{tests}` becomes one argument per test. Inside a longer
argument the tests are joined with spaces. With `shell`, each test is shell-quoted.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Done. Test failures don't change the exit code. |
| 1 | A run or tool failure: no lcov output, a malformed lcov file, a git error, the wrong-tree check. |
| 2 | A usage error: bad arguments, a missing input file, an unknown preset, a bad base ref, a malformed presets file. |
| 3 | The patch doesn't apply. |

## Versioning

sundgren follows [SemVer](https://semver.org/). The public interface is the command line
(commands, options, exit codes), the report schema, the preset format and the built-in preset
names. Before 1.0, a breaking change bumps the minor version and anything else the patch version.
The report schema has its own version, `sundgren.delta/N`, which goes up on a breaking schema
change.

## Warnings

The report's `warnings` list has a `code` and a `message` for each.

| Code | Meaning |
| --- | --- |
| `dirty_tree` | Your checkout has uncommitted changes. They are not part of the base. |
| `patch` | Something in the patch was read loosely, for example a copy treated as a rename. |
| `patch_offset` | git applied a hunk away from the line in its header. The line maps follow what git did. |
| `nothing_to_run` | None of the selected tests exist on one side, so that side's coverage is empty. |
| `junit_unreadable` | A JUnit file couldn't be parsed, so test outcomes are left out. |
| `inferred_file` | A file is missing from one side's coverage but exists in its tree, so its lines count as missed. |
| `unmeasured` | Some lines are executable on one side only, or the file wasn't measured. |
| `unpaired_file` | A base file has the same path as a file already paired through the patch, and is left out. |
| `tests_one_side` | Test outcomes exist for one side only, so none are compared. |
