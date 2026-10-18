"""Coverage data model shared by every layer."""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from enum import Enum
from typing import Any, Literal

LineNo = int  # 1-based

Outcome = Literal["passed", "failed", "skipped", "xfailed", "xpassed", "error", "not_run"]


class LineStatus(Enum):
    ABSENT = "absent"
    MISS = "miss"
    HIT = "hit"


@dataclass(frozen=True, slots=True, order=True)
class BranchKey:
    line: LineNo
    block: str  # lcov block id, kept as a string (may carry an 'e' prefix)
    branch: str


@dataclass(frozen=True, slots=True)
class FunctionRecord:
    name: str
    start: LineNo
    end: LineNo | None  # None when the lcov version has no end line
    hits: int | None  # from FNDA


@dataclass(frozen=True, slots=True)
class FileCoverage:
    path: str  # normalized, repo-relative, posix
    lines: Mapping[LineNo, int]  # executable line -> hit count
    branches: Mapping[BranchKey, int | None]  # None = '-' (block never reached)
    functions: tuple[FunctionRecord, ...]
    external: bool  # path couldn't be made repo-relative

    def status(self, line: LineNo) -> LineStatus:
        count = self.lines.get(line)
        if count is None:
            return LineStatus.ABSENT
        return LineStatus.HIT if count > 0 else LineStatus.MISS


@dataclass(frozen=True, slots=True)
class Snapshot:
    files: Mapping[str, FileCoverage]
    test_names: tuple[str, ...]  # from lcov TN:, informational


@dataclass(frozen=True, slots=True)
class ReportWarning:
    code: str  # stable machine id, e.g. "unmeasured", "inferred_file", "dirty_tree"
    message: str


def _add_taken(a: int | None, b: int | None) -> int | None:
    if a is None:
        return b
    if b is None:
        return a
    return a + b


def merge_file_coverage(a: FileCoverage, b: FileCoverage) -> FileCoverage:
    """Merge two records for the same file: sum DA and BRDA, merge FNDA by name."""
    lines = dict(a.lines)
    for ln, count in b.lines.items():
        lines[ln] = lines.get(ln, 0) + count
    branches = dict(a.branches)
    for key, taken in b.branches.items():
        branches[key] = _add_taken(branches[key], taken) if key in branches else taken
    funcs: dict[str, FunctionRecord] = {f.name: f for f in a.functions}
    for f in b.functions:
        old = funcs.get(f.name)
        if old is None:
            funcs[f.name] = f
        else:
            funcs[f.name] = FunctionRecord(
                name=old.name,
                start=old.start,
                end=old.end if old.end is not None else f.end,
                hits=_add_taken(old.hits, f.hits),
            )
    return FileCoverage(
        path=a.path,
        lines=lines,
        branches=branches,
        functions=tuple(sorted(funcs.values(), key=lambda f: (f.start, f.name))),
        external=a.external and b.external,
    )


def snapshot_to_dict(snapshot: Snapshot) -> dict[str, Any]:
    """Plain-JSON view of a Snapshot (for `sundgren ingest`)."""
    files = []
    for path in sorted(snapshot.files):
        fc = snapshot.files[path]
        files.append(
            {
                "path": fc.path,
                "external": fc.external,
                "lines": {str(ln): fc.lines[ln] for ln in sorted(fc.lines)},
                "branches": [
                    {"line": k.line, "block": k.block, "branch": k.branch, "taken": fc.branches[k]}
                    for k in sorted(fc.branches)
                ],
                "functions": [
                    {"name": f.name, "start": f.start, "end": f.end, "hits": f.hits}
                    for f in fc.functions
                ],
            }
        )
    return {"test_names": list(snapshot.test_names), "files": files}
