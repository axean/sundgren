"""core.report.json_report: sundgren.delta/1 shape, filtering, schema, determinism."""

from __future__ import annotations

import json
from collections.abc import Mapping
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator, ValidationError

from sundgren.core.diff import Delta, diff
from sundgren.core.model import (
    BranchKey,
    FileCoverage,
    FunctionRecord,
    Outcome,
    ReportWarning,
    Snapshot,
)
from sundgren.core.report.json_report import build_report, dumps, load_schema

SCHEMA_FILE = Path(__file__).resolve().parents[2] / "src/sundgren/core/report/delta.schema.json"
VALIDATOR = Draft202012Validator(load_schema())
SHA = "ab" * 32

# The spec's canonical example: skip test_divide, divide goes dark.
#  1 def add(a, b):          5     if b == 0:                10     if n < 0:
#  2     return a + b        6         raise ZeroDivision... 11         return "neg"
#  3                         7     return a / b              12     elif n == 0:
#  4 def divide(a, b):       8                               13         return "zero"
#                            9 def classify(n):              14     return "pos"
CALC = "shed/calc.py"
EXECUTABLE = (1, 2, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14)
FUNCTIONS = (
    FunctionRecord("add", 1, 2, 1),
    FunctionRecord("divide", 4, 7, 1),
    FunctionRecord("classify", 9, 14, 3),
)
ARCS = ((5, 6), (5, 7), (10, 11), (10, 12), (12, 13), (12, 14))


def calc(dark: bool, hits: int = 1) -> FileCoverage:
    lines = {ln: 0 if dark and ln in (5, 6, 7) else hits for ln in EXECUTABLE}
    branches: dict[BranchKey, int | None] = {
        BranchKey(src, "0", f"jump to line {dst}"): None if dark and src == 5 else 1
        for src, dst in ARCS
    }
    return FileCoverage(CALC, lines, branches, FUNCTIONS, False)


def snap(*files: FileCoverage) -> Snapshot:
    return Snapshot({f.path: f for f in files}, ())


INIT = FileCoverage("shed/__init__.py", {}, {}, (), False)
BASE_OUTCOMES: dict[str, Outcome] = {
    "tests.test_calc::test_add": "passed",
    "tests.test_calc::test_divide": "passed",
}
PATCHED_OUTCOMES: dict[str, Outcome] = {
    "tests.test_calc::test_add": "passed",
    "tests.test_calc::test_divide": "skipped",
}


def canonical(**kw: Any) -> Delta:
    return diff(
        [snap(INIT, calc(dark=False))],
        [snap(INIT, calc(dark=True))],
        {},
        base_outcomes=BASE_OUTCOMES,
        patched_outcomes=PATCHED_OUTCOMES,
        **kw,
    )


def diff_meta(**over: Any) -> dict[str, Any]:
    meta: dict[str, Any] = {
        "sundgren_version": "0.1.0",
        "created_utc": "2026-10-18T07:00:00Z",
        "mode": "diff",
        "base_ref": None,
        "patch": None,
        "preset": None,
        "commands": None,
        "selection": None,
        "repeat": None,
        "inputs": {"base_lcov_sha256": [SHA], "patched_lcov_sha256": [SHA]},
    }
    return {**meta, **over}


def run_meta(**over: Any) -> dict[str, Any]:
    run: dict[str, Any] = {
        "mode": "run",
        "base_ref": "a1b2c3d",
        "patch": {"path": "skip_divide.patch", "sha256": SHA, "files": ["tests/test_calc.py"]},
        "preset": "pytest",
        "commands": ["python -m coverage run --branch -m pytest tests/test_calc.py"],
        "selection": {"tests": ["tests/test_calc.py"], "run_all": False, "reason": "touched"},
        "repeat": 1,
    }
    return diff_meta(**{**run, **over})


def report(delta: Delta, meta: Mapping[str, Any] | None = None, **kw: Any) -> dict[str, Any]:
    out = build_report(delta, meta if meta is not None else diff_meta(), **kw)
    VALIDATOR.validate(out)
    return out


# --- schema ------------------------------------------------------------------


