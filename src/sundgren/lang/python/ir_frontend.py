"""Python frontend: sundgren IR JSON for source files, built with Python's own `ast`.

Meant to run under the target project's interpreter, so its grammar always matches. Stdlib only,
Python 3.8+.

`python ir_frontend.py [--l1] FILE [PATH]`: PATH is the repo-relative path to record (default
FILE). `--l1` adds L1 nodes (every statement and expression, with labels and local bindings).
Exit 0: IR JSON on stdout. Exit 1: the source does not parse (message on stderr).

`python ir_frontend.py --batch`: first writes `{"ready": PRODUCER}`, then reads JSON lines
`{"file": ..., "path": ..., "l1": bool}` (`l1` optional) from stdin and writes one line per
request: the IR, or `{"path": ..., "error": ..., "kind": "syntax" | "unreadable" | "internal"}`.
A failing file does not stop the batch. See design/ir-mapping.md.
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


def build(data: bytes, path: str, l1: bool = False) -> dict[str, Any]:
    """The IR of `data` as a JSON-ready dict, with L1 nodes if `l1`. Raises SyntaxError if it
    does not parse."""
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

    nodes = _l1_nodes(tree, src, data, path, found) if l1 else _l0_nodes(src, data, path, found)
    ir: dict[str, Any] = {
        "schema": SCHEMA,
        "path": path,
        "language": "python",
        "producer": _producer(),
        "source_sha256": hashlib.sha256(data).hexdigest(),
        "newlines": "universal",
        "nodes": nodes,
    }
    if l1:
        ir["levels"] = ["L1"]
    return ir


def _l0_nodes(
    src: Source, data: bytes, path: str, found: list[tuple[int, int, int, ast.AST, str, int]]
) -> list[dict[str, Any]]:
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
    return nodes


# ---- L1: every statement and expression, with labels and local bindings -------------------

_OPS = (ast.boolop, ast.operator, ast.unaryop, ast.cmpop)
_FUNCS = (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)
_TYPE_PARAMS = ("TypeVar", "ParamSpec", "TypeVarTuple")  # bound in an annotation scope
_COMPS = (ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)
_SHARED_KINDS = {
    "If": "If",
    "For": "Loop",
    "AsyncFor": "Loop",
    "While": "Loop",
    "Match": "Match",
    "match_case": "Arm",
    "Return": "Return",
    "Call": "Call",
    "Assign": "Assign",
    "AugAssign": "Assign",
    "AnnAssign": "Assign",
    "Expr": "Expr",
}


class _Scope:
    """A Python name scope: module, class, function (def or lambda) or comprehension."""

    def __init__(self, kind: str, node: ast.AST | None, parent: _Scope | None) -> None:
        self.kind = kind
        self.node = node
        self.parent = parent
        self.bound: set[str] = set()
        self.globals: set[str] = set()
        self.nonlocals: set[str] = set()

    def owner(self, name: str) -> _Scope | None:
        """The function or comprehension scope binding `name` as seen from here, else None."""
        s: _Scope | None = self
        first = True
        while s is not None and s.kind != "module":
            if s.kind == "class":
                # Class-level names are not locals; enclosing functions skip class scopes.
                if first and name in s.bound:
                    return None
            elif name in s.globals:
                return None
            elif name in s.bound and name not in s.nonlocals:
                return s
            s, first = s.parent, False
        return None

    def callable(self) -> ast.AST | None:
        """The nearest enclosing def or lambda (a comprehension's variables belong to it)."""
        s: _Scope | None = self
        while s is not None and s.kind != "function":
            s = s.parent
        return s.node if s is not None else None


def _arg_nodes(args: ast.arguments) -> list[ast.arg]:
    out = [*getattr(args, "posonlyargs", []), *args.args, *args.kwonlyargs]
    out += [a for a in (args.vararg, args.kwarg) if a is not None]
    return out


def _locals(tree: ast.Module) -> dict[int, tuple[ast.AST, str]]:
    """For each node that binds or uses a function-local name: (its def or lambda, name).

    Python's scoping rules: a name assigned anywhere in a function is local to it unless
    declared global or nonlocal; defaults, decorators, annotations, bases and a comprehension's
    first iterable are evaluated in the enclosing scope; a walrus in a comprehension binds in
    the enclosing function.
    """
    module = _Scope("module", None, None)
    where: dict[int, _Scope] = {}  # node -> the scope its names are looked up in
    uses: list[tuple[ast.AST, str, _Scope]] = []  # (node, name, scope it is looked up from)
    walrus: set[int] = set()

    def binder(scope: _Scope) -> _Scope:
        while scope.kind == "comp" and scope.parent is not None:
            scope = scope.parent
        return scope

    stack: list[tuple[ast.AST, _Scope]] = [(tree, module)]
    while stack:
        node, scope = stack.pop()
        where[id(node)] = scope
        children: list[tuple[ast.AST, _Scope]] = []
        if isinstance(node, _FUNCS):
            inner = _Scope("function", node, scope)
            if not isinstance(node, ast.Lambda):
                scope.bound.add(node.name)
                uses.append((node, node.name, scope))
                children += [(d, scope) for d in node.decorator_list]
                if node.returns is not None:
                    children.append((node.returns, scope))
                children += [(t, scope) for t in getattr(node, "type_params", [])]
            args = node.args
            where[id(args)] = scope
            children += [(d, scope) for d in args.defaults]
            children += [(d, scope) for d in args.kw_defaults if d is not None]
            for a in _arg_nodes(args):
                where[id(a)] = inner
                inner.bound.add(a.arg)
                uses.append((a, a.arg, inner))
                if a.annotation is not None:
                    children.append((a.annotation, scope))
            body = node.body if isinstance(node.body, list) else [node.body]
            children += [(b, inner) for b in body]
        elif isinstance(node, ast.ClassDef):
            scope.bound.add(node.name)
            uses.append((node, node.name, scope))
            inner = _Scope("class", node, scope)
            children += [(c, scope) for c in node.decorator_list + node.bases]
            children += [(k, scope) for k in node.keywords]
            children += [(t, scope) for t in getattr(node, "type_params", [])]
            children += [(b, inner) for b in node.body]
        elif isinstance(node, _COMPS):
            inner = _Scope("comp", node, scope)
            for i, gen in enumerate(node.generators):
                where[id(gen)] = inner
                children.append((gen.iter, scope if i == 0 else inner))
                children.append((gen.target, inner))
                children += [(c, inner) for c in gen.ifs]
            elts = [node.key, node.value] if isinstance(node, ast.DictComp) else [node.elt]
            children += [(e, inner) for e in elts]
        else:
            if isinstance(node, ast.Name):
                if isinstance(node.ctx, (ast.Store, ast.Del)):
                    (binder(scope) if id(node) in walrus else scope).bound.add(node.id)
                uses.append((node, node.id, scope))
            elif isinstance(node, ast.NamedExpr):
                walrus.add(id(node.target))
            elif isinstance(node, ast.Global):
                scope.globals.update(node.names)
            elif isinstance(node, ast.Nonlocal):
                scope.nonlocals.update(node.names)
            elif isinstance(node, ast.alias):
                if node.name != "*":
                    scope.bound.add(node.asname or node.name.split(".")[0])
            else:
                for field in ("name", "rest"):  # except-as, match captures
                    value = getattr(node, field, None)
                    if isinstance(value, str) and type(node).__name__ not in _TYPE_PARAMS:
                        scope.bound.add(value)
                        uses.append((node, value, scope))
            children += [(c, scope) for c in ast.iter_child_nodes(node)]
        stack.extend(reversed(children))

    out: dict[int, tuple[ast.AST, str]] = {}
    for node, name, scope in uses:
        owner = scope.owner(name)
        fn = owner.callable() if owner is not None else None
        if fn is not None:
            out[id(node)] = (fn, name)
    return out


def _label(node: ast.AST, local: bool) -> str:
    """Native class plus scalar fields; operators fold in, ctx and binding names don't."""
    if isinstance(node, ast.Constant):
        parts = [type(node.value).__name__, repr(node.value)]
    elif isinstance(node, ast.Name):
        parts = [] if local else [node.id]
    elif isinstance(node, ast.arg):
        parts = []
    elif isinstance(node, ast.keyword):
        parts = [_kw(node)]
    elif isinstance(node, ast.alias):
        parts = [_alias(node)]
    else:
        parts = []
        for field in node._fields:
            if field in ("ctx", "type_comment"):
                continue
            value = getattr(node, field, None)
            if field in ("name", "rest") and isinstance(value, str):
                # A binding name: def/class names and except-as/match captures.
                if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                    parts.append("as" if local else f"as {value}")
                continue
            if isinstance(value, _OPS):
                parts.append(type(value).__name__)
            elif isinstance(value, list) and value and all(isinstance(v, _OPS) for v in value):
                parts.append(",".join(type(v).__name__ for v in value))
            elif isinstance(value, list) and value and all(isinstance(v, str) for v in value):
                parts.append(",".join(value))
            elif value is not None and not isinstance(value, (ast.AST, list)):
                parts.append(str(value))
    return "".join([type(node).__name__, *(":" + p for p in parts)])


def _kw(node: ast.keyword) -> str:
    return node.arg if node.arg is not None else "**"


def _alias(node: ast.alias) -> str:
    return node.name if node.asname is None else f"{node.name} as {node.asname}"


class _Rec:
    """One IR node under construction."""

    def __init__(self, node: ast.AST, parent: _Rec | None, seq: int) -> None:
        self.node = node
        self.parent = parent
        self.seq = seq
        self.start = -1
        self.end = -1
        self.children: list[_Rec] = []
        self.folds: list[str] = []
        self.id = ""


def _l1_nodes(
    tree: ast.Module,
    src: Source,
    data: bytes,
    path: str,
    found: list[tuple[int, int, int, ast.AST, str, int]],
) -> list[dict[str, Any]]:
    named = {id(node): qual for _, _, _, node, qual, _ in found}
    local = _locals(tree)

    # Build the tree. Positionless nodes (arguments, comprehension, withitem, match_case, and
    # keyword/alias before 3.9/3.10) are spanned by their children, or dropped if they have
    # none; a dropped keyword or alias folds into its parent's label.
    root = _Rec(tree, None, 0)
    root.start, root.end = 0, len(data)
    recs = [root]
    stack: list[tuple[ast.AST, _Rec]] = [(c, root) for c in _children(tree)]
    while stack:
        node, parent = stack.pop()
        if isinstance(node, (*_OPS, ast.expr_context)):
            continue
        positioned = getattr(node, "lineno", None) is not None
        if not positioned and isinstance(node, (ast.keyword, ast.alias)):
            parent.folds.append(
                f"kw={_kw(node)}" if isinstance(node, ast.keyword) else f"alias={_alias(node)}"
            )
            stack.extend((c, parent) for c in _children(node))
            continue
        rec = _Rec(node, parent, len(recs))
        recs.append(rec)
        parent.children.append(rec)
        if positioned:
            rec.start, rec.end = _start(src, node), _end(src, node)
            if rec.end > rec.start and data[rec.end - 1] in b"\r\n":
                # Only a piece of a multi-line f-string ends in its newline; a span must not.
                rec.end -= 2 if data[rec.start : rec.end].endswith(b"\r\n") else 1
        stack.extend((c, rec) for c in _children(node))
    for rec in reversed(recs):
        if rec.start < 0 and rec.children:
            rec.start = min(c.start for c in rec.children)
            rec.end = max(c.end for c in rec.children)
        rec.children = [c for c in rec.children if c.start >= 0]
        rec.children.sort(key=lambda c: (c.start, -c.end, c.seq))

    # Ids in node order, then the nodes.
    order: list[_Rec] = []
    seen: dict[str, int] = {}
    walk = [root]
    while walk:
        rec = walk.pop()
        order.append(rec)
        qual = named.get(id(rec.node))
        if rec.parent is None:
            rec.id = path
        elif qual is not None:
            k = seen.get(qual, 0)
            seen[qual] = k + 1
            rec.id = f"{path}::{qual}" if k == 0 else f"{path}::{qual}#{k}"
        else:
            rec.id = f"{rec.parent.id}/{rec.parent.children.index(rec)}"
        walk.extend(reversed(rec.children))
    id_of = {id(rec.node): rec.id for rec in order}

    nodes = []
    for rec in order:
        node = rec.node
        qual = named.get(id(node))
        cls = type(node).__name__
        body_span = None
        if rec.parent is None:
            kind: Any = "Module"
        elif isinstance(node, ast.ClassDef):
            kind = "Class"
        elif isinstance(node, _FUNCS) and qual is not None:
            kind = "Closure" if isinstance(node, ast.Lambda) else "Function"
            body = _body_start(src, node)
            if body is not None:
                body_span = src.span(body, rec.end)
        else:
            kind = _SHARED_KINDS.get(cls, {"Other": cls})
        bound = local.get(id(node))
        label = _label(node, bound is not None) + "".join("|" + f for f in rec.folds)
        nodes.append(
            {
                "id": rec.id,
                "parent": rec.parent.id if rec.parent is not None else None,
                "kind": kind,
                "span": src.span(rec.start, rec.end),
                "body_span": body_span,
                "name": qual.rsplit(".", 1)[-1] if qual is not None else None,
                "qualified_name": qual,
                "lang": {"ast": cls},
                "l1": {
                    "label": label,
                    "local": {"scope": id_of[id(bound[0])], "name": bound[1]} if bound else None,
                },
            }
        )
    return nodes


def _children(node: ast.AST) -> list[ast.AST]:
    """Child nodes, reversed for a stack."""
    return list(ast.iter_child_nodes(node))[::-1]


def _producer() -> str:
    return f"{PRODUCER} {platform.python_implementation()} {platform.python_version()}"


def _dumps(obj: dict[str, Any]) -> str:
    return json.dumps(obj, separators=(",", ":"))


def batch(requests: Any, out: Any) -> None:
    """Serve `--batch`: JSON-line requests in, one IR or error line out per request."""

    def write(obj: dict[str, Any]) -> None:
        out.write((_dumps(obj) + "\n").encode("ascii"))
        out.flush()

    write({"ready": _producer()})
    for raw in requests:
        req = json.loads(raw)
        path = req["path"]
        try:
            with open(req["file"], "rb") as f:
                data = f.read()
        except OSError as exc:
            write({"path": path, "error": str(exc), "kind": "unreadable"})
            continue
        try:
            ir = build(data, path, bool(req.get("l1")))
        except SyntaxError as exc:
            write({"path": path, "error": str(exc), "kind": "syntax"})
            continue
        except Exception as exc:  # one bad file must not end the batch
            write({"path": path, "error": f"{type(exc).__name__}: {exc}", "kind": "internal"})
            continue
        write(ir)


def main(argv: list[str]) -> int:
    if argv[1:] == ["--batch"]:
        batch(sys.stdin.buffer, sys.stdout.buffer)
        return 0
    l1 = "--l1" in argv
    argv = [a for a in argv if a != "--l1"]
    if len(argv) not in (2, 3):
        print("usage: ir_frontend.py [--l1] FILE [PATH] | --batch", file=sys.stderr)
        return 2
    with open(argv[1], "rb") as f:
        data = f.read()
    try:
        ir = build(data, argv[2] if len(argv) == 3 else argv[1], l1)
    except SyntaxError as exc:
        print(f"{argv[1]}: {exc}", file=sys.stderr)
        return 1
    sys.stdout.write(_dumps(ir) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
