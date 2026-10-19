"""Stubs for sundgren._native.core_diff."""

from collections.abc import Mapping, Sequence
from dataclasses import Field
from pathlib import Path
from typing import Any, ClassVar, Literal

from sundgren.core.linemap import LineMap
from sundgren.core.model import Outcome, ReportWarning, Snapshot
from sundgren.core.rollup import FunctionDelta, Stats, TreeNode

_Side = Literal["base", "patched"]
_LineCategory = Literal[
    "kept_hit",
    "kept_miss",
    "lost",
    "gained",
    "unmeasured",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
]
_BranchCategory = Literal[
    "branch_kept_hit",
    "branch_kept_miss",
    "branch_lost",
    "branch_gained",
    "branch_new",
    "branch_deleted",
    "branch_flaky",
    "branch_unmeasured",
]

class LineDelta:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("line", "base_line", "category", "hits")
    def __init__(
        self,
        line: int | None,
        base_line: int | None,
        category: _LineCategory,
        hits: tuple[int | None, int | None],
    ) -> None: ...
    @property
    def line(self) -> int | None: ...
    @property
    def base_line(self) -> int | None: ...
    @property
    def category(self) -> _LineCategory: ...
    @property
    def hits(self) -> tuple[int | None, int | None]: ...
    def __hash__(self) -> int: ...

class BranchDelta:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("line", "base_line", "block", "branch", "category", "taken")
    def __init__(
        self,
        line: int | None,
        base_line: int | None,
        block: str,
        branch: str,
        category: _BranchCategory,
        taken: tuple[int | None, int | None],
    ) -> None: ...
    @property
    def line(self) -> int | None: ...
    @property
    def base_line(self) -> int | None: ...
    @property
    def block(self) -> str: ...
    @property
    def branch(self) -> str: ...
    @property
    def category(self) -> _BranchCategory: ...
    @property
    def taken(self) -> tuple[int | None, int | None]: ...
    def __hash__(self) -> int: ...

class FlakyLine:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("line", "base_line", "hits", "sides")
    def __init__(
        self,
        line: int | None,
        base_line: int | None,
        hits: tuple[int | None, int | None],
        sides: tuple[_Side, ...],
    ) -> None: ...
    @property
    def line(self) -> int | None: ...
    @property
    def base_line(self) -> int | None: ...
    @property
    def hits(self) -> tuple[int | None, int | None]: ...
    @property
    def sides(self) -> tuple[_Side, ...]: ...
    def __hash__(self) -> int: ...

class FileDelta:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = (
        "path",
        "base_path",
        "sha256",
        "source_changed",
        "inferred_side",
        "lines",
        "branches",
        "functions",
        "flaky",
        "stats",
    )
    def __init__(
        self,
        path: str,
        base_path: str | None,
        sha256: tuple[str | None, str | None],
        source_changed: bool,
        inferred_side: _Side | None,
        lines: tuple[LineDelta, ...],
        branches: tuple[BranchDelta, ...],
        functions: tuple[FunctionDelta, ...],
        flaky: tuple[FlakyLine, ...],
        stats: Stats,
    ) -> None: ...
    @property
    def path(self) -> str: ...
    @property
    def base_path(self) -> str | None: ...
    @property
    def sha256(self) -> tuple[str | None, str | None]: ...
    @property
    def source_changed(self) -> bool: ...
    @property
    def inferred_side(self) -> _Side | None: ...
    @property
    def lines(self) -> tuple[LineDelta, ...]: ...
    @property
    def branches(self) -> tuple[BranchDelta, ...]: ...
    @property
    def functions(self) -> tuple[FunctionDelta, ...]: ...
    @property
    def flaky(self) -> tuple[FlakyLine, ...]: ...
    @property
    def stats(self) -> Stats: ...
    @property
    def changed(self) -> bool: ...
    def __hash__(self) -> int: ...

class OutcomePair:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("id", "base", "patched")
    def __init__(self, id: str, base: Outcome, patched: Outcome) -> None: ...
    @property
    def id(self) -> str: ...
    @property
    def base(self) -> Outcome: ...
    @property
    def patched(self) -> Outcome: ...
    def __hash__(self) -> int: ...

class Delta:
    __dataclass_fields__: ClassVar[dict[str, Field[Any]]]
    __match_args__ = ("files", "tests", "tree", "external_files", "warnings")
    def __init__(
        self,
        files: tuple[FileDelta, ...],
        tests: tuple[OutcomePair, ...],
        tree: TreeNode,
        external_files: int,
        warnings: tuple[ReportWarning, ...],
    ) -> None: ...
    @property
    def files(self) -> tuple[FileDelta, ...]: ...
    @property
    def tests(self) -> tuple[OutcomePair, ...]: ...
    @property
    def tree(self) -> TreeNode: ...
    @property
    def external_files(self) -> int: ...
    @property
    def warnings(self) -> tuple[ReportWarning, ...]: ...
    def __hash__(self) -> int: ...

def diff(
    base: Sequence[Snapshot],
    patched: Sequence[Snapshot],
    linemaps: Mapping[str, LineMap],
    *,
    base_root: Path | None = None,
    patched_root: Path | None = None,
    base_outcomes: Mapping[str, Outcome] | None = None,
    patched_outcomes: Mapping[str, Outcome] | None = None,
) -> Delta: ...
