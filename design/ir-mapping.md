# Mapping native parsers into the IR

Research for M2 (IR L0). Decision taken: parse with each language's native tools, and map them
faithfully into the IR. tree-sitter is only the fallback for languages without one.

Everything below was checked by running it: Python 3.12.13 and 3.14.4, rustc 1.97.1,
`ra_ap_syntax` 0.0.349, `syn` 3.0.9 with `proc-macro2` 1.0.107. Scratch probes are not checked in.

## Summary

| Language | Parser | How it runs |
|---|---|---|
| Python | `ast` of the target project's interpreter | Out of process: `src/sundgren/lang/python/ir_frontend.py` writes IR JSON |
| Rust | `ra_ap_syntax` (rust-analyzer), not `syn` | In process: `sundgren-frontend-rust` crate |
| Others | tree-sitter | In process, later |

- The Python frontend prototype agrees with `PythonSymbolizer` on every function of the 3.12
  and 3.14 standard libraries and this repo (about 36,000 functions), except one edge case
  (decorator in parentheses, see below), where the IR is the correct one.
- Its output validates in Rust (`File::validate`) for every file tried.

## Shared IR rules (`crates/sundgren-ir`)

- **Spans** are half-open byte ranges of the original file bytes, plus 1-based line and column
  at both ends. Columns count bytes, not characters. A span never includes the trailing newline,
  so `end.line` is its last line. The root (the whole file) is the one exception.
- **Lines** depend on the language: the file records `newlines`. Python breaks lines at `\n`,
  `\r\n` and a lone `\r` (`universal`). Rust only at `\n` (`lf`). Neither breaks at `\f`,
  `\v`, `\x1c`–`\x1e`, `\x85` or ` `, which `str.splitlines()` does. Never compute line
  numbers with `splitlines()`.
- **Node ids**: the root is the path, a named node is `<path>::<qualified_name>`, and the k-th
  repeat of a name in the file (k ≥ 1, in node order) gets `#k` (`a.py::C.x#1` for a property
  setter). Ids survive edits elsewhere in the file. Unnamed L1 nodes will use a structural path
  under their nearest named ancestor.
- **Names** use the language's own naming: Python `__qualname__`, Rust rustc's def path. These
  are what runtime and coverage tools print, so the join can match them.
- **Nodes** are a flat list in pre-order, each with its `parent` id. Flat because ASTs nest
  thousands deep (long `elif` chains, long `+` chains) and serde_json stops at depth 128.
  `parent` is the nearest enclosing node by span, which is not always the naming scope: a
  lambda in a decorator is named in the outer scope but lies inside the decorated def.
- **`kind`** is the shared vocabulary; anything else is `{"Other": "<native kind>"}`.
  `lang` carries the native node type (`{"ast": "AsyncFunctionDef"}`, `{"syntax": "FN"}`).
- **`body_span`** (functions and closures): from the first statement that counts as the body to
  the end of the function. `null` for a one-liner, whose whole span then counts. This is exactly
  the prototype's `Span.body_start` rule, so `Span(name, span.start.line, span.end.line,
  body_span.start.line)` reproduces today's spans.
- **Faithful** means: every span slices out exactly the node's text, and a text edit on a span
  changes that node and nothing else. Tested: replacing every node with its own text is the
  identity, and replacing a body keeps signature and decorators. `File::validate` checks the
  hash, ids, tree order, and that every line/column agrees with its byte offset.

## Python: `ast`

Run by the target interpreter, so its grammar always matches the code (the problem noted in
DESIGN.md under "Grammar versions"). The script is stdlib only and parses as Python 3.8.

### Node mapping (L0)

| `ast` | IR kind | name / qualified_name |
|---|---|---|
| `Module` | `Module` (root) | none |
| `ClassDef` | `Class` | `C`, `outer.<locals>.C` |
| `FunctionDef`, `AsyncFunctionDef` | `Function` | `__qualname__`, e.g. `C.m`, `f.<locals>.g` |
| `Lambda` | `Closure` | `<lambda>`, `f.<locals>.<lambda>` |

Only the body opens a new name scope. Decorators, defaults and base classes are named in the
enclosing scope, as Python does. Not modelled: comprehension scopes. A lambda inside a list
comprehension is `<listcomp>.<lambda>` on 3.11 but plain `<lambda>` on 3.12+ (PEP 709); the IR
always uses the latter. Lambdas are not in the prototype's spans, so this only matters later.

### Spans

`ast` gives `lineno`/`end_lineno` (1-based) and `col_offset`/`end_col_offset`. Checked:

- Columns are **UTF-8 byte offsets** into the decoded line, not characters (`é` counts 2,
  `😀` counts 4). For UTF-8 files they are file byte columns, so `byte = line_start + col`.
- **Coding cookie** (`# -*- coding: latin-1 -*-`): columns are still UTF-8 offsets of the
  decoded text, so they are not file bytes. The frontend re-encodes the line prefix.
- **BOM**: `ast.parse(str)` rejects a BOM; `ast.parse(bytes)` strips it and line 1 columns don't
  count it. The frontend adds 3 on line 1.
- **Newlines**: a lone `\r` is a line break (also inside strings). `\f` is not; it counts as one
  column.
- **Decorators**: `FunctionDef.lineno` is the `def` line, and a decorator's position is its
  expression, after the `@` and any `(`. The frontend finds the `@`: it starts a logical line,
  so it is the first non-blank on its physical line, at or above the expression's line.
