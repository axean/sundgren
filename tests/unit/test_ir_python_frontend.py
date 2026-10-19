"""The Python IR frontend: schema, exact spans, and agreement with PythonSymbolizer."""

from __future__ import annotations

import ast
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator

from sundgren._native import frontend_python
from sundgren.core.symbols import Span
from sundgren.lang.python import ir_frontend
from sundgren.lang.python.symbolizer import PythonSymbolizer

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT / "crates/sundgren-ir/ir.schema.json").read_text(encoding="utf-8"))
FRONTEND = ROOT / "src/sundgren/lang/python/ir_frontend.py"


def build(source: str | bytes) -> dict[str, Any]:
    data = source.encode("utf-8") if isinstance(source, str) else source
    ir = ir_frontend.build(data, "m.py")
    Draft202012Validator(SCHEMA).validate(ir)
    return ir


def function_spans(ir: dict[str, Any]) -> list[Span]:
    """What L0 gives in place of the symbolizer: Function nodes (lambdas are skipped)."""
    out = []
    for n in ir["nodes"]:
        if n["kind"] == "Function":
            body = n["body_span"]
            start, end = n["span"]["start"]["line"], n["span"]["end"]["line"]
            out.append(Span(n["qualified_name"], start, end, body and body["start"]["line"]))
    return sorted(out, key=lambda s: (s.start, s.name))


def text(ir: dict[str, Any], data: bytes, node_id: str) -> bytes:
    span = next(n["span"] for n in ir["nodes"] if n["id"] == node_id)
    return data[span["start"]["byte"] : span["end"]["byte"]]


def corpus() -> list[Path]:
    paths = [*(ROOT / "src/sundgren").rglob("*.py"), *(ROOT / "tests").rglob("*.py")]
    return sorted(p for p in paths if "fixtures" not in p.parts or "shed_py" in p.parts)


def test_agrees_with_symbolizer_on_shed_py() -> None:
    path = ROOT / "tests/fixtures/shed_py/shed/calc.py"
    want = PythonSymbolizer().functions(path.read_text(encoding="utf-8"))
    assert function_spans(build(path.read_bytes())) == want
    assert [s.name for s in want] == ["add", "divide", "classify"]


def test_agrees_with_symbolizer_on_this_repo() -> None:
    mismatched = []
    for path in corpus():
        want = PythonSymbolizer().functions(path.read_text(encoding="utf-8"))
        if function_spans(build(path.read_bytes())) != want:
            mismatched.append(path)
    assert mismatched == []


@pytest.mark.parametrize(
    "source",
    [
        "\ufeffdef f():\n    return 1\n",
        "x = 1\r\ndef f():\r\n    return 1\r\n",
        "def f(): return 1\n@dec\ndef k(): return (\n    4\n)\ndef tail(\n    a): return a\n",
        'def f():\n    """Only."""\n'
        'def g():\n    """Doc."""\n    @dec\n    class K:\n        pass\n',
        "if x == 0:\n    pass\n"
        + "".join(f"elif x == {i}:\n    def g{i}(): pass\n" for i in range(1, 500)),
        "x = " + "+".join(["1"] * 2000) + "\ndef f():\n    return 1\n",
    ],
    ids=["bom", "crlf", "one-liners", "docstrings", "elif-chain", "deep-expr"],
)
def test_agrees_with_symbolizer_on_edge_cases(source: str) -> None:
    assert function_spans(build(source)) == PythonSymbolizer().functions(source)


def test_parenthesized_decorator_starts_at_the_at_sign() -> None:
    # Known difference: ast puts a decorator at its expression, so the symbolizer starts this
    # function on line 2. The IR starts it at the `@`.
    source = "@(\n    dec\n)\ndef f():\n    return 1\n"
    assert PythonSymbolizer().functions(source) == [Span("f", 2, 5, 5)]
    ir = build(source)
    assert function_spans(ir) == [Span("f", 1, 5, 5)]
    assert text(ir, source.encode(), "m.py::f") == source.rstrip("\n").encode()


