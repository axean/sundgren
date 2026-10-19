"""Python frontend: sundgren IR (L0) JSON for one source file, built with Python's own `ast`.

Meant to run under the target project's interpreter, so its grammar always matches. Stdlib only,
Python 3.8+. Usage: `python ir_frontend.py FILE [PATH]`, where PATH is the repo-relative path to
record (default FILE). Exit 0: IR JSON on stdout. Exit 1: the source does not parse (message on
stderr). See design/ir-mapping.md.
"""

from __future__ import annotations

import ast
import bisect
import codecs
import hashlib
import io
import json
import platform
import sys
import tokenize
import warnings
from typing import Any

SCHEMA = "sundgren.ir/1"
PRODUCER = "sundgren-python-ast/1"

_Named = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)


class Source:
    """The file's bytes, with ast (line, UTF-8 column) to byte offset and back."""

    def __init__(self, data: bytes) -> None:
        self.data = data
        self.encoding = tokenize.detect_encoding(io.BytesIO(data).readline)[0]
        self.bom = len(codecs.BOM_UTF8) if data.startswith(codecs.BOM_UTF8) else 0
        # Python ends lines at \n, \r\n and a lone \r.
        starts = [0]
        i, n = 0, len(data)
        while i < n:
            c = data[i]
            if c == 0x0A or c == 0x0D:
                if c == 0x0D and i + 1 < n and data[i + 1] == 0x0A:
                    i += 1
                starts.append(i + 1)
            i += 1
        self.starts = starts

    def line(self, line: int) -> tuple[int, bytes]:
        """Byte offset and text (no BOM, newline kept) of a 1-based line."""
        start = self.starts[line - 1] + (self.bom if line == 1 else 0)
        end = self.starts[line] if line < len(self.starts) else len(self.data)
        return start, self.data[start:end]

    def byte(self, line: int, col: int) -> int:
        """File byte offset of an ast position; `col` counts UTF-8 bytes of the decoded line."""
        start, raw = self.line(line)
        if self.encoding in ("utf-8", "utf-8-sig"):
            return start + col
        prefix = raw.decode(self.encoding).encode("utf-8")[:col].decode("utf-8")
        return start + len(prefix.encode(self.encoding))

    def pos(self, byte: int) -> dict[str, int]:
        line = bisect.bisect_right(self.starts, byte)
        return {"byte": byte, "line": line, "col": byte - self.starts[line - 1] + 1}

    def span(self, start: int, end: int) -> dict[str, dict[str, int]]:
        return {"start": self.pos(start), "end": self.pos(end)}

    def at_sign(self, line: int) -> int:
        """Byte offset of the `@` of a decorator whose expression starts on `line`.

        ast positions a decorator at its expression, after the `@` and any `(`. The `@` starts
        a logical line, so it is the first non-blank on its physical line, at or above `line`.
        """
        for ln in range(line, 0, -1):
            start, raw = self.line(ln)
            stripped = raw.lstrip(b" \t\f")
            if stripped.startswith(b"@"):
                return start + len(raw) - len(stripped)
        raise ValueError(f"no '@' above line {line}")


def _start(src: Source, node: ast.AST) -> int:
    """Start of a statement or expression, counting decorators."""
    if isinstance(node, _Named) and node.decorator_list:
        return min(src.at_sign(d.lineno) for d in node.decorator_list)
    return src.byte(node.lineno, node.col_offset)  # type: ignore[attr-defined]


def _end(src: Source, node: ast.AST) -> int:
    end_line = node.end_lineno  # type: ignore[attr-defined]
    end_col = node.end_col_offset  # type: ignore[attr-defined]
    return src.byte(end_line, end_col)


def _is_docstring(stmt: ast.stmt) -> bool:
    return (
        isinstance(stmt, ast.Expr)
        and isinstance(stmt.value, ast.Constant)
        and isinstance(stmt.value.value, str)
    )


