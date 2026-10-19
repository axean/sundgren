# Decisions

Choices made where the scaffold spec was silent or where the implementation deviates from it.
Spec deviations are marked **(deviation)**.

## Build and layout

- maturin build (see "Rust core" below). `coverage` and `pytest` are dev dependencies only, for
  the e2e target project; sundgren never imports them.
- Language packs register their own `presets.toml` (`register_builtin_presets`), so `run` never
  globs `lang/`. Only `cli.py` imports `lang`.
- The docs dependency group (Sphinx 9, MyST, shibuya) is installed only on Python >= 3.12, because
  Sphinx 9 needs 3.12. sundgren itself keeps supporting 3.11.

## Versioning and releases

- The version is static in `pyproject.toml`, so `uv version --bump` works. `__version__` reads the
  installed metadata, and importing from an uninstalled source tree fails.
- SemVer covers the CLI (commands, flags, exit codes), the report schema (`sundgren.delta/N`), the
  preset / `sundgren.toml` format and the built-in preset names. Python modules are internal.
- Before 1.0, a breaking change bumps the minor version; anything else bumps the patch.
- Release tags are strict `vX.Y.Z`. `tools/release.py check` requires the tag to equal the
  version, a lint-clean changelog with a dated `[X.Y.Z]` section, and an empty `Unreleased`.
  The GitHub release notes are that section.
- `main` only takes changes through pull requests. A release is a PR that bumps the version and
  dates the changelog section, then a pushed tag.
- The `pypi` and `github-pages` environments accept only `v*` tags, so only a release can
  publish or deploy.

## CLI and run mode

- `--format` defaults to json. `{python}` is `python` on PATH, else `python3`; `--python` overrides.
- Run mode builds line maps from the diff git actually applied, not from the user's patch file:
  `git apply --index`, then `git diff --cached` in the patched worktree. That diff's shape is pinned
  against user git config (prefixes, colour, external diff, textconv, rename detection, context).
  An offset or a non-`a/`/`b/` prefix in the user's patch therefore can't skew the line maps.
- A hunk applied at an offset adds the warning `patch_offset`. `i/`/`w/` prefixes don't warn.
- `meta.patch.files` comes from the applied diff; `meta.patch.sha256` is the user's file.
- Patches are read as bytes and decoded as UTF-8 with `surrogateescape`, without newline
  translation, so CRs and non-UTF-8 path bytes survive.
- Exit codes: a bad `--base` ref is 2, a patch that doesn't apply is 3, any other git failure is 1.
- `meta.base_ref` is the full sha. `meta.commands` expands only `{python}` and `{tests}`, because temp
  paths would break determinism.
- `sundgren diff --patch` uses the user's patch as it is and assumes it applies exactly to the base
  tree.

## Presets, templates, runner

- Only `commands` (non-empty) and `lcov` are required in a preset. Unknown keys are rejected.
  Defaults: description "", env {}, junit/selector None, tests_style "args", shell false, hints ().
- If `sundgren.toml` and `[tool.sundgren.presets]` define the same name, `sundgren.toml` wins. Two
  language packs defining the same name differently raise `PresetError`.
- Templates expand in one regex pass, so an expanded value containing `{out}` is not expanded again.
  In shell mode only `{tests}` is quoted.
- The runner runs every command; a side fails only when its lcov is missing. `returncode` is the
  first non-zero code. Commands get stdin from /dev/null. A command that can't start is logged as
  127 (missing) or 126 (other OSError).
- Relative lcov/junit templates resolve against the worktree. Leftover output files are deleted
  before the commands run. Repeats go to `run-1`, `run-2`, ….
- Each side runs with its own `CARGO_TARGET_DIR`: slower, but the trees never share build artifacts.
- Both worktrees are checked out at the resolved base sha. If setup fails, everything is cleaned up,
  even with `--keep-worktrees`. With `--workdir`, only `base/` and `patched/` are removed.

## Test selection (Python)

- **(deviation)** A touched test file that was added, deleted or renamed gives `run_all`. The same
  selection runs on both sides, so a new test file would otherwise fail on the base side.
- With `tests_style = "args"`, test paths missing on a side are dropped for that side. If none
  remain, that side counts as empty coverage with a `nothing_to_run` warning.