def test_spans_are_exact_bytes() -> None:
    data = (
        '\ufeffs = "é😀"; f = lambda x: x\r\nclass C:\r\n    @property\r\n    def x(self): pass\r\n'
    )
    raw = data.encode("utf-8")
    ir = build(raw)
    assert text(ir, raw, "m.py::<lambda>") == b"lambda x: x"
    assert text(ir, raw, "m.py::C.x") == b"@property\r\n    def x(self): pass"
    lam = next(n for n in ir["nodes"] if n["kind"] == "Closure")
    # Columns are file bytes: 3 (BOM) + 18 more before `lambda` on line 1.
    assert lam["span"]["start"] == {"byte": 21, "line": 1, "col": 22}


def test_non_utf8_coding_cookie_and_lone_cr() -> None:
    raw = '# -*- coding: latin-1 -*-\ns = "é"; t = lambda: 1\rdef f():\r    return 1\r'.encode(
        "latin-1"
    )
    ir = build(raw)
    assert text(ir, raw, "m.py::<lambda>") == b"lambda: 1"
    assert text(ir, raw, "m.py::f") == b"def f():\r    return 1"
    assert function_spans(ir) == [Span("f", 3, 4, 4)]


def test_ids_parents_and_scopes() -> None:
    source = (
        "@deco(lambda: 0)\n"
        "def f(a=lambda: 1):\n"
        "    g = lambda: 2\n"
        "    class K:\n"
        "        def m(self): pass\n"
        "def f(): pass\n"
    )
    nodes = [(n["id"], n["parent"], n["kind"]) for n in build(source)["nodes"]]
    assert nodes == [
        ("m.py", None, "Module"),
        ("m.py::f", "m.py", "Function"),
        ("m.py::<lambda>", "m.py::f", "Closure"),
        ("m.py::<lambda>#1", "m.py::f", "Closure"),
        ("m.py::f.<locals>.<lambda>", "m.py::f", "Closure"),
        ("m.py::f.<locals>.K", "m.py::f", "Class"),
        ("m.py::f.<locals>.K.m", "m.py::f.<locals>.K", "Function"),
        ("m.py::f#1", "m.py", "Function"),
    ]


@pytest.mark.parametrize("source", [b"def f(:\n    pass\n", b"x = 1\0\n", b"\xff\xfe = 1\n"])
def test_unparsable_source_raises_syntax_error(source: bytes) -> None:
    with pytest.raises(SyntaxError):
        ir_frontend.build(source, "m.py")


def test_script_exit_codes(tmp_path: Path) -> None:
    good, bad = tmp_path / "good.py", tmp_path / "bad.py"
    good.write_text("def f():\n    return 1\n", encoding="utf-8")
    bad.write_text("def f(:\n", encoding="utf-8")

    def run(*args: object) -> subprocess.CompletedProcess[str]:
        cmd = [sys.executable, str(FRONTEND), *map(str, args)]
        return subprocess.run(cmd, capture_output=True, text=True, check=False)

    ok = run(good, "pkg/good.py")
    assert ok.returncode == 0
    assert json.loads(ok.stdout)["nodes"][1]["id"] == "pkg/good.py::f"
    failed = run(bad)
    assert (failed.returncode, failed.stdout) == (1, "")
    assert "bad.py" in failed.stderr
    assert run().returncode == 2


@pytest.mark.parametrize("name", ["tricky", "latin1_cr"])
def test_rust_fixtures_are_current(name: str) -> None:
    # crates/sundgren-ir validates these files against their sources in `cargo test`.
    data = ROOT / "crates/sundgren-ir/tests/data"
    got = build((data / f"{name}.py.src").read_bytes())
    want = json.loads((data / f"{name}.ir.json").read_text(encoding="utf-8"))
    got["path"], got["producer"] = want["path"], want["producer"]
    for n in got["nodes"]:
        n["id"] = n["id"].replace("m.py", want["path"], 1)
        if n["parent"]:
            n["parent"] = n["parent"].replace("m.py", want["path"], 1)
    assert got == want


