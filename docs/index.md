# sundgren

<p class="sg-hero"><img class="sg-hero-light" src="_static/brand/sundgren-logo.svg" alt="" width="420"><img class="sg-hero-dark" src="_static/brand/sundgren-logo-dark.svg" alt="" width="420"></p>

Given a repo, a base ref and a patch, usually one that changes tests, sundgren answers: **what
happened to source coverage because of this patch?** It runs a coverage command on the base tree
and on the patched tree, diffs the two lcov files with line numbers mapped through the patch, and
reports the lines, branches, functions and test outcomes that changed.

Install it from [PyPI](https://pypi.org/project/sundgren/). [How it works](how-it-works.md)
explains the steps, and the [reference](reference.md) lists every option, preset key, exit code and
warning.

> [!NOTE]
> Each example below is a small project, a patch and the command. The docs tests rerun the command
> and compare its output with the files shown here, and the graphic is drawn from the recorded
> JSON report.

## Python: skipping a test

### Project

```{literalinclude} examples/skip_divide/project/shed/calc.py
:caption: shed/calc.py
:language: python
```

```{literalinclude} examples/skip_divide/project/tests/test_calc.py
:caption: tests/test_calc.py
:language: python
```

```{literalinclude} examples/skip_divide/project/.coveragerc
:caption: .coveragerc
:language: ini
```

```{literalinclude} examples/skip_divide/project/pyproject.toml
:caption: pyproject.toml
:language: toml
```

### Patch

The patch skips `test_divide`, the only test that calls `divide`.

```{literalinclude} examples/skip_divide/skip_divide.patch
:caption: skip_divide.patch
:language: diff
```

### Command

Run in the project's git repo, with the patch file at its root. The `pytest` preset is picked from
`pyproject.toml`.

```{literalinclude} examples/skip_divide/commands.sh
:language: sh
```

### Result

The body of `divide` and both branches of its `if` go from hit to miss, so the function went dark.
Its `def` line stays hit, because it runs at import.

```{sundgren-graphic} examples/skip_divide
```

### Text report

```{literalinclude} examples/skip_divide/report.txt
:language: text
```

### JSON report

Four fields in `meta` are normalized so the recording is stable: `created_utc`,
`sundgren_version`, the lcov hashes in `inputs`, and the Python path in `commands`.

````{details} report.json
```{literalinclude} examples/skip_divide/report.json
:language: json
```
````

## Rust: ignoring a test

### Project

```{literalinclude} examples/ignore_divide_rs/project/src/lib.rs
:caption: src/lib.rs
:language: rust
```

```{literalinclude} examples/ignore_divide_rs/project/Cargo.toml
:caption: Cargo.toml
:language: toml
```

### Patch

The patch marks `test_divide`, the only test that calls `divide`, with `#[ignore]`.

```{literalinclude} examples/ignore_divide_rs/ignore_divide.patch
:caption: ignore_divide.patch
:language: diff
```

### Command

Run in the project's git repo, with the patch file at its root.

```{literalinclude} examples/ignore_divide_rs/commands.sh
:language: sh
```

The recording was made with the rustc and cargo-tarpaulin versions pinned in the example's
manifest. Other versions can change the hit counts.

```{literalinclude} examples/ignore_divide_rs/example.toml
:caption: example.toml
:language: toml
```

### Result

tarpaulin records no branches or test outcomes and leaves the `#[cfg(test)]` module out. As with
a Python `def`, the signature line of `divide` stays hit, so sundgren judges `divide` by its body.

```{sundgren-graphic} examples/ignore_divide_rs
```

### Text report

```{literalinclude} examples/ignore_divide_rs/report.txt
:language: text
```

### JSON report

The same normalization applies.

````{details} report.json
```{literalinclude} examples/ignore_divide_rs/report.json
:language: json
```
````