## Patch parsing and line maps

- Paths drop their first component as `git apply -p1` does (`a/`, `i/`, `old/`, anything); a bare
  name is kept. `rename from/to` names are never stripped. `/dev/null` is `None`.
- `new file mode` / `deleted file mode` override the `---`/`+++` paths; `rename from/to` and
  `---`/`+++` override the `diff --git` header.
- Plain (non-git) diffs never rename: both sides take the `+++` name, or the `---` name when it is a
  prefix of it (`diff -u x.orig x` is a change to `x`).
- C-quoted paths are decoded (escapes, octal bytes, UTF-8 with surrogateescape).
- Text before the first diff is skipped (format-patch email, commit message, diffstat, signature).
  Unknown lines inside a git header are skipped.
- Binary diffs give `binary=True`, no hunks, and a warning; their line map has no changed lines.
  Copies are treated as renames with a warning. Mode-only changes have no hunks and no warning.
- The hunk header's counts decide which lines belong to a hunk. Malformed input raises `ValueError`
  with a 1-based line number. This includes extra hunk lines after an undercounting header, which is
  stricter than `git apply`.
- A hunk whose `new_start` disagrees with `old_start` plus the drift so far is placed from
  `old_start` and warns.
- Line maps are keyed by both old and new path. On a key collision, the map whose `new_path` is the
  key wins.
- `LineMap.to_new`/`to_old` use a counting rule from `deleted_old`/`added_new`; `old_to_new` holds
  in-hunk pairs only. A randomized check against `difflib` (1600 diffs, context 0–3) found no
  disagreement.

## lcov ingest

- BRDA: line and block split from the left, taken from the right, so branch ids may contain commas.
  Block stays a string.
- FN: `start,<digits>,name` is lcov 2.x; anything else is 1.x, so names may contain commas. FNL/FNA
  (lcov 2.2) are supported.
- A FNDA with no FN of that name is dropped (debug log); a FN with no FNDA has `hits=None`.
- Repeats within one SF section are summed. Duplicate SF sections merge with `merge_file_coverage`
  (`-` + n = n, `-` + `-` = `-`).
- Malformed known records raise `ValueError` (exit 1); unknown records are skipped with a debug log.
- A leading BOM is stripped. Lines split on `\n` only, not on `\f`, `\x85` and the like.
- `dump` is deterministic and round-trips: `parse(dump(s)) == s`.
- Rust function names stay mangled.

## Path normalization

