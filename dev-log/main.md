# Dev log: main (coordinator)

Branch `sundgren-vscode`. Goal: a VS Code extension, Ruff style (bundled binary, platform-specific
.vsix), with one action, **"sg: pluck this test"**: a code lens above each Rust or Python test that
skips the test in a generated patch and shows what happens to coverage. Reliability first: check
before you leap, fail fast, refuse instead of guessing.

## Agreed with Axel (2026-10-20)

- One action only: pluck this test (`#[ignore]` in Rust, `@pytest.mark.skip` in Python).
- Code lens above tests; works without rust-analyzer or the Python extension (text scan).
- Preconditions, all before anything runs: workspace trusted; file saved; git repo with a commit;
  no merge/rebase in progress; no uncommitted tracked changes (sundgren's `dirty_tree` rule,
  checked up front); file tracked; the test is where the lens says in HEAD; the patch applies;
  the project is at the repo root (refuse subfolder layouts: `python.testing.cwd`,
  `rust-analyzer.linkedProjects`, nearest pyproject/Cargo.toml); tools present.
- Language decides the preset: Rust test -> Rust preset, Python test -> `pytest`.
- Python plucks run the whole suite (correct over fast). Rust runs all tests anyway.
- **Cancel is the most important feature**: snappy, responsive, and it must really stop the run
  (test processes too) and clean up. Notification with Cancel, and the lens turns into
  "sg: plucking... (click to cancel)".
- First run per project: show the plan (preset, interpreter, command), ask to confirm; remember a
  fingerprint in private workspace state; ask again when it changes.
- Settings: only `sundgren.path` (machine scope). No writes to `.vscode/settings.json`.
- Interpreter: Microsoft Python extension's selection (official API) -> `.venv` -> `python`.
- Bundled binary like Ruff; extension version = sundgren version. Store publishing: later
  (needs accounts and tokens; decision for Axel).
