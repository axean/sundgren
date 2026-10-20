"""Code on the docs pages only includes tested files from docs/examples/; examples are complete."""

from __future__ import annotations

import contextlib
import json
import os
import re
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator

import docs_examples

DOCS = docs_examples.ROOT / "docs"
CHANGELOG = docs_examples.ROOT / "CHANGELOG.md"
NOT_PAGES = {"examples", "_build"}
PAGES = sorted(p for p in DOCS.rglob("*.md") if p.relative_to(DOCS).parts[0] not in NOT_PAGES)
DIRS = docs_examples.example_dirs()

INCLUDE_OPTIONS = {"language", "lines", "caption", "emphasize-lines", "linenos", "lineno-start"}
CODE_ROLES = {"code", "literal", "samp", "kbd", "command", "file"}
OPTION = re.compile(r":([\w-]+):(?:\s.*)?")
DIRECTIVE = re.compile(r"\{([\w-]+)\}\s*(.*)")
HTML_CODE = re.compile(r"<(pre|code|samp|kbd)\b", re.IGNORECASE)


def md_parser() -> Any:
    pytest.importorskip("myst_parser", reason="myst-parser needs Python >= 3.12")
    from markdown_it.renderer import RendererHTML
    from myst_parser.config.main import MdParserConfig
    from myst_parser.parsers.mdit import create_md_parser

    return create_md_parser(MdParserConfig(enable_extensions={"colon_fence"}), RendererHTML)


def example_files() -> set[Path]:
    files: set[Path] = set()
    for path in DIRS:
        with contextlib.suppress(docs_examples.ExampleError):  # test_example_complete reports it
            files |= docs_examples.tested_files(docs_examples.load(path))
    return files


def resolve(page: Path, arg: str) -> Path:
    """A directive path the way Sphinx resolves it: `/x` from docs/, else from the page's dir."""
    base = DOCS if arg.startswith("/") else page.parent
    return Path(os.path.normpath(base / arg.lstrip("/")))


def split_options(content: str) -> tuple[list[str], str]:
    """Leading `:key: value` option names and the rest of a directive's body."""
    lines = content.splitlines()
    names: list[str] = []
    while lines and (m := OPTION.fullmatch(lines[0])):
        names.append(m.group(1))
        lines.pop(0)
    return names, "\n".join(lines)


def check_directive(page: Path, line: int, info: str, content: str, md: Any) -> list[str]:
    where = f"{page.relative_to(DOCS)}:{line}"
    m = DIRECTIVE.fullmatch(info)
    if not m:
        return [f"{where}: code block with inline content; include a file from docs/examples/"]
    name, arg = m.groups()
    options, body = split_options(content)
    if body.lstrip().startswith("---"):
        return [f"{where}: {{{name}}}: use `:key: value` options, not a YAML block"]
    if name == "details":
        return check_markdown(page, body, md, offset=line + len(options))
    if name == "literalinclude":
        problems = [
            f"{where}: {{literalinclude}} option :{o}: not allowed"
            for o in options
            if o not in INCLUDE_OPTIONS
        ]
        if body.strip():
            problems.append(f"{where}: {{literalinclude}} has inline content")
        target = resolve(page, arg)
        if not target.is_file():
            problems.append(f"{where}: {arg!r} does not exist")
        elif target not in example_files():
            problems.append(f"{where}: {arg!r} is not a tested file of a docs example")
        return problems
    if name == "include":
        # Only the changelog page: CHANGELOG.md is a record, not a runnable example.
        if resolve(page, arg) != CHANGELOG or options or body.strip():
            return [f"{where}: {{include}} is only allowed for CHANGELOG.md, without options"]
        return []
    if name == "sundgren-graphic":
        problems = [f"{where}: {{sundgren-graphic}} takes no options or body"] if content else []
        target = resolve(page, arg)
        if (
            target.parent != docs_examples.EXAMPLES
            or not (target / docs_examples.MANIFEST).is_file()
        ):
            problems.append(f"{where}: {arg!r} is not a docs example directory")
        return problems
    return [f"{where}: directive {{{name}}} not allowed on a page"]


def check_markdown(page: Path, text: str, md: Any, offset: int = 0) -> list[str]:
    """Code on a page that isn't an include of a tested file; `offset` is text's first line - 1."""
    problems: list[str] = []
    line = offset + 1
    for token in md.parse(text):
        line = offset + token.map[0] + 1 if token.map else line
        at = f"{page.relative_to(DOCS)}:{line}"
        if token.type in ("fence", "colon_fence"):
            problems += check_directive(page, line, token.info.strip(), token.content, md)
        elif token.type == "code_block":
            problems.append(f"{at}: indented code block")
        elif token.type == "html_block" and HTML_CODE.search(token.content):
            problems.append(f"{at}: HTML code element")
        for child in token.children or ():
            spaced = bool(re.search(r"\s", child.content))
            if child.type == "code_inline" and spaced:
                problems.append(f"{at}: inline code with whitespace: `{child.content}`")
            elif child.type == "myst_role" and child.meta["name"] in CODE_ROLES and spaced:
                problems.append(f"{at}: {{{child.meta['name']}}} role with whitespace")
            elif child.type == "html_inline" and HTML_CODE.search(child.content):
                problems.append(f"{at}: HTML code element")
    return problems


