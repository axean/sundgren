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

## Decisions for Axel
(none yet)

## Next steps
- Spawn workers; scaffold extension logic modules; open draft PR.
