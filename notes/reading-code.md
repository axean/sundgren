# Reading code

`sundgren outline FILE...` prints what sundgren reads in Python, Rust, C, C++, JavaScript and
TypeScript files: the items (functions, classes, impls, ...) and the lines each spans. `--tokens`
also lists the tokens line by line; `--format json` gives `sundgren.outline/0`.

```console
$ sundgren outline src/syntax.rs
src/syntax.rs (rust)
  5-5        module tree
  7-7        import serde::Serialize
  9-22       enum Kind
    12-13      variant Ident
    ...
```

In VS Code, **sundgren: Show Outline** opens a panel beside the editor with the active file as
sundgren reads it: each item as a coloured band beside its lines (nested items further right, colour
by kind), a label on its first line, and comments, strings, numbers and C preprocessor lines
coloured. Click a band or label to go to the item. The panel follows the active editor and reloads
when the file is saved.

## How it reads

With [tree-sitter](https://tree-sitter.github.io/): one engine (`src/syntax/tree.rs`) for every
language. A language is a tree-sitter grammar and a query (`src/syntax/queries/`) that tags its
nodes: `@item.KIND` is an item, `@binding.KIND` an item except inside a function, `@name` names it,
`@lead` is a doc comment or attribute that belongs to the item after it, `@token.KIND` a comment,
string, number or directive. C++ adds to the C query; JavaScript and TypeScript share one.

The same rules for every language:

- Items nest by where they are.
- Inside a function, only definitions are items (functions, classes, ...), not variables or
  imports. An anonymous function counts as a function.
- A value (variable, constant, field, ...) holds no items: what's in it belongs to its parent.
- A variable in a class, struct, ... is a field.
- A call at the top of a file is an item.

Item kinds: function, prototype (no body), class, struct, union, enum, variant, field, trait,
interface, impl, module, type, const, static, variable, macro, import, call.

Files are chosen by extension: `.py` `.pyi`, `.rs`, `.c` `.h`, `.cc` `.cpp` `.cxx` `.hh` `.hpp`
`.hxx` and a few more for C++, `.js` `.mjs` `.cjs` `.jsx`, `.ts` `.mts` `.cts` `.tsx`. Others are not
read.

Another language is a grammar crate and a query file.

## Checked on

`uv run utils/outline_check.py LANGUAGE` reads a corpus of system files (read only) and compares the
definitions with each language's own parser. On 2026-11-07:

| Language | Files | Found of the parser's | Also in the parser's |
| --- | --- | --- | --- |
| Python (`ast`) | 1,500 | 100 % | 99.9 % |
| Rust (rust-analyzer) | 400 | 100 % | 100 % |
| TypeScript, JavaScript (the TypeScript compiler) | 1,500 | 99.6 % | 93.0 % |

The TypeScript differences are mostly in two minified bundles, where items nested in expressions
get other qualified names. C and C++ have no parser to compare with; no crashes on 1,500 files.

## Known limits

- C, C++: no preprocessor. Macros around declarations (`__THROW`, `_GLIBCXX_VISIBILITY(default)`)
  can hide an item or make one of the wrong name; both branches of `#if`/`#else` are read.
- Code that doesn't parse: tree-sitter recovers, but an unclosed item is lost (what follows it is
  read).
- TypeScript: `global { }` inside `declare module` is not read.

## In containment

`sundgren contain` uses the reading where it can. For files in a language sundgren reads, at each
side's revision:

- every added line gets the innermost item around it and whether it is only a comment;
- lines that are only a comment are left out of the work by default (`--comments count` keeps
  them); each side reports its `comment_lines`;
- each direction gets `items`: per item (or the lines outside items, per file), how many lines
  were found, and the `counterpart`, the file and item on the other side most of them were found
  in, with both items' lines;
- `--lines` lists every added line, how it was found and where (the inspect view).

Matching is global: a line is found anywhere in the other side's work, so whether and how well
lines are found doesn't depend on items. Where a line is found does: among equally good lines of
the other side it takes the nearest, in the same file and item first, then the same file, then an
item of the same name elsewhere, then the first in diff order. Exact matches are made nearest
first over all lines. So a copy of the code in another file doesn't take the lines of the
original. A file sundgren doesn't read gets no items, and its comments count.

On the old synthetic demo, unrelated branches scored 10–17 % from a shared license header and 0 %
with comments left out. On a real 91-line fix branch against its 4.2k-line dev branch, every item
of the fix maps to the item of the same name in dev; the run takes 0.06 s.