def _body_start(
    src: Source, node: ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda
) -> int | None:
    """Where the counted body starts: the prototype symbolizer's rule. None for a one-liner."""
    if isinstance(node, ast.Lambda):
        first: ast.AST = node.body
    else:
        body = node.body
        first = body[1] if len(body) > 1 and _is_docstring(body[0]) else body[0]
    if first.lineno == node.lineno:  # type: ignore[attr-defined]
        return None
    return _start(src, first)


def build(data: bytes, path: str) -> dict[str, Any]:
    """The IR of `data` as a JSON-ready dict. Raises SyntaxError if it does not parse."""
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            tree = ast.parse(data)
    # ValueError: null bytes on 3.11. RecursionError/MemoryError: source too deeply nested.
    except (ValueError, RecursionError, MemoryError) as exc:
        raise SyntaxError(str(exc)) from exc
    src = Source(data)

    # (start, -end, seq, node, qualified_name, parent seq). Parent is the nearest enclosing IR
    # node; the name follows Python's scopes, which differ (a decorator's lambda is named in the
    # outer scope but lies inside the decorated def).
    found: list[tuple[int, int, int, ast.AST, str, int]] = []
    # (node to visit, scope prefix for its name, parent seq). Explicit stack: elif chains and
    # long expressions nest thousands deep.
    stack: list[tuple[ast.AST, str, int]] = [(c, "", 0) for c in ast.iter_child_nodes(tree)]
    while stack:
        node, prefix, parent = stack.pop()
        if not isinstance(node, (*_Named, ast.Lambda)):
            stack.extend((c, prefix, parent) for c in ast.iter_child_nodes(node))
            continue
        name = "<lambda>" if isinstance(node, ast.Lambda) else node.name
        qual = prefix + name
        seq = len(found) + 1
        found.append((_start(src, node), -_end(src, node), seq, node, qual, parent))
        inner = qual + ("." if isinstance(node, ast.ClassDef) else ".<locals>.")
        # Only the body is in the new scope; decorators, defaults and bases are outside.
        body: Any = node.body
        in_body = {id(b) for b in (body if isinstance(body, list) else [body])}
        stack.extend(
            (c, inner if id(c) in in_body else prefix, seq) for c in ast.iter_child_nodes(node)
        )
    found.sort()

    seen: dict[str, int] = {}
    ids = {0: path}
    nodes: list[dict[str, Any]] = [
        {
            "id": path,
            "parent": None,
            "kind": "Module",
            "span": src.span(0, len(data)),
            "body_span": None,
            "name": None,
            "qualified_name": None,
            "lang": {"ast": "Module"},
        }
    ]
    for start, neg_end, seq, node, qual, parent in found:
        k = seen.get(qual, 0)
        seen[qual] = k + 1
        ids[seq] = f"{path}::{qual}" if k == 0 else f"{path}::{qual}#{k}"
        body_span = None
        if isinstance(node, ast.ClassDef):
            kind = "Class"
        else:
            kind = "Closure" if isinstance(node, ast.Lambda) else "Function"
            body = _body_start(src, node)  # type: ignore[arg-type]
            if body is not None:
                body_span = src.span(body, -neg_end)
        nodes.append(
            {
                "id": ids[seq],
                "parent": ids[parent],
                "kind": kind,
                "span": src.span(start, -neg_end),
                "body_span": body_span,
                "name": qual.rsplit(".", 1)[-1],
                "qualified_name": qual,
                "lang": {"ast": type(node).__name__},
            }
        )
    return {
        "schema": SCHEMA,
        "path": path,
        "language": "python",
        "producer": f"{PRODUCER} {platform.python_implementation()} {platform.python_version()}",
        "source_sha256": hashlib.sha256(data).hexdigest(),
        "newlines": "universal",
        "nodes": nodes,
    }


def main(argv: list[str]) -> int:
    if len(argv) not in (2, 3):
        print("usage: ir_frontend.py FILE [PATH]", file=sys.stderr)
        return 2
    with open(argv[1], "rb") as f:
        data = f.read()
    try:
        ir = build(data, argv[2] if len(argv) == 3 else argv[1])
    except SyntaxError as exc:
        print(f"{argv[1]}: {exc}", file=sys.stderr)
        return 1
    sys.stdout.write(json.dumps(ir, separators=(",", ":")) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
