<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/axean/sundgren/main/docs/_static/brand/sundgren-logo-dark.svg">
  <img alt="sundgren" src="https://raw.githubusercontent.com/axean/sundgren/main/docs/_static/brand/sundgren-logo.svg" width="420">
</picture>

[![CI](https://github.com/axean/sundgren/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/axean/sundgren/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/axean/sundgren/badges/coverage.json)](https://github.com/axean/sundgren/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/sundgren)](https://pypi.org/project/sundgren/)
[![License](https://img.shields.io/pypi/l/sundgren)](https://github.com/axean/sundgren/blob/main/LICENSE)
[![Docs](https://img.shields.io/badge/docs-axean.github.io%2Fsundgren-blue)](https://axean.github.io/sundgren/)
[![Checked with pyright](https://microsoft.github.io/pyright/img/pyright_badge.svg)](https://microsoft.github.io/pyright/)
[![ESLint](https://img.shields.io/badge/ESLint-checked-4B32C3.svg?logo=eslint&logoColor=white)](https://eslint.org)
[![Types: checked](https://img.shields.io/badge/types-checked-3178C6.svg?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
[![pre-commit](https://img.shields.io/badge/pre--commit-enabled-brightgreen?logo=pre-commit)](https://github.com/pre-commit/pre-commit)

sundgren is a multi-language code review tool: a similarity check between git branches, from
their common ancestor. Once work is rebased, squashed or extracted into its own merge request, git
no longer knows it is the same work; sundgren compares what each branch added since it left a
common base, and shows which branch already holds what, what was moved or renamed, and what is
left. A preview: how it decides what counts as the same work may still
change.

- **VS Code extension**: containment of every branch in a target, inspect a branch line by line,
  and a map of branches around a base. On the
  [Marketplace](https://marketplace.visualstudio.com/items?itemName=axean.sundgren).
- **Command line**: `pip install sundgren`, then `sundgren contain A B --base main`,
  `sundgren graph`, `sundgren outline FILE`.

[Documentation](https://axean.github.io/sundgren/), with live demos.

## Languages

- **Any text file** is compared line by line.
- **Python** (`.py`, `.pyi`), **Rust** (`.rs`) and **C** (`.c`, `.h`) are also read as code:
  containment per function, class or other item, with where each went in the other branch (tagged
  _moved_ or _renamed_), and comment lines (license headers, doc comments) left out.
  - Python: `def`, `class`.
  - Rust: `fn`, `struct`, `enum`, `union`, `trait`, `impl`, `mod`, `const`, `static`, `type`,
    `macro_rules!`.
  - C: functions, `struct`, `union`, `enum`, `#define`.

## Development

How to build, test and release sundgren:
[DEVELOPMENT.md](https://github.com/axean/sundgren/blob/main/DEVELOPMENT.md).

## License

Apache-2.0. See [LICENSE](https://github.com/axean/sundgren/blob/main/LICENSE).
