# Docs examples

Every code block and output on the docs site is a file from here. The docs tests rerun each
example and compare the output with the recorded files. This directory is excluded from the
Sphinx build.

## Layout

```
<name>/
  example.toml   manifest
  commands.sh    the commands, one per line, exactly as shown on the page
  project/       the base tree, committed as the example repo
  <patch>        the patch, e.g. skip_divide.patch
  report.json    recorded output (normalized, see below)
  report.txt     recorded output
```

Nothing else may sit next to these files; `tests/docs/test_no_untested_code.py` checks this.

`example.toml`:

```toml
language = "python"                       # or "rust"
patch = "skip_divide.patch"
outputs = ["report.json", "report.txt"]   # files the commands write, recorded here
requires = [                              # optional: pinned tools, see "Rust" below
    { run = "rustc --version", expect = "rustc 1.99.1 " },
]
```

`commands.sh` is plain sh, but runs without a shell: each line is split with `shlex`, must start
with `sundgren` and must pass `--patch <patch>`. No `$`, comments or shell syntax. The commands
write their outputs with `-o`, so the page shows where each recorded file comes from.

## How an example runs

`tools/docs_examples.py`:

1. copies the files under `project/` that git tracks in the sundgren checkout (`git add` a new
   file before recording) to a temp dir and commits them with a fixed identity and date
   (2026-01-01T00:00:00Z) and no user or system git config, so `meta.base_ref` is the same
   everywhere;
2. copies the patch into the repo root, untracked;
3. runs each line in the repo, `sundgren` as `python -m sundgren` from the running interpreter, in
   a clean env: no `GIT_*`, `PYTEST_*`, `COVERAGE_*` or `PYTHONPATH`, a private `TMPDIR`, and the
   running interpreter's directory first on `PATH`, so `{python}` is the venv's python;
4. reads the outputs and normalizes JSON reports. It fails if a command exits non-zero, an output
   is missing, or an output contains the temp dir or the checkout path.

Normalized fields, all in `meta` (the page says so above the JSON):

| Field | Recorded as | Why |
|---|---|---|
| `created_utc` | `2026-01-01T00:00:00Z` | wall clock |
| `sundgren_version` | `<version>` | changes every release |
| `inputs.*_lcov_sha256` | 64 zeros | lcov files hold temp paths and tool versions |
| `commands` | the venv's python path → `python` | machine path |

Everything else is real output.

## Commands

```sh
uv run python tools/docs_examples.py                  # check all examples
uv run python tools/docs_examples.py --update NAME    # re-record NAME
uv run pytest tests/docs -rs                          # the same check, plus the page checks
```

An example whose `requires` fail is skipped. With `SUNDGREN_DOCS_REQUIRE=python,rust` the listed
languages fail instead; CI sets it so a missing toolchain can't pass silently. Review the diff
before committing a re-recording: the page and its graphic change with it.

## Rust

`ignore_divide_rs` needs rustc 1.99.1 and cargo-tarpaulin 0.37.5, the versions CI installs.
Other versions change the output, so the test skips. On a dev box:

```sh
rustup toolchain install 1.99.1
cargo install cargo-tarpaulin --version 0.37.5 --locked
RUSTUP_TOOLCHAIN=1.99.1 uv run pytest tests/docs/test_examples.py -rs
```

When bumping a pin, change `requires` in its `example.toml`, the Rust job in
`.github/workflows/ci.yml` and in `docs.yml`, then re-record.

## Adding an example

Create the directory, run `--update NAME`, check the outputs, and include the files from
`docs/index.md` with `{literalinclude}` (the command from `commands.sh`) or
`{sundgren-graphic} examples/NAME`. Pages may not contain any other code block.
