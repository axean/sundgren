# v2 plan

Implements [DESIGN.md](DESIGN.md). Updated as work lands. Bar lengths are rough hours; the order
and dependencies matter more than the times.

```mermaid
gantt
    title sundgren v2
    dateFormat YYYY-MM-DD HH:mm
    axisFormat %H:%M

    section Setup
    Scaffold: workspace, maturin, stubs (main)        :done, s0, 2026-10-19 13:30, 1h

    section M1 port
    W1 model                                           :done, a1, after s0, 1h
    W1 lcov, junit, normalize, hashing                 :done, a2, after a1, 3h
    W1 differential harness + Rust CI                  :done, a3, after a2, 2h
    W2 patch                                           :done, b1, after s0, 3h
    W2 linemap, symbols                                :done, b2, after b1, 3h
    W2 json and text reports                           :done, b3, after c2, 2h
    W3 rollup                                          :done, c1, after a1, 2h
    W3 diff                                            :done, c2, after c1, 5h
    W4 template, guards, presets, selectors            :done, d1, after s0, 3h
    W4 workspace, runner                               :done, d2, after d1, 3h
    W4 CLI (W5 takes the run subcommand)               :active, d3, after d2, 4h
    W1 MSRV 1.88, release wheels (maturin-action)      :active, a4, after a3, 2h
    Conformance green, prototype removed (all)         :m1, after b3 d3 a3, 2h

    section M2 IR L0
    W5 IR types, node ids, serialization               :done, e1, after s0, 3h
    W5 native parser to IR mapping (research)          :done, e2, after s0, 2h
    W2 Python frontend, out-of-process (Python ast)    :active, e3, after b3, 3h
    W5 Rust frontend (rust-analyzer parser)            :done, e4, after e1, 4h
    Frontend spans replace symbolizers                 :e5, after e3 e4, 2h

    section M3 L1, join, first analysis
    W3 L1 contract + structural hashing                :active, f1, after f2, 3h
    W3 coverage to IR join (started early)             :done, f2, after c2, 3h
    W3 duplicate-function analysis                     :f3, after f1, 3h

    section M4 verification
    Suggestions as patches through run                 :g1, after f3, 4h
```

M5 (CFGs, branch mappers, richer coverage formats) is planned once M3 is done.

The native `sundgren` binary (no Python) comes after M1. In M1, argparse stays in `cli.py`, and
Rust owns every command handler behind a plain args struct that a clap front end can fill later.

## Roster

| Worker | Owns |
|---|---|
| W1 | `model`, `ingest/lcov`, `junit`, `normalize`, `hashing`; `tools/differential.py`; Rust CI jobs |
| W2 | `patch`, `linemap`, `symbols`, `report/json_report`, `report/text_report` |
| W3 | `rollup`, `diff`; then `crates/sundgren-join` |
| W4 | `run/*`, `cli` (except the `run` subcommand) |
| W5 | `crates/sundgren-ir`, frontends, `design/ir-mapping.md`; the CLI `run` subcommand (`crates/sundgren-cli/src/run.rs`) |

Owning a Python module means owning its Rust file in `crates/sundgren-*/src/`, its binding file
`crates/sundgren-py/src/<pymod>.rs`, its stub `src/sundgren/_native/<pymod>.pyi`, and the
Python module itself. `<pymod>` is the module path joined by `_`: `core_model`, `run_presets`,
`cli`.

## Port conventions

- **Faithful port.** Same behaviour as the prototype, no new features. `DECISIONS.md` is the spec
  for edge cases.
- **Plain Rust outside `sundgren-py`.** Only the bindings crate uses PyO3.
- **Switch-over.** When a module's Rust port is ready, its Python file becomes a re-export:
  `from sundgren._native import core_patch as _impl`, then `parse_patch = _impl.parse_patch`.
  File paths stay, so every import keeps working.
- **Python types.** Frozen `#[pyclass]` types with the dataclasses' field names, defaults,
  equality, hashing and repr.
- **Native tools stay.** Where a language has an exact tool, sundgren uses it instead of
  reimplementing it: the Python pack keeps Python's `ast`. Registries that language packs fill
  (`symbols._REGISTRY`, `selectors._REGISTRY`, `presets._BUILTINS`) stay Python dicts; the
  bindings read them at call time and wrap the entries for the Rust side.
- **Tests are the conformance suite.** Existing tests pass unchanged. If a test is tied to a
  Python internal that a Rust port can't honour, ask main before changing it, and record the
  change in `DECISIONS.md`.
- **Differential runs.** `tools/differential.py` runs sundgren 0.1.2 from PyPI and the working
  tree on the same inputs and compares the JSON byte for byte.
- **Checks before a piece is done:** `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test`, `uv run ruff check .`, `uv run ruff format --check .`, `uv run mypy`,
  `uv run pytest`.

## Status

**Paused 2026-10-19 by Axel.** Everything from the worker branches is merged here.
`crates/sundgren-verify` is a stub (the checker is skipped for now).

M1: every module is in Rust except the CLI `run` handler (W5). `cli.py` switches over after it.
`tools/differential.py` vs 0.1.2: 238/239 cases byte-identical; the one known difference is the
JUnit parse-error text (roxmltree vs expat).

| Piece | Owner | State |
|---|---|---|
| core: model, lcov, junit, normalize, hashing, patch, linemap, symbols, rollup, diff, reports | W1, W2, W3 | Rust, switched over |
| run/* | W4 | Rust, switched over |
| CLI diff/presets/ingest handlers | W4 | Rust, matches prototype on 33 argv cases |
| CLI run handler, cli.py switch-over | W5 | in progress |
| MSRV 1.88, release wheels | W1 | in progress |
| IR crate, Rust frontend | W5 | done (validated on 3,799 .rs files) |
| Python frontend (out-of-process) | W2 | in progress |
| coverage to IR join | W3 | done |
| L1, structural hashing, duplicate-function analysis | W3 | in progress |
