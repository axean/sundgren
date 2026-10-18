"""ast-based function spans for Python source."""

from __future__ import annotations

import ast
import warnings

from sundgren.core.symbols import Span

_FunctionNode = ast.FunctionDef | ast.AsyncFunctionDef
# Defs only occur in statement blocks, so expressions are never walked.
_Block = ast.stmt | ast.excepthandler | ast.match_case

_BOM = "\N{ZERO WIDTH NO-BREAK SPACE}"


class PythonSymbolizer:
    """Spans for every def/async def, named like `__qualname__`. Lambdas are skipped."""

    def functions(self, source: str) -> list[Span]:
        try:
            # The target's SyntaxWarnings (e.g. invalid escapes) must neither print nor, under
            # -W error, fail the parse.
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                tree = ast.parse(source.removeprefix(_BOM))
        # ValueError: null bytes on 3.11. RecursionError/MemoryError: source too deeply nested.
        except (ValueError, RecursionError, MemoryError) as exc:
            raise SyntaxError(str(exc)) from exc
        return sorted(_collect(tree), key=lambda s: (s.start, s.name))


def _collect(tree: ast.Module) -> list[Span]:
    out: list[Span] = []
    # Explicit stack: an elif chain nests one ast.If per branch.
    stack: list[tuple[ast.AST, str]] = [(tree, "")]
    while stack:
        node, prefix = stack.pop()
        for child in ast.iter_child_nodes(node):
            if isinstance(child, _FunctionNode):
                name = prefix + child.name
                out.append(_span(child, name))
                stack.append((child, f"{name}.<locals>."))
            elif isinstance(child, ast.ClassDef):
                stack.append((child, f"{prefix}{child.name}."))
            elif isinstance(child, _Block):
                stack.append((child, prefix))
    return out


def _span(node: _FunctionNode, name: str) -> Span:
    end = node.end_lineno if node.end_lineno is not None else node.lineno
    return Span(name, _first_line(node), end, _body_start(node))


def _body_start(node: _FunctionNode) -> int | None:
    body = node.body
    first = body[1] if len(body) > 1 and _is_docstring(body[0]) else body[0]
    return None if first.lineno == node.lineno else _first_line(first)


def _first_line(stmt: ast.stmt) -> int:
    """Statement start, counting decorators (ast puts lineno on the def/class line)."""
    if isinstance(stmt, _FunctionNode | ast.ClassDef):
        return min((d.lineno for d in stmt.decorator_list), default=stmt.lineno)
    return stmt.lineno


def _is_docstring(stmt: ast.stmt) -> bool:
    return (
        isinstance(stmt, ast.Expr)
        and isinstance(stmt.value, ast.Constant)
        and isinstance(stmt.value.value, str)
    )
