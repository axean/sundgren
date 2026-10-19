"""The Python IR frontend: schema, exact spans, and agreement with PythonSymbolizer."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator

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
