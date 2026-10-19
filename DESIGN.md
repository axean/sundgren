# Sundgren v2 design: Rust core, PyO3 bindings, source IR

Status: draft for discussion. Nothing here is implemented yet. Sections marked **Open** list a
proposed default that holds until decided otherwise.

The Python package in `src/sundgren` is the prototype. v2 rewrites it in Rust, ships it as a
Python package with PyO3 bindings, and adds a language-neutral intermediate representation (IR) of
source code. The IR is combined with test coverage to produce refactoring suggestions, and the
existing base-vs-patched coverage diff verifies them.

## Goals

- Rust implementation of everything the prototype does: lcov ingest, unified-diff parsing, line
  maps, coverage diff, function deltas, JUnit outcomes, JSON and text reports, the `run` pipeline.
- One Python wheel (`pip install sundgren`) whose `sundgren` module is a thin layer over the Rust
  extension. The CLI keeps working.
- A source IR built by per-language frontends that use the best available parser for each
  language.
- Coverage joined onto the IR, so every IR node can say whether tests execute it, and which.
- Refactoring suggestions: language-independent analyses on the IR, and language-aware ones in
  language packs. First target: exact duplicate functions.
- Every suggestion can be emitted as a patch and checked with the existing diff pipeline.

## Non-goals

- Running or instrumenting code ourselves. Execution stays with each language's own test and
  coverage tools. Sundgren is a meta tool and only reads what they produce.
- A universal semantic IR (types, name resolution across languages). The IR models structure and
  control flow, not meaning.
- Proving behaviour preservation. Coverage gives evidence, not proof (see Reachability).

## Reachability: what coverage can and cannot say

Exact reachability is undecidable. Sundgren does not solve that; it brackets it:

```
executed by tests   ⊆   actually reachable   ⊆   statically maybe-reachable
  (lcov: witness)          (undecidable)          (IR / CFG: over-approximation)
```

- A hit line is a witness: some execution reached it. This is a must-reach fact.
- A missed line is only absence of evidence, never proof of dead code.
- The static side (IR, later the CFG) over-approximates what may run.
- Code between the two bounds is uncertain. Suggestions touching it must say so.

The question that matters for refactoring is narrower than reachability: is this code's behaviour
pinned by tests? Coverage answers that directly. Suggestions carry this as a confidence signal.

## Architecture

```
 language tools (external)            sundgren
 ─────────────────────────            ─────────────────────────────────────────────
 coverage.py, cargo-llvm-cov,  ──lcov──▶ ingest ──▶ Coverage model ─┐
 tarpaulin, … (test execution)                                       ├─▶ join ─▶ annotated IR
 source files ─────────────────────────▶ frontends ──▶ IR ──────────┘            │
                                                                                 ▼
                                    verify ◀── patch ◀── suggestions ◀── analyses
                                      │
                                      └─ run base + patched, diff coverage and outcomes
```

Two interchange formats, one per kind of fact:

- **lcov** (and later richer formats) for dynamic facts, produced by external test tools.
- **The IR** for static facts, produced by frontends, in process or out of process.

### Cargo workspace

| Crate | Contents | Depends on |
|---|---|---|
| `sundgren-core` | Coverage model, lcov, patch parser, line maps, diff, JUnit, reports | — |
| `sundgren-ir` | IR types, stable node ids, serialization, structural hashing | — |
| `sundgren-join` | Coverage ↔ IR join, source-hash checks | core, ir |
| `sundgren-analyze` | Language-independent analyses and the `Suggestion` type | ir, join |
| `sundgren-frontend-python` | Python frontend | ir |
| `sundgren-frontend-rust` | Rust frontend | ir |
| `sundgren-run` | Worktrees, command templates, presets, selectors | core |
| `sundgren-cli` | Native CLI binary | all |
| `sundgren-py` | PyO3 module `sundgren._native`, built with maturin | all |

Rules, carried over from the prototype's layering (`tests/unit/test_layering.py`):

- `core`, `ir`, `join` and `analyze` know no specific language and start no subprocesses.
- Only `sundgren-py` depends on PyO3. Every other crate is plain Rust, so the native CLI and Rust
  users don't need Python.
- Language knowledge lives in frontend crates and preset files, registered by file extension.

### Python package

- Build backend: maturin, mixed layout (`python/sundgren/` + `sundgren._native`).
- `abi3` wheels for the lowest supported Python (currently 3.11), one wheel per platform.
- The Python layer only re-exports types, converts errors, and keeps the `sundgren` console
  entry point. No logic lives in Python.
- Python-visible types mirror today's dataclasses (`Snapshot`, `FileCoverage`, `Delta`, …) as
  frozen `#[pyclass]` types, with the same field names, so existing callers keep working.
- **Open:** whether to also expose the IR to Python as objects, or only as serialized JSON.
  Proposed default: JSON first, typed classes once the IR stabilises.

## Coverage model

Ported from `sundgren/core/model.py` unchanged in meaning: per file, line hit counts, branches
keyed by `(line, block, branch)`, function records, test names.