- **Ends** exclude trailing comments. Comments are not in `ast` at all, so L0 can't attach them;
  L1 will need `tokenize` for comments.

So nothing is lost for spans, but three corrections are needed (cookie, BOM, `@`). The script
parses the file bytes with `ast.parse(bytes)`, which applies the cookie as the interpreter does.

### Body spans

Same rule as `PythonSymbolizer`: skip a leading docstring when another statement follows; the
first counted statement starts at its first decorator; a one-liner is a body whose first
statement is on the `def` line (`def tail(\n a): return a` is not a one-liner).

**Known difference**, for M2: the symbolizer starts a function at its first decorator's
*expression* line. For `@(\n    dec\n)` that is the line after the `@`. The IR starts at the
`@`. Only a decorator split across lines inside parentheses or after a backslash triggers it
(none in the stdlib). Test: `test_parenthesized_decorator_starts_at_the_at_sign`.

### Running it

`python ir_frontend.py FILE [PATH]`: exit 0 with JSON on stdout, 1 if the file doesn't parse
(including null bytes, invalid UTF-8, too deeply nested source), 2 on bad usage. Parsing the
3.14 stdlib (628 files) takes about 3 s in one process. Starting one interpreter per file costs
about 30 ms each, so the real runner should send many files per process (JSON lines).

## Rust: `ra_ap_syntax` vs `syn`

| | `ra_ap_syntax` 0.0.349 | `syn` 3.0.9 |
|---|---|---|
| Lossless | yes: tree text == source, comments and whitespace are tokens | no: comments dropped, doc comments become attributes |
| Broken code | partial tree plus errors, keeps going | whole file fails |
| Offsets | `TextRange` in bytes | `Span::byte_range()` (needs `proc-macro2` feature `span-locations`) |
| Columns | bytes (via `ra_ap_line-index`) | characters (`LineColumn.column`), must be converted |
| Speed | 4,249-line file in 6 ms (release) | not measured |
| Build, clean, 2 jobs | 6.8 s debug, 15 s release, 37 crates | 3.4 s debug, 16 s release |
| MSRV | follows the newest stable Rust (0.0.359 needs 1.98) | low |

**Recommendation: `ra_ap_syntax`.** Lossless trees are what L1 and text edits need, and it
tolerates broken code. Costs:

- Its MSRV tracks the latest stable Rust. The workspace says `rust-version = "1.85"`; with that,
  cargo picks `ra_ap_syntax` 0.0.273. Pin one version and bump it deliberately, or raise the
  workspace MSRV for the frontend crate only.
- `ra-ap-rustc_lexer` asserts at compile time that `unicode-ident` and `unicode-properties` use
  the same Unicode version. The newest `unicode-ident` (1.0.27, Unicode 18) breaks it;
  1.0.24 (Unicode 17) works. Pin it in `Cargo.lock` (`cargo update unicode-ident --precise
  1.0.24`); this is shared with `syn` and everything else in the workspace.

### Node mapping (L0)

| `SyntaxKind` | IR kind | qualified_name (rustc def path, relative to the file's module) |
|---|---|---|
| `SOURCE_FILE` | `Module` (root) | none |
| `MODULE` (inline `mod`) | `Module` | `inner` |
| `FN` | `Function` | `f`, `f::nested`, `<S>::m`, `<S as T>::r`, `T::d` |
| `CLOSURE_EXPR`, async/gen blocks | `Closure` | `<S>::m::{closure#0}` |
| `IMPL`, `TRAIT`, `STRUCT`, `ENUM`, … | `Other("IMPL")`, … | `<S>`, `<S as T>`, `T` |

Closure numbering, checked against rustc's v0 symbols: one counter per enclosing fn or closure,
in source order, and async blocks share it (`let a = |x| {..}; let b = ..; async {..}; || 5`
gives `{closure#0}`, `#1`, `#2`, `#3`; a closure inside `a` is `{closure#0}::{closure#0}`).
llvm-cov names are mangled (`_RNvCs…_4shed6divide`); the join will demangle them
(`rustc-demangle`) and drop crate and module prefixes before matching names. A default trait
method shows up per implementing type (`<S as T>::d`), and generic code per instance.

### Spans

- ra attaches comments that directly precede an item (no blank line between) to the item:
  `// note\nfn b() {}` makes `FN` start at the comment. The IR span starts at the first token
  that is not whitespace or a plain comment, so doc comments and attributes are included
  (like Python decorators), plain comments are not.
- Trailing comments are not attached.
- Macro bodies are token trees: functions made by `macro_rules!` or by attributes have no node.
  `#[test]` makes a closure whose lcov `FN` line is the attribute line; since spans include
  attributes, the join attributes those lines to the test function.

### Body spans

Mirror the Python rule: the body starts at the first token after `{` that is not whitespace or
a comment (the `}` for an empty body). `None` when that token is on the `fn` keyword's line.
This matches llvm-cov, which counts the signature line on entry; the lcov FN fallback already
uses `start + 1`.

## Open questions

1. Rust `qualified_name`: rustc's def path (`<S as T>::r`, recommended above, matches coverage
   tools) or a friendlier `S::r` for reports? The report could pretty-print either.
2. Rust kinds: map `impl`/`trait` to `Class`, or keep them `Other` (current proposal) until an
   analysis needs a shared "type body" kind?
3. M2 switch: accept the decorator-start difference above (proposed: yes, the IR is right).
4. Out-of-process protocol: one file per call (prototype) or JSON lines over many files
   (proposed for the real runner).
