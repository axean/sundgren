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
| `--patch` FILE | none | The patch to measure. Without it, both trees are the base and the whole suite runs on each: a check that the tests run and give the same coverage twice. |
| `--preset` NAME | picked from the repo | The preset to run: `pytest` if the repo has a `pyproject.toml`, `setup.cfg` or `pytest.ini`, `cargo-llvm-cov` if it has a `Cargo.toml`. With both or neither, pass `--preset` or `--cmd`. |
| `--cmd` CMD | | An ad-hoc command instead of a preset. Repeat it for several commands. Needs `--lcov`. |
| `--lcov` TEMPLATE | | With `--cmd`: where the commands write the lcov file. |
| `--junit` TEMPLATE | | With `--cmd`: where the commands write JUnit XML, if they do. |
| `--shell` | off | With `--cmd`: run the commands through `/bin/sh` instead of splitting them into arguments. |
| `--repo` DIR | `.` | The repository. Any directory inside it works. |
| `--base` REF | `HEAD` | The commit both trees start from. |
| `--tests` T... | | The tests to run, overriding the preset's selection: pytest node IDs (down to one case of a parametrized test), or name filters for the Rust presets. See [running only some tests](examples.md#running-only-some-tests). |
| `--all-tests` | off | Run all tests, ignoring the preset's selector. Can't be combined with `--tests`. |
| `--python` PATH | `python` on `PATH`, else `python3` | The value of `{python}`. |
| `--env` K=V | | Set an environment variable for the commands. Repeatable. The value can use placeholders. |
| `--repeat` N | 1 | Run each side N times. Lines whose status differs between runs are reported as `flaky`. |
| `--workdir` DIR | a temporary directory | Where the worktrees and run outputs go. The outputs are kept. |
| `--keep-worktrees` | off | Keep the two worktrees after the run, for debugging. |
| `--silent` | off | Don't print progress. By default `run` prints a line to stderr as each step starts and ends, with how long each test run took. The report goes to stdout or `-o` either way. |
| `--check` | off | Check that the run can start, without running any tests. See [checking before a run](#checking-before-a-run). |

The [output options](#output-options) apply too.

Any tool that writes lcov works with `--cmd`, for example Jest with its lcov reporter: pass the
test command with `--cmd` and where it writes the lcov file with `--lcov`, usually under `{out}`.

SIGINT (Ctrl-C) or SIGTERM stops a run at any point: sundgren stops the running commands,
removes the worktrees and exits with 130 or 143. See [cancelling](how-it-works.md#cancelling).

## Checking before a run

With `--check`, `run` does everything up to running the tests, then stops: it resolves the
repository and the base, checks that the checkout has no uncommitted changes to tracked files,
applies the patch in a temporary worktree (removed again), picks the preset and the tests, and
runs the preset's `checks`. Nothing is left behind. It takes the same options as `run` and always
prints JSON (to `-o` if given), with the schema id `sundgren.check/1`
([schema/check.schema.json](https://github.com/axean/sundgren/blob/main/schema/check.schema.json)):

| Key | Value |
| --- | --- |
| `schema` | `sundgren.check/1` |
| `ok` | `true` when there are no problems. |
| `repo` | The repository root, or null. |
| `base_ref` | The full sha of the base, or null. |
| `preset` | The preset's name (`adhoc` for `--cmd`), or null. |
| `python` | The value of `{python}`, or null if the preset doesn't use it. |
| `commands` | The commands as the report shows them, or null. |
| `selection` | The tests to run, as in the report's `meta.selection`, or null. |
| `problems` | A list of `code` and `message`. Each message is one sentence for the user. |

Every problem found is listed; the independent checks all run.

| Code | Meaning |
| --- | --- |
| `usage` | A bad argument. |
| `not_a_repo` | `--repo` isn't inside a git repository. |
| `patch_not_found` | The patch file doesn't exist. |
| `bad_base` | The base ref isn't a commit. |
| `preset` | An unknown or malformed preset, or no default preset for this repo. |
| `dirty_tree` | The checkout has uncommitted changes to tracked files. |
| `patch_does_not_apply` | git can't apply the patch to the base. |
| `check_failed` | One of the preset's `checks` didn't exit 0, or its program wasn't found. |
| `wrong_tree` | The tests would import the project from your main checkout instead of the temporary one, for example through an editable install, so both runs would measure the same code. Checked for presets that run pytest with `{python}`: sundgren collects the tests (without running them) and looks at where the project's packages come from. |
| `error` | Anything else. |

The exit code is 0 when `ok` is true. Otherwise it is the one `run` would exit with: 2 if any
problem is a usage problem (`usage`, `not_a_repo`, `patch_not_found`, `bad_base`, `preset`), else
3 if the patch doesn't apply, else 1.

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
| `pytest` | coverage.py with branch coverage around pytest, then coverage.py's lcov export. Writes JUnit, and runs only the touched test files. Its check imports coverage and pytest with `{python}`. |
| `cargo-llvm-cov` | cargo-llvm-cov, with lines and functions. Its check asks cargo-llvm-cov for its version. |
| `cargo-tarpaulin` | cargo-tarpaulin. Its check asks cargo-tarpaulin for its version. |

Your own presets go in `sundgren.toml` as `[presets.NAME]` tables, or in `pyproject.toml` as
`[tool.sundgren.presets.NAME]` tables. A preset with a built-in's name replaces it. If both files
define the same name, `sundgren.toml` wins.

| Key | Required | Meaning |
| --- | --- | --- |
| `commands` | yes | The commands to run, in order, in the worktree. A non-empty list of strings. |
| `lcov` | yes | Where the commands write the lcov file. |
| `junit` | | Where the commands write JUnit XML. |
| `checks` | | Commands that must exit 0 for [`--check`](#checking-before-a-run) to pass, for example a version query of the tool. They run in the repository root, with `env`. A list of strings. |
| `env` | | Environment variables for the commands, as a table of strings. |
| `description` | | Shown by the presets command. |
| `selector` | | How to pick tests from the patch. The only one is `python.pytest_files`. Without one, all tests run. |
| `tests_style` | | `args` (the default): tests are paths, and one missing on a side is dropped there. `filter`: tests are name filters, passed as they are. |
| `shell` | | `true` runs each command through `/bin/sh`. Default `false`. |
| `hints` | | Advice printed by the text report when there are warnings, and with the wrong-tree error. |

Paths in `lcov` and `junit` are relative to the worktree. Unknown keys are an error.

## Placeholders

Placeholders are words in braces that sundgren fills in just before it runs a command, separately
for each side (base and patched) and each run. They let one command line work in both worktrees
and keep the two sides' output files apart. [This example](examples.md#your-own-commands-placeholders)
walks through a real run.

They can be used in commands (presets and `--cmd`), in the `lcov` and `junit` paths, and in
environment values (presets and `--env`):

| Placeholder | Value |
| --- | --- |
| `{python}` | `python` on your `PATH`, else `python3`, or the value of `--python`. |
| `{worktree}` | The worktree the commands run in (they already run inside it). |
| `{out}` | A fresh output directory for this side and run, inside sundgren's work directory. |
| `{lcov}` | The lcov path: the preset's `lcov` or `--lcov`, with its own placeholders filled in. |
| `{junit}` | The JUnit path, filled in the same way, or empty. |
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
| 130 | Cancelled by SIGINT (Ctrl-C). |
| 143 | Cancelled by SIGTERM. |

## Versioning

sundgren follows [SemVer](https://semver.org/). The public interface is the command line
(commands, options, exit codes), the report schema, the preset format and the built-in preset
names. Before 1.0, a breaking change bumps the minor version and anything else the patch version.
The report schema has its own version, `sundgren.delta/N`, which goes up on a breaking schema
change, and so does the output of `--check`, `sundgren.check/N`.

## Warnings

The report's `warnings` list has a `code` and a `message` for each.

| Code | Meaning |
| --- | --- |
| `no_patch` | `run` without `--patch`: both trees are the base. Only meaningful as a check that the tests run and give the same coverage twice; it says nothing about any change. |
| `dirty_tree` | Your checkout has uncommitted changes. They are not part of the base. |
| `patch` | Something in the patch was read loosely, for example a copy treated as a rename. |
| `patch_offset` | git applied a hunk away from the line in its header. The line maps follow what git did. |
| `nothing_to_run` | None of the selected tests exist on one side, so that side's coverage is empty. |
| `junit_unreadable` | A JUnit file couldn't be parsed, so test outcomes are left out. |
| `inferred_file` | A file is missing from one side's coverage but exists in its tree, so its lines count as missed. |
| `unmeasured` | Some lines are executable on one side only, or the file wasn't measured. |
| `unpaired_file` | A base file has the same path as a file already paired through the patch, and is left out. |
| `tests_one_side` | Test outcomes exist for one side only, so none are compared. |