- Lexical only: `\` becomes `/`, `.` and empty components are dropped, and `..` pops. The only file
  system access is `realpath`.
- The longest matching root wins, on component boundaries only. Every absolute root also matches
  its `realpath`, and an unmatched absolute path is retried via `realpath` (`/tmp` vs `/private/tmp`).
- An absolute path matching no root, or a relative path escaping via `..`, is external.
- `C:/…` is absolute, with a case-insensitive drive letter; the rest of a Windows path stays
  case-sensitive.

## JUnit

- pytest 9 writes xfail as `<skipped type="pytest.xfail">`, so `type` decides: `pytest.xfail` gives
  xfailed and `pytest.skip` gives skipped, whatever the message says. An untyped `<skipped>` follows
  the spec's message-prefix rule.
- **(deviation)** A strict XPASS (`<failure message="[XPASS(strict)]…">`) is `xpassed`, not
  `failed`. A non-strict xpass can't be detected and reads as passed.
- Severity order: passed < skipped < xfailed < xpassed < failed < error. Duplicate ids (reruns,
  failure plus teardown error) and multiple result children resolve to the most severe.
- Test id is `classname::name`, or `name` when classname is empty. Testcases are found at any
  nesting depth, and suite names are ignored.
- A testcase without a name is dropped with a debug log. Malformed XML or a wrong root raises
  `ValueError`, and the CLI then drops the file with a `junit_unreadable` warning.

## Function spans

- Function status counts only `Span.body_lines`, so signature and decorator lines, which run at
  import in Python, don't count.
- Python symbolizer (`ast`): names match `__qualname__`; `start` is the first decorator line. A
  leading docstring is skipped when another statement follows. A one-liner has `body_start = None`.
- Every parse failure becomes `SyntaxError`, including `RecursionError` and `MemoryError` on deeply
  nested source, so `resolve_spans` falls back to lcov FN. Warnings from parsing the target's
  source are silenced. A leading BOM is stripped.
- `LcovFnSymbolizer` sets `body_start = start + 1` for multi-line spans: tarpaulin and coverage.py
  report the FN line as hit even when the body never runs. A missing end runs to the next distinct
  FN start − 1, and the last function runs to the last executable line.

## Diff

- File pairing: patched paths first, via `linemap_for(P).old_path`; then unclaimed base paths. A
  base file colliding with an already-paired path is dropped with an `unpaired_file` warning.
- A side is *measured*, *inferred* (missing from coverage but present under a known root),
  *nonexistent* (per the line map or the root), or *unmeasured* (root unknown, line map silent).
- An inferred side takes the other side's executable lines and branches, mapped through the line
  map, with count 0, and warns `inferred_file`. When the measured side has nothing executable,
  nothing is inferred and there is no warning.
- An unchanged line executable on one side only is `unmeasured`, with one warning per file (none
  when the count is 0).
- A changed line counts as deleted plus added, so an edited `if` line gives branch_deleted plus
  branch_new.
- Branches match on (mapped line, block, mapped id), and every `line N` in a base branch id is
  mapped through the line map. coverage.py ids name the target line (`jump to line 7`), so a shift
  would otherwise produce false lost/gained. A target that doesn't map leaves the branch unmatched.
- Branch categories: `branch_kept_hit`, `branch_kept_miss`, `branch_lost`, `branch_gained`,
  `branch_new`, `branch_deleted`, `branch_flaky`, `branch_unmeasured`. Entries carry `base_line`
  (an addition to spec §7).
- Flaky (hit in some repeats but not all) lines are removed from `lines` entirely, including the
  new/deleted counts, and listed in `files[].flaky`. Stats come from run 0.
- `source_changed`: sha256 differs when both shas are known; otherwise true if the file is absent
  on one side or its line map has changes. A pure rename is not a source change.
- Tests: when both outcome maps are None the list is `[]`. When only one exists it's `[]` plus a
  `tests_one_side` warning. Otherwise the result is the union of ids, with `not_run` for a missing
  side.
- Warnings are `{code, message}`, deduplicated, sorted, and use repo-relative paths only.
- Sort order: lines by line then base_line, with deleted lines (`line` null) last. Branches follow,
  then by block and a natural-sorted branch id.

## Functions and tree

- Functions match by qualified name (first span per name), then by largest line overlap after
  mapping. That overlap step pairs coverage.py FN names (`outer.inner`) with symbolizer names
  (`outer.<locals>.inner`).
- `went_dark`/`lit_up` take precedence. Otherwise `mixed`, `partial_loss` or `partial_gain` follow
  from the body's lost/gained counts, else `unchanged`. Flaky lines are ignored.
- A side with unknown spans borrows the other side's spans through the line map.
- `tree` covers every non-external file on either side, not just changed ones. Dir stats are sums.
  A file and a dir may share a name (file `a` deleted, `a/b` added).

## Reports

- Default output lists only files with a non-kept change, drops `kept_*` lines, kept branches and
  `unchanged` functions, and lists only changed test outcomes. `--full` keeps everything; `summary`
  and `tree` are never filtered.
- `--counts` fills `count_changed` and also lists files whose only change is hit counts.
- `summary.functions` has 7 statuses (no `unchanged`). Coverage % is from the root tree stats,
  rounded to 1 decimal, and null when nothing is executable.
- `dumps` is `json.dumps(sort_keys=True, indent=2)` plus a trailing newline. The schema is Draft
  2020-12 with `additionalProperties: false`, and meta fields are conditional on mode.
- The text report renders from the JSON dict, not the Delta. Hints are printed only when there are
  warnings or a listed file has `inferred_side`.

## Tests and fixtures

- `shed_py` is the spec's §12.1 code verbatim, including its header comment, so `divide` is at
  lines 5–8.
- `shed_rs` returns `Result` from `divide`. No `Cargo.lock` is committed.
- Rust E2E covers both `cargo-llvm-cov` and `cargo-tarpaulin`; each is skipped if not installed.
  CI installs both. Tarpaulin skips when it can't trace (no ptrace). The stderr pattern is a guess,
  since no machine without ptrace was available.
- E2E runs sundgren in a clean env (no `GIT_*`, `PYTEST_*`, `COVERAGE_*`, `PYTHONPATH`, user git
  config) with a private TMPDIR. After every run it asserts that no worktree or temp dir is left.
  One scenario uses a deliberately hostile git config.
- Saved JUnit fixtures have neutral hostname, timestamps and paths.

## Docs site

- GitHub Pages: one page with a Python and a Rust example, plus a changelog page. No API docs.
- Every code block on the page is an include of a file under `docs/examples/`. The docs tests
  rerun each example's exact commands and compare the output with the recordings. A test rejects
  any other code on the page.
- The graphic is rendered from the recorded `report.json`. A report line, branch or span that
  doesn't land in the source fails the build.
- Recordings normalize only `created_utc`, `sundgren_version`, lcov shas and the venv Python path
  in `meta`; everything else is real output.
- The Rust example uses cargo-tarpaulin, not llvm-cov: llvm-cov's mangled names carry a crate
  hash that changes with rustc. rustc and tarpaulin are pinned (1.99.1, 0.37.5) in the example
  manifest, `ci.yml` and `docs.yml`.
- `docs.yml` runs the docs tests and a strict build (`-W`) on every push and PR, and uploads the
  Pages artifact. `release.yml` calls it once to gate the PyPI publish, then deploys that same
  artifact after the publish.

## Known limitations (v1)

- Functions with the same name in one lcov file collapse into one record, as lcov keys by name.
- Rust closures have FN records with no end, so the enclosing function is cut at the closure's
  start. Fixing this needs a Rust symbolizer.
- A one-line function can't be `went_dark` from line data, because its def line runs at import.
- A deleted file plus a rename onto the same path (or a file turning into a symlink) can't both be
  represented in the line maps.
- A 1.x FN name starting with `<digits>,` is read as 2.x.
- The Python symbolizer parses with the running interpreter's grammar, so newer syntax falls back
  to lcov FN.
- lcov 2.2 FNL/FNA support was written from the documented format, without real output to check
  against. The llvm-cov unit fixture and nextest JUnit are hand-written; real llvm-cov output is
  only exercised by the CI e2e run, which asserts loosely (no exact function names).

## Spec examples vs. real output

- skip_divide loses 3 lines, not the 6 in the §7 example, because the def line runs at import. The
  lost branches have taken `[1, null]` (coverage.py writes `-`), not `[1, 0]`.
- **(deviation)** touch_source: the new function's def line is `new_hit`, because it runs at import;
  only its body is `new_miss`.
- With tarpaulin, `divide`'s signature line stays hit when its test is ignored, which is why
  `LcovFnSymbolizer` skips the FN line.

## Rust core

The prototype was ported module by module to Rust (`sundgren-core`, `sundgren-run`,
`sundgren-cli`) with PyO3 bindings (`sundgren-py`, built by maturin instead of hatchling). The
Python modules re-export `sundgren._native`. The unit, e2e and docs tests passed unchanged.

### Bindings and Python types

- Classes are frozen pyclasses wrapping the Rust value. A shared `dataclass!` helper gives
  `__dataclass_fields__` (so `dataclasses.asdict/astuple/fields/replace` work), `__match_args__`,
  the dataclass repr and `__reduce__` (copy, pickle). Dict and tuple fields are built on first
  access and cached (`x.lines is x.lines`). Mutating such a dict changes nothing.
- A class holding a dict is unhashable, as before; the `TypeError` names the class.
- `LineStatus` has `.name`/`.value` and the Enum repr, but no `LineStatus("hit")` or iteration.
- Exceptions keep their bases (`PresetError(ValueError)`, `GitError(RuntimeError)`,
  `BadRefError(GitError)`, `PatchApplyError(Exception)`). `RunFailure` is defined from Python
  source so its `(message, log_tail)` constructor and `str()` match.
- An exception the prototype didn't catch (from a selector, symbolizer or I/O) reaches Python as
  the original exception object.

### Build, CI and release

- MSRV is 1.88 (let-chains); a CI job runs `cargo check` on 1.88. A `rust` job runs fmt, clippy and `cargo test` on stable.
- `cargo test` excludes `sundgren-py` (its harness would link libpython); pytest covers it.
- Releases ship abi3 wheels (cp311+) for manylinux2014 x86_64 and aarch64 and macOS x86_64 and
  arm64, plus an sdist, built with maturin-action. Each wheel is installed into a clean 3.11 venv
  and runs `sundgren diff` on a fixture before publishing. The release checks gate the builds.

### Registries and hooks

- `symbols._REGISTRY`, `selectors._REGISTRY` and `presets._BUILTINS` stay Python dicts in their
  modules. The bindings read them by module attribute on every call, so patching or replacing
  them works. Python entries are wrapped as Rust traits (`Symbolizer`, `Selector`).
- `default_python` takes a `which` hook; the binding passes `shutil.which`, looked up at call time.
- TOML is decoded by `tomllib` behind the `TomlLoader` trait; Rust validates the tables. Accepted
  input and decode errors stay tomllib's.
- argparse stays in `cli.py`. Rust owns every command handler; it gets the parsed arguments as a
  plain struct and everything Python provides (registries, `which`, tomllib, the version,
  `sys.stdout`/`sys.stderr`) through a `Host` trait.

### Text, numbers and paths

- Patch text, paths and warning messages are `PyText`: a Python str stored as generalized UTF-8,
  so surrogateescape-decoded input survives and sorts by code point. `to_os_bytes` gives the raw
  file system bytes back.
- Line numbers are i64 and counts i128. `py_int` follows `int(str)`, including Unicode decimal
  digits (Unicode 15.0 table). **(deviation)** A line number outside i64 is a malformed lcov
  record; a hunk header must use ASCII digits that fit i64.
- `realpath` ports Python 3.12's `posixpath.realpath` (non-strict); 0 differences on 271k paths.
- `read_text` keeps Python's UTF-8 decode messages and universal newlines, since both show up in
  CLI errors and warnings.
- Ported where messages depend on them: `shlex` split/quote/join, `repr()` of a str (unassigned
  code points count as printable), `tempfile.mkdtemp` (TMPDIR, TEMP, TMP, then /tmp, /var/tmp,
  /usr/tmp, cwd; `tempfile.tempdir` set from Python is not honoured), `str.splitlines`, and the
  `[Errno N] strerror: 'argv0'` text of a command that can't start (Python names the cwd when
  the chdir failed).
- Command-line arguments and selected test ids are UTF-8 strings: one with undecodable bytes
  raises instead of being passed on.
- `sha256_file`'s OSError message reads "No such file or directory (os error 2)".

### Modules

- JUnit XML is parsed with roxmltree instead of expat. **(deviation)** The text after
  "malformed JUnit XML: " in a `junit_unreadable` warning is roxmltree's. Outcomes, accept/reject
  and "not JUnit XML" messages are unchanged.
- lcov and junit debug logs go through a sink; the binding forwards them only when DEBUG is on.
- `build_linemaps` returns one object per file under both keys (`m[old] is m[new]`).
- `resolve_spans` returns new `Span` objects equal to the symbolizer's.
- Branch-id `line N` refs use Unicode digits like Python's `\d`; an N outside i64 is unmapped.
- `dumps` writes `json.dumps(sort_keys=True, indent=2)` in Rust (ensure_ascii, Python float repr)
  for plain data and falls back to `json.dumps` for anything else. `load_schema` stays Python.
- Display commands quote tokens outside `[\w@%+=:,./{}-]`; Rust's `is_alphanumeric` stands in
  for `\w` and differs only on combining marks.

### Checks

- Every port was fuzzed or diffed against the prototype (shlex 200k inputs, lcov 100k tracefiles,
  junit 120k documents, patch 50k, `json.dumps` 150k values, text report 20k, build_report 12k
  deltas).
- `tools/differential.py` runs 0.1.2 from PyPI and the working tree on 239 fixture cases and
  compares stdout, stderr and exit code, ignoring `created_utc` and `sundgren_version`. The one
  known difference is the JUnit parse-error text.
- The run pipeline lives in `sundgren_run::pipeline` (`run_pair`). The CLI maps `PipelineError` to the original messages and exit
  codes.
