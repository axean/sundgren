"""Stubs for sundgren._native.core_rollup."""

from collections.abc import Iterable, Mapping, Sequence
from dataclasses import Field
from typing import Any, ClassVar, Literal

from sundgren.core.linemap import LineMap
from sundgren.core.symbols import Span

_Status = Literal[
    "went_dark",
    "lit_up",
    "partial_loss",
    "partial_gain",
    "mixed",
    "unchanged",
    "added",
    "removed",
]

class FunctionDelta:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("name", "start", "end", "status", "lost", "gained", "executable")
    def __init__(
        self,
        name: str,
        start: int,
        end: int,
        status: _Status,
        lost: int,
        gained: int,
        executable: int,
    ) -> None: ...
    @property
    def name(self) -> str: ...
    @property
    def start(self) -> int: ...
    @property
    def end(self) -> int: ...
    @property
    def status(self) -> _Status: ...
    @property
    def lost(self) -> int: ...
    @property
    def gained(self) -> int: ...
    @property
    def executable(self) -> int: ...
    def __hash__(self) -> int: ...

class Stats:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = (
        "lost",
        "gained",
        "new_hit",
        "new_miss",
        "deleted_hit",
        "deleted_miss",
        "executable_base",
        "executable_patched",
        "hit_base",
        "hit_patched",
    )
    def __init__(
        self,
        lost: int = 0,
        gained: int = 0,
        new_hit: int = 0,
        new_miss: int = 0,
        deleted_hit: int = 0,
        deleted_miss: int = 0,
        executable_base: int = 0,
        executable_patched: int = 0,
        hit_base: int = 0,
        hit_patched: int = 0,
    ) -> None: ...
    @property
    def lost(self) -> int: ...
    @property
    def gained(self) -> int: ...
    @property
    def new_hit(self) -> int: ...
    @property
    def new_miss(self) -> int: ...
    @property
    def deleted_hit(self) -> int: ...
    @property
    def deleted_miss(self) -> int: ...
    @property
    def executable_base(self) -> int: ...
    @property
    def executable_patched(self) -> int: ...
    @property
    def hit_base(self) -> int: ...
    @property
    def hit_patched(self) -> int: ...
    def __add__(self, other: Stats) -> Stats: ...
    def __hash__(self) -> int: ...

class TreeNode:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("name", "path", "kind", "stats", "children")
    def __init__(
        self,
        name: str,
        path: str,
        kind: Literal["dir", "file"],
        stats: Stats,
        children: tuple[TreeNode, ...] = (),
    ) -> None: ...
    @property
    def name(self) -> str: ...
    @property
    def path(self) -> str: ...
    @property
    def kind(self) -> Literal["dir", "file"]: ...
    @property
    def stats(self) -> Stats: ...
    @property
    def children(self) -> tuple[TreeNode, ...]: ...
    def __hash__(self) -> int: ...

def function_deltas(
    base_spans: Sequence[Span] | None,
    patched_spans: Sequence[Span] | None,
    lm: LineMap,
    base_lines: Mapping[int, int],
    patched_lines: Mapping[int, int],
    *,
    lost: frozenset[int] = ...,
    gained: frozenset[int] = ...,
    flaky_base: frozenset[int] = ...,
    flaky_patched: frozenset[int] = ...,
) -> tuple[FunctionDelta, ...]: ...
def build_tree(files: Iterable[tuple[str, Stats]]) -> TreeNode: ...