def test_pages_exist() -> None:
    assert PAGES, f"no Markdown pages under {DOCS}"
    others = [
        p
        for p in DOCS.rglob("*")
        if p.suffix in (".rst", ".ipynb") and p.relative_to(DOCS).parts[0] not in NOT_PAGES
    ]
    assert not others, f"pages must be Markdown, so the checks below cover them: {others}"


@pytest.mark.parametrize("page", PAGES, ids=[str(p.relative_to(DOCS)) for p in PAGES])
def test_page_has_only_tested_code(page: Path) -> None:
    problems = check_markdown(page, page.read_text(encoding="utf-8"), md_parser())
    assert not problems, "untested code on a docs page:\n" + "\n".join(problems)


REPORT_TXT = "examples/skip_divide/report.txt"
GOOD = """\
Skip `test_divide` and run `--full`:

```{literalinclude} examples/skip_divide/commands.sh
:language: sh
:caption: commands
```

```{sundgren-graphic} examples/skip_divide
```

````{details} Full report
Some prose.

```{literalinclude} /examples/skip_divide/report.json
:language: json
:lines: 1-20
```
````
"""

INLINE = "inline content"
BAD = {  # page text -> expected problem
    "plain fence": ("```sh\nsundgren run\n```\n", INLINE),
    "tilde fence": ("~~~\nx\n~~~\n", INLINE),
    "bare fence": ("```\nx\n```\n", INLINE),
    "indented": ("Text\n\n    x = 1\n", "indented code block"),
    "in list": ("- item\n\n  ```py\n  x\n  ```\n", INLINE),
    "in blockquote": ("> [!NOTE]\n> ```py\n> x\n> ```\n", INLINE),
    "colon fence": (":::{code-block} sh\nx\n:::\n", "{code-block} not allowed"),
    "code-block": ("```{code-block} sh\nx\n```\n", "{code-block} not allowed"),
    "eval-rst": ("```{eval-rst}\n.. code-block:: sh\n\n   x\n```\n", "{eval-rst} not allowed"),
    "include": ("```{include} examples/README.md\n```\n", "only allowed for CHANGELOG.md"),
    "include options": (
        "```{include} ../CHANGELOG.md\n:start-line: 3\n```\n",
        "only allowed for CHANGELOG.md",
    ),
    "missing file": ("```{literalinclude} examples/skip_divide/nope.txt\n```\n", "not exist"),
    "outside examples": ("```{literalinclude} ../README.md\n```\n", "not a tested file"),
    "untested file": ("```{literalinclude} examples/README.md\n```\n", "not a tested file"),
    "prepend": (f"```{{literalinclude}} {REPORT_TXT}\n:prepend: $ x\n```\n", ":prepend: not"),
    "body": (f"```{{literalinclude}} {REPORT_TXT}\nx\n```\n", INLINE),
    "yaml options": (f"```{{literalinclude}} {REPORT_TXT}\n---\nx: 1\n---\n```\n", "YAML"),
    "graphic not example": ("```{sundgren-graphic} examples\n```\n", "not a docs example"),
    "graphic body": ("```{sundgren-graphic} examples/skip_divide\n:x: y\n```\n", "no options"),
    "details": ("````{details} x\n```sh\nx\n```\n````\n", "page.md:2: code block"),
    "inline span": ("Run `sundgren run --full`.\n", "inline code with whitespace"),
    "code role": ("Run {code}`sundgren run`.\n", "{code} role"),
    "html pre": ("<pre>sundgren run</pre>\n", "HTML code"),
    "html code": ("Run <code>sundgren</code>.\n", "HTML code"),
}


def test_checker_accepts_includes() -> None:
    assert check_markdown(DOCS / "page.md", GOOD, md_parser()) == []


def test_checker_accepts_changelog_include() -> None:
    page = "```{include} ../CHANGELOG.md\n```\n"
    assert check_markdown(DOCS / "changelog.md", page, md_parser()) == []


@pytest.mark.parametrize(("text", "expected"), BAD.values(), ids=BAD.keys())
def test_checker_rejects(text: str, expected: str) -> None:
    problems = check_markdown(DOCS / "page.md", text, md_parser())
    assert len(problems) == 1 and expected in problems[0], problems


def test_examples_exist() -> None:
    assert DIRS, f"no examples under {docs_examples.EXAMPLES}"


@pytest.mark.parametrize("path", DIRS, ids=[p.name for p in DIRS])
def test_example_complete(path: Path) -> None:
    example = docs_examples.load(path)  # raises ExampleError for a malformed example
    absent = [n for n in example.outputs if not (path / n).is_file()]
    assert not absent, (
        f"{example.name}: no recording of {absent}; run tools/docs_examples.py --update"
    )
    extra = docs_examples.unexpected_files(example)
    assert not extra, f"{example.name}: files outside the example format: {extra}"
    schema = json.loads((docs_examples.ROOT / "schema/delta.schema.json").read_text("utf-8"))
    validator = Draft202012Validator(schema)
    for name in example.outputs:
        text = (path / name).read_bytes().decode("utf-8")
        assert docs_examples.normalize(name, text, None) == text, f"{name} is not normalized"
        if name.endswith(".json"):
            validator.validate(json.loads(text))  # pyright: ignore[reportUnknownMemberType]