- Windows: refuse (sundgren doesn't support it).
- Strict TS (qc2 flags), ESLint type-aware strict + unicorn + sonarjs + security, Prettier,
  pre-commit hooks, ESLint/TypeScript badges.

## Team

| Who | Owns (only these edit these files) |
|---|---|
| main | `editors/vscode/src/*.ts` (extension logic: preconditions, test finding, patches, running sundgren, cancel, results), `editors/vscode/GUIDE.md`, `CHANGELOG.md` (extension entry), `dev-log/main.md` |
| w1 | Rust: `src/**`, `schema/`, Python harness tests for the CLI under `tests/`, `docs/reference.md` and `docs/how-it-works.md` sections for the new CLI behaviour, `dev-log/w1.md`. Tells main the CHANGELOG entries. |
| w2 | Extension infra: `editors/vscode/{package.json,package-lock.json,tsconfig.json,eslint.config.mjs,.prettierrc.json,.vscodeignore,README.md,.vscode-test.mjs}`, `editors/vscode/src/test/**` harness (not the test cases for main's modules), `.pre-commit-config.yaml`, `.github/workflows/*` (new extension job, packaging), root `README.md` badges, `dev-log/w2.md`. |

Workers work in their own git worktrees and branches (`sundgren-vscode-w1`, `-w2`), commit
locally, never push. main merges into `sundgren-vscode` and pushes.

## Interfaces (main decides; ask before changing)

### `sundgren run --check` (w1)
Same arguments as `run`. Does everything up to running the tests, then stops:
resolve repo and base, dirty-tree check, patch applies to the base (no worktrees left behind),
preset resolved, preset tool checks pass. Prints JSON (schema id `sundgren.check/1`):

```json
{"schema": "sundgren.check/1", "ok": true, "repo": "/abs/repo", "base_ref": "<sha>",
 "preset": "pytest", "python": "/abs/.venv/bin/python",
 "commands": ["python -m coverage run ..."], "selection": {"tests": [], "run_all": true, "reason": "..."},
 "problems": [{"code": "dirty_tree", "message": "..."}]}
```
`ok` is false when any problem would make the run fail or the comparison not make sense. Exit
code: 0 ok, otherwise the code `run` would exit with (2 usage, 3 patch doesn't apply, 1 other).

### Preset tool checks (w1)
New optional preset key `checks`: commands (templates) that must exit 0, e.g. pytest:
`{python} -c "import coverage, pytest"`; cargo-llvm-cov: `cargo llvm-cov --version`.

### Cancellation (w1, top priority)
On SIGTERM or SIGINT at any phase: stop the test commands (run them in their own process group;
SIGTERM the group, SIGKILL after a short grace), remove the worktrees, exit promptly (130 for
SIGINT, 143 for SIGTERM). Target: sundgren gone within ~2 s of the signal. The extension sends
SIGTERM, shows "cancelling..." at once, and SIGKILLs sundgren if it hasn't exited after ~5 s.

## Log
- 12:37 Branch created from main @ 1e53a1f. Plan and interfaces written. Spawning w1, w2.
- 12:45 Spawned w1 (Rust: cancellation first, then `--check`, preset `checks`) and w2 (extension
  scaffold, checks, test harness, CI, platform .vsix). Draft PR #17. Resume wake-ups scheduled for
  13:57 and 14:23 (session-only cron).
- 12:55 `src/tests.ts` (find tests by text scan: pytest functions and `Test*` methods; Rust
  `#[test]`/`#[x::test]`, skipping already `#[ignore]`d) and `src/patch.ts` (unified diff that
  inserts `#[ignore]` or `@pytest.mark.skip(reason="sundgren: plucked")`, plus `import pytest`
  above the enclosing top-level statement when the file doesn't bind `pytest`). Checked by hand
  with `git apply` on Python and Rust samples, including multi-line decorators, docstrings,
  nested functions. Unit tests follow once w2's scaffold builds (mocha, BDD, `node:assert`).
  Layout agreed with w2: esbuild bundles `src/extension.ts` to `dist/`; tsc compiles to `out/`;
  unit-tested modules must not import `vscode`; bundled binary at `bundled/sundgren`.
- 13:10 `src/git.ts` (repo root, HEAD, operation in progress, changed tracked files, tracked,
  HEAD text) and `src/sundgren.ts` (find the binary: setting -> bundled -> .venv -> PATH; version
  check; run with cancel). Cancel design: sundgren leads its own process group; Cancel sends
  SIGTERM; on sundgren's `exit` after a cancel the rest of the group gets SIGKILL (stray test
  processes); if sundgren is still alive after 5 s the whole group gets SIGKILL. Measured with
  stand-ins: a cooperative one (exits on TERM, leaves a child) stops in ~0.2 s with the child
  gone; one that ignores TERM stops at the 5 s fallback, child gone. Lesson: react to `exit`,
  not `close` (a stray child holding the pipes delays `close`).
- 13:30 `src/report.ts` (defensive parse of the delta report: lost lines per file, functions that
  went dark or lost lines, coverage before/after; one-sentence headline; checked on recorded
  reports) and `src/preconditions.ts` (the agreed checks in fail-fast order, each refusal one
  sentence saying what to do). Session limit hit; workers stopped uncommitted.
- 13:52 Resumed after the limit reset.
- 14:00 Merged w2's scaffold (esbuild, strict tsc, ESLint strict + unicorn/sonarjs/security,
  prettier, mocha + @vscode/test-cli, pre-commit hooks, CI jobs, per-platform .vsix in releases)
  and w1's cancellation (signal-safe stop of test commands, worktrees removed, 130/143; Linux
  PDEATHSIG safety net), `run --check`, preset `checks`, `--all-tests`, docs.
- 14:05 All my modules lint-clean under the strict config (only documented per-file exceptions
  for workspace paths and the user's git on PATH). w2's 50 unit tests found 3 real bugs, fixed:
  `import pytest` dropped when it and the skip go before the same line (the commonest case!);
  `#[ignore]` before `#[test]`; `#[ignore] // comment`. extension.ts: code lens, one pluck at a
  time, progress with Cancel (notification + lens), --check then confirmation then run, results
  (headline, highlights, output panel). Integration tests pass in real VS Code (activation,
  commands, lenses). The command returns an `Outcome`; no modal in ExtensionMode.Test.
- 14:10 CHANGELOG entries (#17). GUIDE.md written from the actual behaviour. Next: w2 writes
  end-to-end pluck and cancel tests in real VS Code; w1 makes cancel instant even with a big
  out/ dir (move to trash, delete in a detached process).
- 14:30 w2's end-to-end tests in real VS Code found a real bug: a repo opened through a symlink
  (and so on macOS /tmp -> /private) was refused as "not committed". Fixed by working with real
  paths. Also fixed from my own review: the run is pinned to the commit `--check` resolved (a
  commit made meanwhile changed what was compared), and highlights use the report's `base_line`
  (in Rust the inserted `#[ignore]` shifts the patched tree's lines in the same file). w1: cancel
  now exits at once even with a huge out/ dir (moved aside, deleted in the background). CI fully
  green on the PR, including the extension jobs. 13/13 e2e integration tests: done, symlink,
  dirty and unsaved refusals, cancel during the run and during --check, the plucking lens, HEAD
  moving during --check, Rust line shift.
- 14:35 Refuse clearly when the found sundgren is too old (no --check). Next: w2 smoke-tests the
  platform .vsix with the bundled binary; w1 adds the package.json version to the release
  check, dogfoods the extension's CLI sequence on the example projects, reviews macOS.

## Decisions for Axel
(none yet)

## Next steps
- Spawn workers; scaffold extension logic modules; open draft PR.