Additions:

- **Source identity.** Each `FileCoverage` may carry the sha256 of the source it was measured on.
  The join refuses to attach coverage to an IR built from different bytes and warns instead.
- **Granularity tag.** Each input states what it can resolve: `line` (lcov), `region` (line and
  column spans, e.g. llvm-cov JSON), `arc` (coverage.py JSON). lcov stays the format every tool
  must be able to produce; richer formats are optional and add precision.
- **Per-test coverage (contexts).** Optional map from test id to the lines and branches it hit.
  Sources: coverage.py dynamic contexts, or running tests one at a time. Needed to know which
  tests pin a node, and to re-run only those when verifying a suggestion.
- **Open:** which richer formats to support first. Proposed default: llvm-cov JSON export, then
  coverage.py JSON, after the lcov port is done.

## IR

The IR is built in layers. Each layer is useful on its own, and v1 stops where suggestions stop
needing more.

| Layer | Contents | Enables |
|---|---|---|
| L0 symbols | Files, modules, classes/impls, functions, closures, with spans and qualified names | Function deltas; fixes Rust closures cutting spans and same-name collisions |
| L1 syntax | Normalized statement and expression tree, comments and formatting removed | Structural hashing, clone detection |
| L2 control flow | Per-function CFG with branch nodes | Mapping branch coverage; dead-branch and simplification suggestions |

### Nodes

Every node has:

- `id`: stable across runs on identical source. Derived from file path, qualified name and a
  structural path, not from memory addresses or traversal counters.
- `kind`: a small shared vocabulary (`Module`, `Class`, `Function`, `Closure`, `Block`, `If`,
  `Loop`, `Match`, `Arm`, `Return`, `Call`, `Assign`, `Expr`, …). Constructs with no shared kind
  use `Other(lang_kind)`.
- `span`: byte offsets plus 1-based line and column for start and end, in the original file.
  Spans are exact, because suggestions are applied as text edits on them.
- `body_span` for functions: the lines that count as the body. The prototype's rule is kept:
  signature and decorator lines don't count, since they run at import or definition time.
- `name` and `qualified_name` where they exist (Python names match `__qualname__`).
- `lang`: an opaque, language-specific payload. Generic analyses ignore it; language packs use it.

### Serialization

- A versioned, documented format, so out-of-process frontends can produce it. **Open:** JSON vs. a
  binary format. Proposed default: JSON for v1, with the schema checked in like
  `core/report/delta.schema.json` is today.

### Structural hashing (L1)

- `exact` hash: the normalized subtree, ignoring whitespace and comments. Finds exact clones.
- `renamed` hash: the same, after renaming local bindings consistently in order of first use.
  Finds clones that differ only in local names.
- Hashes are computed per node, bottom-up, so any subtree can be compared, not just functions.

## Frontends

A frontend turns one source file into IR. Trait sketch:

```rust
pub trait Frontend {
    fn language(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn parse(&self, path: &str, source: &[u8]) -> Result<ir::File, FrontendError>;
}
```

- **Parsers.** Each language uses the most accurate parser available that keeps exact spans.
  Candidates: Rust: `ra_ap_syntax` (rust-analyzer, lossless) or `syn`; Python: ruff's parser or
  tree-sitter-python. tree-sitter is the fallback for any language, since it is lossless and
  tolerates broken code, but it only gives syntax.
- **Open:** which parser per language. Proposed default: tree-sitter for both languages in v1
  (one code path, no version coupling), with native parsers added where tree-sitter falls short.
- **Grammar versions.** The prototype parses Python with the running interpreter's grammar, so
  newer syntax fails. A Rust-side parser removes that coupling. A parse failure must never fail
  the run: like today, spans then fall back to lcov `FN` records.
- **Out-of-process frontends.** Any executable that writes the IR format on stdout can be
  registered for an extension. This lets a language use its own tooling, the same way lcov lets
  any language report coverage. **Open:** in v1 or later. Proposed default: design the format for
  it now, implement the runner later.

## Join: coverage onto IR

- The only reliable key is `(file, line)`. Function names from lcov are mangled, collapsed by name
  and spelled differently per tool, so they are used for matching only as a fallback (as the
  prototype already does: name first, then line overlap).
- The join produces an overlay, `NodeId → CoverageFact`, and never modifies the IR. One IR can be
  joined against many runs (base, patched, repeats).
- `CoverageFact`: executable lines in the node's span, hit lines, missed lines, branch outcomes
  mapped onto branch nodes, and covering test ids when per-test data exists.
- Node status, extending today's function statuses: `covered`, `partial`, `uncovered`,
  `unmeasured` (no executable lines known), `flaky` (differs between repeats).
- **Line ambiguity.** lcov can't distinguish two statements on one line or tell which line of a
  multi-line statement was counted. Facts record the granularity they came from, and analyses
  must not claim more precision than that.
