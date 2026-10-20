---
orphan: true
---

# Examples

Each example is a small project, a patch and the command that measures it. The docs tests rerun
every command and compare its output with the files shown here, and each graphic is drawn from
the recorded JSON report.

**Contents**

- [Python](#python)
  - [A patch that only touches tests](#a-patch-that-only-touches-tests)
  - [A patch that changes source code](#a-patch-that-changes-source-code)
  - [A patch that changes a fixture](#a-patch-that-changes-a-fixture)
  - [Running only some tests](#running-only-some-tests)
  - [Your own commands: placeholders](#your-own-commands-placeholders)
- [Rust](#rust)
  - [A patch that only touches tests](#a-patch-that-only-touches-tests-1)
  - [A patch that changes source code](#a-patch-that-changes-source-code-1)
  - [A patch that changes a test helper](#a-patch-that-changes-a-test-helper)
  - [Running only some tests](#running-only-some-tests-1)

> [!NOTE]
> Each graphic shows the counts that changed, line coverage before and after, the tests whose
> outcome changed, and the changed file as a diff with every line marked by its category. A legend
> under it explains the marks; [How it works](how-it-works.md#4-comparing) defines the categories.

## Python

All Python examples use the `pytest` preset, which sundgren picks because the project has a
`pyproject.toml`. The project is a small module with three functions and their tests.

### A patch that only touches tests

The patch skips `test_divide`, the only test that calls `divide`.

```{literalinclude} examples/skip_divide/project/shed/calc.py
:caption: shed/calc.py
:language: python
```

```{literalinclude} examples/skip_divide/project/tests/test_calc.py
:caption: tests/test_calc.py
:language: python
```

````{details} .coveragerc and pyproject.toml
```{literalinclude} examples/skip_divide/project/.coveragerc
:caption: .coveragerc
:language: ini
```

```{literalinclude} examples/skip_divide/project/pyproject.toml
:caption: pyproject.toml
:language: toml
```
````

```{literalinclude} examples/skip_divide/skip_divide.patch
:caption: skip_divide.patch
:language: diff
```

Run in the project's git repo, with the patch file at its root:

```{literalinclude} examples/skip_divide/commands.sh
:language: sh
```

> [!TIP]
> The patch only touches a test file, so the `pytest` preset runs just that file on both sides,
> not the whole suite.

The body of `divide` and both branches of its `if` go from hit to miss, so the function went dark.
Its `def` line stays hit, because it runs at import.

```{sundgren-graphic} examples/skip_divide
```

```{literalinclude} examples/skip_divide/report.txt
:caption: report.txt
:language: text
```

Four fields in `meta` are normalized so the recording is stable: `created_utc`,
`sundgren_version`, the lcov hashes in `inputs`, and the Python path in `commands`.

````{details} report.json
```{literalinclude} examples/skip_divide/report.json
:language: json
```
````

### A patch that changes source code

Same project. The patch adds a branch to `classify` for numbers over 100, without a test for it.

```{literalinclude} examples/py_source_change/big.patch
:caption: big.patch
:language: diff
```

```{literalinclude} examples/py_source_change/commands.sh
:language: sh
```

> [!NOTE]
> The patch changes `shed/calc.py`, which is not a test file, so the whole suite runs on both
> sides. sundgren can't know which tests reach which code, so any source change runs everything.

The new `if` line runs on every call of `classify`, so it is `new_hit`. The line returning `"big"`
under it never runs, so it is `new_miss`, and its two branches are new. Nothing that was there
before changed, so no line is lost: coverage drops only because of the new, untested line.

```{sundgren-graphic} examples/py_source_change
```

```{literalinclude} examples/py_source_change/report.txt
:caption: report.txt
:language: text
```

### A patch that changes a fixture

Here the test cases come from a fixture in `conftest.py`. The patch drops the negative sample.

```{literalinclude} examples/py_fixture_change/project/tests/conftest.py
:caption: tests/conftest.py
:language: python
```

```{literalinclude} examples/py_fixture_change/project/tests/test_calc.py
:caption: tests/test_calc.py
:language: python
```

```{literalinclude} examples/py_fixture_change/drop_negative.patch
:caption: drop_negative.patch
:language: diff
```

```{literalinclude} examples/py_fixture_change/commands.sh
:language: sh
```

> [!NOTE]
> A fixture can be used by any test, so a change to `conftest.py` runs the whole suite, like a
> source change.

No test calls `classify` with a negative number any more, so the line returning `"neg"` and its
branch are lost. The rest of `classify` still runs, so the function is a `partial_loss`, not
`went_dark`.

```{sundgren-graphic} examples/py_fixture_change
```

```{literalinclude} examples/py_fixture_change/report.txt
:caption: report.txt
:language: text
```

### Running only some tests

The same source change as above, but measured with only one case of the parametrized
`test_classify`. `--tests` overrides the preset's choice of tests.

```{literalinclude} examples/py_one_param/commands.sh
:language: sh
```

> [!TIP]
> **Choosing tests with `--tests` in pytest.** Each value is a pytest node ID, so it can be as
> narrow as you like:
>
> | To run | Pass |
> | --- | --- |
> | a test file | `tests/test_calc.py` |
> | one test | `tests/test_calc.py::test_classify` |
> | one case of a parametrized test | `tests/test_calc.py::test_classify[5-pos]` |
> | a test in a class | `tests/test_x.py::TestX::test_y` |
>
> List several values to run several tests. A parametrized case's ID is its parameter values
> joined by `-`; pytest's `--collect-only` option lists every ID.

> [!WARNING]
> Quote IDs that contain `[` in your shell, as above. Otherwise the shell may treat them as file
> name patterns.

Only `test_classify[5-pos]` runs, so `add`, `divide` and most of `classify` count as missed on both
sides, and the coverage is 50% before and after. The change itself shows the same as before: the
new `if` line is hit by `5`, and the line returning `"big"` is not.

```{sundgren-graphic} examples/py_one_param
```

```{literalinclude} examples/py_one_param/report.txt
:caption: report.txt
:language: text
```

### Your own commands: placeholders

Instead of a preset, `--cmd` runs your own commands; repeat it for several commands. `--lcov`
says where they write the lcov file. This example does by hand what the `pytest` preset does,
on the `skip_divide` patch from the first example:

```{literalinclude} examples/py_custom_cmd/commands.sh
:language: sh
```

The words in braces are **placeholders**. sundgren fills them in separately for each side and
each run, just before it runs the command:

| Placeholder | Becomes | In this example |
| --- | --- | --- |
| `{python}` | `python` on your `PATH` (or `--python`) | the interpreter that runs coverage.py and pytest |
| `{tests}` | the selected tests, one argument each | nothing: `--cmd` has no test selection, so all tests run |
| `{out}` | a fresh output directory for this side and run | where coverage.py keeps its data file and the lcov file goes |
| `{lcov}` | the `--lcov` value, with its placeholders filled in | `{out}/coverage.lcov`, so the second command writes the file sundgren reads |
| `{worktree}` | the checkout the commands run in | not needed: commands already run inside it |
| `{junit}` | the `--junit` value, or empty | not used |

So on the base side, `{out}` is a directory like `out/base/run-1` inside sundgren's work directory,
and the second command writes `out/base/run-1/coverage.lcov`. On the patched side the same
commands write to `out/patched/run-1`. Every side and run gets its own `{out}`, so the two sides
never overwrite each other's files.

> [!TIP]
> - `{worktree}` is mostly useful in `--env` values. For example, if your package is installed in
>   editable mode, set `PYTHONPATH` to `{worktree}/src` so each side imports its own code.
> - `{tests}` on its own expands to one argument per test, or to nothing. Inside a longer argument,
>   the tests are joined with spaces.
> - Placeholders work the same in a preset's `commands`, `lcov`, `junit` and `env` values. See the
>   [reference](reference.md#placeholders).

```{sundgren-graphic} examples/py_custom_cmd
```

```{literalinclude} examples/py_custom_cmd/report.txt
:caption: report.txt
:language: text
```

## Rust

The Rust examples use the `cargo-tarpaulin` preset. The project is the same three functions in
Rust, with unit tests in a `tests` module.

> [!NOTE]
> The Rust presets have no test selection: unless you pass `--tests`, every patch runs all tests.

### A patch that only touches tests

The patch marks `test_divide`, the only test that calls `divide`, with `#[ignore]`.

```{literalinclude} examples/ignore_divide_rs/project/src/lib.rs
:caption: src/lib.rs
:language: rust
```

```{literalinclude} examples/ignore_divide_rs/project/Cargo.toml
:caption: Cargo.toml
:language: toml
```

```{literalinclude} examples/ignore_divide_rs/ignore_divide.patch
:caption: ignore_divide.patch
:language: diff
```

```{literalinclude} examples/ignore_divide_rs/commands.sh
:language: sh
```

> [!IMPORTANT]
> The recording was made with the rustc and cargo-tarpaulin versions pinned in the example's
> manifest. Other versions can change the hit counts.
>
> ```{literalinclude} examples/ignore_divide_rs/example.toml
> :caption: example.toml
> :language: toml
> ```

tarpaulin records no branches or test outcomes and leaves the `#[cfg(test)]` module out. As with
a Python `def`, the signature line of `divide` stays hit, so sundgren judges `divide` by its body.

```{sundgren-graphic} examples/ignore_divide_rs
```

```{literalinclude} examples/ignore_divide_rs/report.txt
:caption: report.txt
:language: text
```

````{details} report.json
```{literalinclude} examples/ignore_divide_rs/report.json
:language: json
```
````

### A patch that changes source code

Same project. The patch adds a branch to `classify` for numbers over 100, without a test for it.

```{literalinclude} examples/rs_source_change/add_big.patch
:caption: add_big.patch
:language: diff
```

```{literalinclude} examples/rs_source_change/commands.sh
:language: sh
```

The new condition is checked on every call, so its line is `new_hit`. The line returning `"big"`
never runs, so it is `new_miss`. tarpaulin records no branches, so none are reported, and
`classify` keeps its status: an untested new line is not a loss.

```{sundgren-graphic} examples/rs_source_change
```

```{literalinclude} examples/rs_source_change/report.txt
:caption: report.txt
:language: text
```

### A patch that changes a test helper

Rust has no fixtures; shared test data usually comes from a helper function in the tests module.
Here `test_classify` loops over the cases from `samples()`, and the patch drops the negative one.

```{literalinclude} examples/rs_helper_change/project/src/lib.rs
:caption: src/lib.rs
:language: rust
```

```{literalinclude} examples/rs_helper_change/drop_neg_sample.patch
:caption: drop_neg_sample.patch
:language: diff
```

```{literalinclude} examples/rs_helper_change/commands.sh
:language: sh
```

No test reaches the line returning `"neg"` any more, so it is lost and `classify` is a
`partial_loss`. The helper itself doesn't appear in the report: tarpaulin leaves the
`#[cfg(test)]` module out.

```{sundgren-graphic} examples/rs_helper_change
```

```{literalinclude} examples/rs_helper_change/report.txt
:caption: report.txt
:language: text
```

### Running only some tests

The same source change, measured with only `test_classify`:

```{literalinclude} examples/rs_filter/commands.sh
:language: sh
```

> [!TIP]
> **Choosing tests with `--tests` in Rust.** Each value is a test name filter, passed to the
> test runner after `--`, as with cargo's own test command. The smallest unit is one test function.
>
> | To run | Pass |
> | --- | --- |
> | every test whose name contains a word | `classify` |
> | one test, by its path | `tests::test_classify` |
> | several tests | several values, for example `test_classify` and `test_divide` |
> | exactly one name, not every name containing it | the full path, then `--tests=--exact` after it |
> | everything except a test | `--tests=--skip` followed by `--tests=test_divide` |
>
> Runner options like `--exact` must be attached with `=`, as shown, or sundgren reads them as
> its own options.

> [!WARNING]
> A filter that matches nothing runs no tests, and sundgren gives no warning. The report then
> shows almost nothing covered on that side. Try the filter with cargo's test command first.

Only `test_classify` runs, so `divide`'s body counts as missed on both sides, and coverage is low
before and after. The change itself shows as in the full run: one new line hit, one missed.

```{sundgren-graphic} examples/rs_filter
```

```{literalinclude} examples/rs_filter/report.txt
:caption: report.txt
:language: text
```