def test_schema_is_valid_draft_2020_12() -> None:
    Draft202012Validator.check_schema(load_schema())
    assert load_schema() == json.loads(SCHEMA_FILE.read_text())


@pytest.mark.parametrize("meta", [diff_meta(), diff_meta(patch=run_meta()["patch"]), run_meta()])
def test_meta_variants_validate(meta: dict[str, Any]) -> None:
    report(canonical(), meta)


@pytest.mark.parametrize(
    "meta",
    [
        diff_meta(preset="pytest"),  # run-only field in diff mode
        diff_meta(repeat=1),
        run_meta(preset=None),
        run_meta(patch=None),
        diff_meta(mode="other"),
        diff_meta(created_utc="yesterday"),
        diff_meta(extra=1),
        {k: v for k, v in diff_meta().items() if k != "inputs"},
    ],
)
def test_bad_meta_is_rejected(meta: dict[str, Any]) -> None:
    with pytest.raises(ValidationError):
        VALIDATOR.validate(build_report(canonical(), meta))


def test_schema_rejects_unknown_categories_and_keys() -> None:
    good = report(canonical())
    bad = json.loads(json.dumps(good))
    bad["files"][0]["lines"][0]["category"] = "lost_somehow"
    with pytest.raises(ValidationError):
        VALIDATOR.validate(bad)
    bad = json.loads(json.dumps(good))
    bad["summary"]["lines"]["extra"] = 0
    with pytest.raises(ValidationError):
        VALIDATOR.validate(bad)
    bad = json.loads(json.dumps(good))
    bad["calls"] = []
    with pytest.raises(ValidationError):
        VALIDATOR.validate(bad)


# --- canonical example -------------------------------------------------------


def test_canonical_example() -> None:
    r = report(canonical(), run_meta())
    assert r["schema"] == "sundgren.delta/1" and r["calls"] is None
    assert r["meta"] == run_meta()
    assert r["tests"] == [
        {"id": "tests.test_calc::test_divide", "base": "passed", "patched": "skipped"}
    ]
    assert r["summary"] == {
        "lines": {
            "lost": 3,
            "gained": 0,
            "new_hit": 0,
            "new_miss": 0,
            "deleted_hit": 0,
            "deleted_miss": 0,
            "flaky": 0,
            "unmeasured": 0,
        },
        "branches": {"lost": 2, "gained": 0, "new": 0, "deleted": 0},
        "functions": {
            "went_dark": 1,
            "lit_up": 0,
            "partial_loss": 0,
            "partial_gain": 0,
            "mixed": 0,
            "added": 0,
            "removed": 0,
        },
        "coverage": {"base_pct": 100.0, "patched_pct": 75.0},
        "external_files": 0,
    }
    assert r["files"] == [
        {
            "path": CALC,
            "base_path": CALC,
            "sha256": {"base": None, "patched": None},
            "source_changed": False,
            "inferred_side": None,
            "lines": [
                {"line": ln, "base_line": ln, "category": "lost", "hits": [1, 0]}
                for ln in (5, 6, 7)
            ],
            "branches": [
                {
                    "line": 5,
                    "base_line": 5,
                    "block": "0",
                    "branch": f"jump to line {dst}",
                    "category": "branch_lost",
                    "taken": [1, None],
                }
                for dst in (6, 7)
            ],
            "functions": [
                {
                    "name": "divide",
                    "start": 4,
                    "end": 7,
                    "status": "went_dark",
                    "lines": {"lost": 3, "gained": 0, "executable": 3},
                }
            ],
            "flaky": [],
            "count_changed": [],
        }
    ]
    assert r["warnings"] == []


def test_tree_shape() -> None:
    tree = report(canonical())["tree"]
    assert {k: tree[k] for k in ("name", "path", "kind")} == {"name": "", "path": "", "kind": "dir"}
    assert tree["stats"]["lost"] == 3
    assert (tree["stats"]["executable_base"], tree["stats"]["hit_patched"]) == (12, 9)
    [shed] = tree["children"]
    assert (shed["path"], shed["kind"]) == ("shed", "dir")
    assert [(c["name"], c["path"], c["kind"]) for c in shed["children"]] == [
        ("__init__.py", "shed/__init__.py", "file"),
        ("calc.py", "shed/calc.py", "file"),
    ]
    assert all("children" not in c for c in shed["children"])
    assert set(shed["children"][1]["stats"]) == {
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
    }


