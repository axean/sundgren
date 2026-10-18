from __future__ import annotations

import ast
import textwrap
import warnings
from typing import Literal

import pytest

from sundgren.core.symbols import Span
from sundgren.lang.python.symbolizer import PythonSymbolizer


def spans(source: str) -> list[Span]:
    return PythonSymbolizer().functions(textwrap.dedent(source).lstrip("\n"))


def by_name(source: str) -> dict[str, Span]:
    return {s.name: s for s in spans(source)}


def test_module_functions_and_lambdas() -> None:
    src = """
        def add(a, b):
            return a + b

        square = lambda x: x * x

        async def fetch():
            await thing()
            return 1
        """
    assert spans(src) == [Span("add", 1, 2, 2), Span("fetch", 6, 8, 7)]


def test_qualified_names() -> None:
    src = """
        class Outer:
            def method(self):
                def helper():
                    return 1
                return helper()

            class Inner:
                async def deep(self):
                    return 2


        def outer():
            def inner():
                class Local:
                    def m(self):
                        return 3
                return Local
            return inner
        """
    assert [s.name for s in spans(src)] == [
        "Outer.method",
        "Outer.method.<locals>.helper",
        "Outer.Inner.deep",
        "outer",
        "outer.<locals>.inner",
        "outer.<locals>.inner.<locals>.Local.m",
    ]


def test_functions_inside_compound_statements_keep_scope() -> None:
    src = """
        try:
            import fast
        except ImportError:
            def fallback():
                return 0

        class C:
            if True:
                def m(self):
                    return 1

        def f(x):
            match x:
                case 1:
                    def g():
                        return 2
            with ctx():
                def h():
                    return 3
        """
    assert [s.name for s in spans(src)] == [
        "fallback",
        "C.m",
        "f",
        "f.<locals>.g",
        "f.<locals>.h",
    ]


def test_decorators_extend_start() -> None:
    src = """
        import functools


        @functools.cache
        @other(
            arg=1,
        )
        def cached(x):
            return x


        class C:
            @property
            def value(self):
                return 1
        """
    got = by_name(src)
    assert got["cached"] == Span("cached", 4, 9, 9)
    assert got["C.value"] == Span("C.value", 13, 15, 15)


def test_body_start_skips_docstring() -> None:
    src = '''
        def documented(a,
                       b):
            """Doc.

            More doc.
            """
            x = a
            return x + b


        def only_docstring():
            """Nothing else."""


        def multiline_signature(
            a,
        ):
            return a
        '''
    got = by_name(src)
    assert got["documented"] == Span("documented", 1, 8, 7)
    assert got["only_docstring"] == Span("only_docstring", 11, 12, 12)
    assert got["multiline_signature"] == Span("multiline_signature", 15, 18, 18)


def test_one_liners_have_no_body_start() -> None:
    src = """
        def f(): return 1
        async def g(): await h(); return 2
        def d(): "doc"; return 3
        @dec
        def k(): return (
            4
        )
        def tail(
            a): return a
        """
    got = by_name(src)
    assert got["f"] == Span("f", 1, 1, None)
    assert got["g"] == Span("g", 2, 2, None)
    assert got["d"] == Span("d", 3, 3, None)
    assert got["k"] == Span("k", 4, 7, None)
    assert got["tail"] == Span("tail", 8, 9, 9)


def test_body_lines_exclude_signature() -> None:
    src = '''
        @dec
        def f(a):
            """Doc."""
            if a:
                return 1
            return 2
        '''
    assert list(spans(src)[0].body_lines) == [4, 5, 6]


def test_body_start_counts_decorators_of_first_statement() -> None:
    src = '''
        def f():
            @dec
            def inner():
                return 1
            return inner


        def g():
            """Doc."""
            @dec
            class K:
                pass
            return K
        '''
    got = by_name(src)
    assert got["f"] == Span("f", 1, 5, 2)
    assert got["g"] == Span("g", 8, 13, 10)


def test_sorted_by_start_then_name() -> None:
    src = """
        def b():
            def z():
                pass
            def a():
                pass

        def a():
            pass
        """
    assert [(s.start, s.name) for s in spans(src)] == [
        (1, "b"),
        (2, "b.<locals>.z"),
        (4, "b.<locals>.a"),
        (7, "a"),
    ]


def test_redefinitions_keep_the_same_name() -> None:
    src = """
        class C:
            @property
            def x(self):
                return self._x

            @x.setter
            def x(self, v):
                self._x = v
        """
    assert [(s.name, s.start) for s in spans(src)] == [("C.x", 2), ("C.x", 6)]


def test_empty_and_function_free_source() -> None:
    assert spans("") == []
    assert spans("x = 1\nclass C:\n    y = lambda: 2\n") == []


def test_leading_bom_is_ignored() -> None:
    assert PythonSymbolizer().functions("\ufeffdef f():\n    return 1\n") == [Span("f", 1, 2, 2)]


@pytest.mark.parametrize("action", ["error", "always"])
def test_target_syntax_warnings_are_silenced(action: Literal["error", "always"]) -> None:
    # "error" is -W error; "always" would print the warning.
    source = 'def f():\n    return "\\d"\n'
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter(action)
        assert PythonSymbolizer().functions(source) == [Span("f", 1, 2, 2)]
    assert caught == []


def test_crlf_line_endings() -> None:
    assert PythonSymbolizer().functions("x = 1\r\ndef f():\r\n    return 1\r\n") == [
        Span("f", 2, 3, 3)
    ]


@pytest.mark.parametrize("source", ["def f(:\n    pass\n", "x = 1\0\n"])
def test_unparsable_source_raises_syntax_error(source: str) -> None:
    with pytest.raises(SyntaxError):
        PythonSymbolizer().functions(source)


def test_deep_expression_and_long_elif_chain() -> None:
    # Both nest the AST about 2000 levels deep; a recursive walk overflows the stack.
    deep = "x = " + "+".join(["1"] * 2000) + "\ndef f():\n    return 1\n"
    assert PythonSymbolizer().functions(deep) == [Span("f", 2, 3, 3)]
    branches = "".join(f"elif x == {i}:\n    def g{i}(): pass\n" for i in range(1, 2000))
    got = PythonSymbolizer().functions("if x == 0:\n    pass\n" + branches)
    assert len(got) == 1999
    assert got[-1] == Span("g1999", 4000, 4000, None)


@pytest.mark.parametrize(
    "exc", [RecursionError("deep"), MemoryError("too complex"), ValueError("nul")]
)
def test_parser_limits_raise_syntax_error(
    monkeypatch: pytest.MonkeyPatch, exc: BaseException
) -> None:
    # Which inputs hit these limits depends on the Python version, so fake the parser.
    def parse(source: str) -> ast.Module:
        raise exc

    monkeypatch.setattr(ast, "parse", parse)
    with pytest.raises(SyntaxError) as info:
        PythonSymbolizer().functions("def f():\n    pass\n")
    assert info.value.__cause__ is exc