- **Branch mapping is per tool.** `BRDA` branch ids are tool-specific (coverage.py names target
  lines, llvm uses its own ids). Each coverage preset declares a branch mapper that attaches its
  ids to L2 branch nodes. Without L2, branches attach to the enclosing statement.
- **Path normalization** is the existing ingest logic: repo-relative paths, external files
  dropped.

## Analyses and suggestions

```rust
pub struct Suggestion {
    pub id: String,             // stable: analysis name + involved node ids
    pub analysis: String,       // e.g. "duplicate-function"
    pub nodes: Vec<NodeId>,
    pub message: String,
    pub evidence: Evidence,     // coverage facts and covering tests of every involved node
    pub confidence: Confidence, // derived from evidence, see below
    pub edits: Option<Vec<TextEdit>>, // a ready patch, when the analysis can produce one
}
```

- Language-independent analyses live in `sundgren-analyze` and use only generic IR fields.
- Language-aware analyses live in language packs and may read the `lang` payload.
- Applying a suggestion is always language-specific (syntax, imports, formatting), so `edits` are
  produced by the language pack even when detection is generic.
- **Confidence** comes from coverage: every touched node covered by at least one passing test is
  higher confidence than code no test reaches. Uncovered code is reported, not hidden.

### First analysis: duplicate functions

- Group functions by L1 `exact` hash, then by `renamed` hash. Ignore trivial bodies (below a
  minimum node count, configurable).
- Evidence per group: coverage status and covering tests of each copy.
- Suggestion: keep one copy and replace the others with calls or re-exports. Prefer the copy
  covered by the most tests. If no copy is covered, say so; that is itself worth reporting.
- This analysis is the end-to-end test of the whole design: it needs frontends, L0, L1, the join
  and suggestions, but not L2.

## Verification loop

The prototype's `run` command already runs the test suite on a base and a patched worktree and
diffs coverage and test outcomes. In v2 that becomes the check for suggestions:

1. An analysis emits a suggestion with `edits`.
2. The edits are written as a unified diff.
3. `run` executes base and patched, using per-test data to select only the covering tests when it
   exists.
4. The suggestion **holds** when test outcomes are unchanged and no touched node lost coverage
   (no `went_dark`, no lost lines or branches inside the touched spans).
5. Otherwise it is reported with the delta that broke it.

A held suggestion is "preserved behaviour as far as the tests observe". Reports must not word it
as more than that.

**Open:** whether v1 runs this loop automatically or only produces patches the user runs through
`sundgren run`. Proposed default: produce patches in v1, automatic verification in v2.

## Port strategy

1. **Faithful port first.** Port the prototype module by module into `sundgren-core` and
   `sundgren-run` with identical behaviour. No new features during the port.
2. **The prototype's tests are the conformance suite.** The unit, e2e and docs tests, with their
   fixtures and recorded reports, run against the Rust implementation through the Python bindings.
   The port of a module is done when its tests pass unchanged.
3. **Differential runs.** During the port, both implementations run on the same inputs and their
   JSON output is compared byte for byte.
4. **Then remove the prototype.** Once every test passes against Rust, the Python implementation
   is deleted and only the binding layer stays.
5. `DECISIONS.md` stays the record of behaviour choices. Each ported decision keeps its entry.

Suggested split for parallel work, along existing module boundaries:

| Track | Modules |
|---|---|
| A | `model`, `ingest/lcov`, `junit`, `normalize`, `hashing` |
| B | `patch`, `linemap`, `diff`, `symbols`, `rollup` |
| C | `report/json_report`, `report/text_report`, `run/*`, CLI, PyO3 and maturin packaging |

## Milestones

1. **M1 port:** workspace, maturin build, tracks A–C, conformance suite green on Rust. Release as
   a version with identical output.
2. **M2 IR L0:** frontends for Python and Rust, symbol spans replace `PythonSymbolizer` and
   `LcovFnSymbolizer` where a frontend exists. Fixes the Rust closure and naming limitations.
3. **M3 IR L1 + join + first analysis:** structural hashing, coverage overlay, duplicate-function
   suggestions with evidence.
4. **M4 verification:** suggestions as patches, checked through the `run` pipeline.
5. **M5 L2 and richer coverage:** CFGs, branch mappers, region- and arc-level coverage inputs,
   per-test coverage.

## Open questions

| # | Question | Proposed default |
|---|---|---|
| 1 | How far does the IR go in v1? | L0 + L1; L2 in M5 |
| 2 | In-process frontends only, or out-of-process too? | Format designed for both; runner later |
| 3 | Parser per language | tree-sitter first, native where it falls short |
| 4 | Automatic verification of suggestions? | Patches in v1, automatic in M4 |
| 5 | Languages | Python and Rust |
| 6 | Port faithfully first, or redesign while porting? | Faithful port first |
| 7 | IR exposed to Python as objects or JSON? | JSON first |
| 8 | IR serialization format | JSON with a checked-in schema |
| 9 | Richer coverage formats | llvm-cov JSON, then coverage.py JSON, after M1 |