# --- filtering ---------------------------------------------------------------


def test_full_keeps_everything() -> None:
    r = report(canonical(), full=True)
    assert [f["path"] for f in r["files"]] == ["shed/__init__.py", CALC]
    calc_entry = r["files"][1]
    assert len(calc_entry["lines"]) == len(EXECUTABLE)
    assert len(calc_entry["branches"]) == len(ARCS)
    assert [f["status"] for f in calc_entry["functions"]] == ["unchanged", "went_dark", "unchanged"]
    assert [t["id"] for t in r["tests"]] == [
        "tests.test_calc::test_add",
        "tests.test_calc::test_divide",
    ]
    # Filtering never changes the summary or the tree.
    default = report(canonical())
    assert (r["summary"], r["tree"]) == (default["summary"], default["tree"])


def test_unchanged_run_lists_nothing() -> None:
    d = diff([snap(INIT, calc(False))], [snap(INIT, calc(False))], {})
    r = report(d)
    assert r["files"] == [] and r["tests"] == []
    assert r["summary"]["coverage"]["base_pct"] == r["summary"]["coverage"]["patched_pct"]


def test_counts_fills_count_changed() -> None:
    d = diff([snap(calc(False, hits=1))], [snap(calc(False, hits=2))], {})
    assert report(d)["files"] == []
    [f] = report(d, counts=True)["files"]
    assert f["count_changed"] == [
        {"line": ln, "base_line": ln, "hits": [1, 2]} for ln in EXECUTABLE
    ]
    assert f["lines"] == []
    [f] = report(d, full=True)["files"]
    assert f["count_changed"] == []


def test_coverage_pct_rounding_and_null() -> None:
    a = FileCoverage("a.py", {1: 1, 2: 1, 3: 0}, {}, (), False)
    d = diff([snap(a)], [snap(INIT)], {})  # patched side: nothing executable
    cov = report(d)["summary"]["coverage"]
    assert cov == {"base_pct": 66.7, "patched_pct": None}


def test_warnings_merged_sorted_deduplicated() -> None:
    d = diff([snap()], [snap(FileCoverage("x.py", {1: 1}, {}, (), False))], {})
    extra = [
        ReportWarning("patch", "z"),
        ReportWarning("dirty_tree", "uncommitted"),
        ReportWarning("patch", "z"),
    ]
    r = report(d, extra_warnings=extra)
    assert [w["code"] for w in r["warnings"]] == ["dirty_tree", "patch", "unmeasured"]


# --- serialization -----------------------------------------------------------


def test_dumps_format() -> None:
    r = report(canonical(), run_meta())
    text = dumps(r)
    assert text.endswith("}\n") and not text.endswith("\n\n")
    assert text == json.dumps(r, sort_keys=True, indent=2) + "\n"
    assert json.loads(text) == r


def _repo(root: Path) -> Path:
    for side, text in (("base", "x = 1\n"), ("patched", "x = 2\n")):
        for rel in (CALC, "shed/only_here.py"):
            path = root / side / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
    return root


def test_identical_inputs_in_different_dirs_dump_identically(tmp_path: Path) -> None:
    only = FileCoverage("shed/only_here.py", {1: 1}, {}, (), False)
    outs = []
    for name in ("one", "two"):
        root = _repo(tmp_path / name)
        d = diff(
            [snap(calc(False), only)],
            [snap(calc(True))],  # only_here.py inferred on the patched side
            {},
            base_root=root / "base",
            patched_root=root / "patched",
            base_outcomes=BASE_OUTCOMES,
            patched_outcomes=PATCHED_OUTCOMES,
        )
        outs.append(dumps(report(d, run_meta(), full=True)))
    assert outs[0] == outs[1]
    assert str(tmp_path) not in outs[0]
    assert '"inferred_side": "patched"' in outs[0]
