# Reading code

`sundgren outline FILE...` prints what sundgren reads in Python, Rust and C files: the items
(functions, classes, impls, ...) and the lines each spans. `--tokens` also lists the tokens line
by line; `--format json` gives `sundgren.outline/0`.

```console
$ sundgren outline src/syntax/c.rs
src/syntax/c.rs (rust)
  6-7        const NOT_NAMES
  9-13       struct Parser
  15-207     impl Parser<'_>
    16-18      function text
    ...
```

In VS Code, **sundgren: Show Outline** opens a panel beside the editor with the active file as
sundgren reads it: each item as a coloured band beside its lines (nested items further right, colour
by kind), a label on its first line, and comments, strings, numbers and C preprocessor lines
coloured as the lexer took them. Click a band or label to go to the item. The panel follows the
active editor and reloads when the file is saved.

Nothing is used by `sundgren contain` yet. The point of this step is to check the reading first.

## What it reads

Hand-written, no dependencies. Two levels:

1. **Tokens.** One lexer, with comment, string and directive syntax per language: identifiers,
   numbers, strings, comments, C preprocessor lines, punctuation. It never fails; broken or
   unfinished code still comes out as tokens.
2. **Items**, with names and line spans:
   - Python: `def`, `async def`, `class`, nested by indentation; decorators belong to the item.
   - Rust: `fn`, `struct`, `enum`, `union`, `trait`, `impl`, `mod`, `const`, `static`, `type`,
     `macro_rules!`; attributes and `///` docs belong to the item. Items in `impl`, `trait` and
     `mod` blocks are children; items inside function bodies are not read.
   - C: function definitions, `struct`/`union`/`enum` definitions (typedefs named by the typedef),
     `#define`. Prototypes and variables are not items. `extern "C" {` blocks are read through.

Files are chosen by extension: `.py`, `.pyi`, `.rs`, `.c`, `.h`. Others are not read.

A full syntax tree (expressions, statements) is not built. It would allow matching code with
renamed variables, but it is hard to check by eye, so it waits.

## Checked on

No crashes on 7,500 files: 1,500 from the Python standard library, 3,000 kernel C sources and
headers, 3,000 Rust files of crates. Outlines of a few were compared with the source by hand.

## Known limits

- C: code inside `#if`/`#else` is read as if both branches were there; macros that expand to
  declarations can be mistaken for functions.
- Rust: nested items in function bodies and `extern` blocks are skipped.
- Python: lines are joined inside brackets and after `\`; nothing else of the grammar is read.

## Next

Use the reading in `contain`: drop comment lines (license headers), compare short lines by
tokens, and report containment per item, which can also show an item that moved between files.
