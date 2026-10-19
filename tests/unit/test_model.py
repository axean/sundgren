"""The Rust model classes behave like the prototype's frozen dataclasses."""

from __future__ import annotations

import copy
import dataclasses
import pickle

import pytest

from sundgren.core.model import (
    BranchKey,
    FileCoverage,
    FunctionRecord,
    LineStatus,
    Snapshot,
    merge_file_coverage,
    snapshot_to_dict,
)

KEY = BranchKey(1, "0", "1")
FN = FunctionRecord(name="f", start=1, end=None, hits=None)
FC = FileCoverage(
    path="a.py", lines={1: 0, 3: 1}, branches={KEY: None}, functions=(FN,), external=False
)


def test_dataclass_helpers() -> None:
    assert dataclasses.is_dataclass(FC)
    assert [f.name for f in dataclasses.fields(FC)] == [
        "path",
        "lines",
        "branches",
        "functions",
        "external",
    ]
    assert dataclasses.asdict(FN) == {"name": "f", "start": 1, "end": None, "hits": None}
    assert dataclasses.astuple(KEY) == (1, "0", "1")
    moved = dataclasses.replace(FC, path="b.py")
    assert moved.path == "b.py"
    assert moved.lines == FC.lines


def test_repr_eq_hash_order() -> None:
    assert repr(FN) == "FunctionRecord(name='f', start=1, end=None, hits=None)"
    assert repr(FC) == (
        "FileCoverage(path='a.py', lines={1: 0, 3: 1}, "
        "branches={BranchKey(line=1, block='0', branch='1'): None}, "
        "functions=(FunctionRecord(name='f', start=1, end=None, hits=None),), external=False)"
    )
    assert BranchKey(line=1, block="0", branch="1") == KEY
    assert KEY != (1, "0", "1")
    assert hash(KEY) == hash(BranchKey(1, "0", "1"))
    assert sorted([BranchKey(1, "1", "0"), KEY, BranchKey(0, "9", "9")])[1] == KEY
    with pytest.raises(TypeError):
        hash(FC)
    with pytest.raises(AttributeError):
        FC.path = "x"  # type: ignore[misc]


def test_copy_pickle() -> None:
    assert copy.deepcopy(FC) == FC
    assert pickle.loads(pickle.dumps(KEY)) == KEY


def test_status_merge_and_dict() -> None:
    assert FC.status(1) == LineStatus.MISS
    assert FC.status(3) == LineStatus.HIT
    assert FC.status(2).value == "absent"
    big = 2**64 - 1
    fc = FileCoverage(path="a.py", lines={1: big}, branches={KEY: 2}, functions=(), external=True)
    merged = merge_file_coverage(FC, fc)
    assert merged.lines == {1: big, 3: 1}
    assert merged.branches == {KEY: 2}
    assert not merged.external
    out = snapshot_to_dict(Snapshot(files={"a.py": fc}, test_names=("t",)))
    assert out == {
        "test_names": ["t"],
        "files": [
            {
                "path": "a.py",
                "external": True,
                "lines": {"1": big},
                "branches": [{"line": 1, "block": "0", "branch": "1", "taken": 2}],
                "functions": [],
            }
        ],
    }