def test_script_parses_as_python_3_8() -> None:
    # It runs under the target project's interpreter, which may be old.
    ast.parse(FRONTEND.read_text(encoding="utf-8"), feature_version=(3, 8))


def test_batch_mode(tmp_path: Path) -> None:
    good, bad = tmp_path / "good.py", tmp_path / "bad.py"
    good.write_text("def f():\n    return 1\n", encoding="utf-8")
    bad.write_text("def f(:\n", encoding="utf-8")
    requests = [
        {"file": str(good), "path": "pkg/good.py"},
        {"file": str(bad), "path": "pkg/bad.py"},
        {"file": str(tmp_path / "missing.py"), "path": "pkg/missing.py"},
        {"file": str(good), "path": "pkg/again.py"},
    ]
    stdin = "".join(json.dumps(r) + "\n" for r in requests)
    cmd = [sys.executable, "-I", str(FRONTEND), "--batch"]
    out = subprocess.run(cmd, input=stdin, capture_output=True, text=True, check=True).stdout
    ready, *lines = out.splitlines()
    assert json.loads(ready) == {"ready": ir_frontend.build(b"", "x")["producer"]}
    single = subprocess.run(
        [sys.executable, str(FRONTEND), str(good), "pkg/good.py"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    assert lines[0] + "\n" == single  # byte-identical to single-file mode
    results = [json.loads(line) for line in lines]
    assert [r["path"] for r in results] == [r["path"] for r in requests]
    assert [r.get("kind") for r in results] == [None, "syntax", "unreadable", None]
    assert results[3]["nodes"][1]["id"] == "pkg/again.py::f"


def test_rust_runner_spans_agree_with_symbolizer() -> None:
    # The M2 swap path: Rust runner -> IR -> callable_spans, in place of PythonSymbolizer.
    paths = corpus()
    files = [(p, p.relative_to(ROOT).as_posix()) for p in paths]
    got = frontend_python.function_spans(sys.executable, files)
    want = [PythonSymbolizer().functions(p.read_text(encoding="utf-8")) for p in paths]
    assert [p for p, g, w in zip(paths, got, want, strict=True) if g != w] == []
    calc = paths.index(ROOT / "tests/fixtures/shed_py/shed/calc.py")
    assert got[calc] == [
        Span("add", 2, 3, 3),
        Span("divide", 5, 8, 6),
        Span("classify", 10, 15, 11),
    ]


# L1


def l1(source: str) -> dict[str, Any]:
    ir = ir_frontend.build(source.encode("utf-8"), "m.py", l1=True)
    Draft202012Validator(SCHEMA).validate(ir)
    return ir


def locals_by_label(ir: dict[str, Any]) -> list[tuple[str, str | None]]:
    """(label, "scope:name" or None) for every node with a local, plus Name/arg nodes."""
    out = []
    for n in ir["nodes"]:
        loc = n["l1"]["local"]
        if loc or n["lang"]["ast"] in ("Name", "arg"):
            where = loc and f"{loc['scope'].removeprefix('m.py::')}:{loc['name']}"
            out.append((n["l1"]["label"], where))
    return out


def test_l1_named_nodes_are_the_l0_nodes() -> None:
    for path in corpus():
        data = path.read_bytes()
        got = [n for n in ir_frontend.build(data, "m.py", l1=True)["nodes"] if n["qualified_name"]]
        want = ir_frontend.build(data, "m.py")["nodes"][1:]
        keys = ("id", "kind", "span", "body_span", "name", "qualified_name")
        assert [[n[k] for k in keys] for n in got] == [[n[k] for k in keys] for n in want], path


def test_l1_structure_and_labels() -> None:
    ir = l1('x = -a + b.c < 2 <= f(1.0, k="s", **kw)\n')
    assert levels_and_labels(ir) == [
        ("m.py", "Module"),
        ("m.py/0", "Assign"),
        ("m.py/0/0", "Name:x"),
        ("m.py/0/1", "Compare:Lt,LtE"),
        ("m.py/0/1/0", "BinOp:Add"),
        ("m.py/0/1/0/0", "UnaryOp:USub"),
        ("m.py/0/1/0/0/0", "Name:a"),
        ("m.py/0/1/0/1", "Attribute:c"),
        ("m.py/0/1/0/1/0", "Name:b"),
        ("m.py/0/1/1", "Constant:int:2"),
        ("m.py/0/1/2", "Call"),
        ("m.py/0/1/2/0", "Name:f"),
        ("m.py/0/1/2/1", "Constant:float:1.0"),
        ("m.py/0/1/2/2", "keyword:k"),
        ("m.py/0/1/2/2/0", "Constant:str:'s'"),
        ("m.py/0/1/2/3", "keyword:**"),
        ("m.py/0/1/2/3/0", "Name:kw"),
    ]
    assert ir["levels"] == ["L1"]
    assert [n["kind"] for n in ir["nodes"][:3]] == ["Module", "Assign", {"Other": "Name"}]


def levels_and_labels(ir: dict[str, Any]) -> list[tuple[str, str]]:
    return [(n["id"], n["l1"]["label"]) for n in ir["nodes"]]


def test_l1_locals_follow_python_scoping() -> None:
    source = (
        "g = 0\n"
        "def f(a, b=g):\n"
        "    global h\n"
        "    h = [i for i in a if (w := i)]\n"
        "    def inner():\n"
        "        nonlocal a\n"
        "        return a, w, g\n"
        "    class K:\n"
        "        z = a\n"
        "    try:\n"
        "        pass\n"
        "    except E as err:\n"
        "        import os.path as p\n"
        "        return err\n"
        "lam = lambda q: [q for _ in q]\n"
    )
    assert locals_by_label(l1(source)) == [
        ("Name:g", None),
        ("arg", "f:a"),
        ("arg", "f:b"),
        ("Name:g", None),  # default: module scope
        ("Name:h", None),  # declared global
        ("Name", "f:i"),  # comprehension variables belong to the enclosing def
        ("Name", "f:i"),
        ("Name", "f:a"),
        ("Name", "f:w"),  # walrus binds in the def
        ("Name", "f:i"),
        ("FunctionDef", "f:inner"),
        ("Name", "f:a"),  # nonlocal
        ("Name", "f:w"),
        ("Name:g", None),
        ("ClassDef", "f:K"),
        ("Name:z", None),  # class-level name
        ("Name", "f:a"),
        ("ExceptHandler:as", "f:err"),
        ("Name:E", None),
        ("Name", "f:err"),
        ("Name:lam", None),
        ("arg", "<lambda>:q"),
        ("Name", "<lambda>:q"),
        ("Name", "<lambda>:_"),
        ("Name", "<lambda>:q"),
    ]


@pytest.mark.skipif(sys.version_info < (3, 12), reason="f-string pieces have positions on 3.12+")
def test_l1_multiline_fstring_piece_drops_its_newline() -> None:
    source = 'def f(x):\n    return f"""a\n{x}"""\n'
    ir = l1(source)
    piece = next(n for n in ir["nodes"] if n["l1"]["label"] == "Constant:str:'a\\n'")
    assert text(ir, source.encode(), piece["id"]) == b"a"


def test_l1_batch_request() -> None:
    cmd = [sys.executable, "-I", str(FRONTEND), "--batch"]
    req = {"file": str(ROOT / "tests/fixtures/shed_py/shed/calc.py"), "path": "c.py", "l1": True}
    out = subprocess.run(
        cmd, input=json.dumps(req) + "\n", capture_output=True, text=True, check=True
    )
    ir = json.loads(out.stdout.splitlines()[1])
    assert ir["levels"] == ["L1"] and all("l1" in n for n in ir["nodes"])
