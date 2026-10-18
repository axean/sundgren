"""Spec §12.1: `sundgren run --preset pytest` on shed_py, scenarios 1-8, plus patch-format cases."""

from __future__ import annotations

import hashlib
import json
import re
import shlex
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest

from support.shed import (
    PATCHES,
    SHED_PY,
    commit_files,
    git,
    make_shed_py,
    run_json,
    run_sundgren,
)

pytestmark = pytest.mark.slow

CALC = "shed/calc.py"
TEST_FILE = "tests/test_calc.py"
LINE_KEYS = (
    "lost",
    "gained",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
    "flaky",
    "unmeasured",
)
BRANCH_KEYS = ("lost", "gained", "new", "deleted")
FUNCTION_KEYS = ("went_dark", "lit_up", "partial_loss", "partial_gain", "mixed", "added", "removed")
BRANCH_CHANGES = ("branch_lost", "branch_gained", "branch_new", "branch_deleted")

PYTEST_COMMANDS = (
    "{python} -m coverage run --branch -m pytest {tests} -p no:cacheprovider -q --junitxml={junit}",
    "{python} -m coverage lcov --data-file={out}/.coverage -o {lcov}",
)


def line_of(text: str, needle: str) -> int:
    """1-based number of the first line containing `needle`."""
    for no, line in enumerate(text.splitlines(), 1):
        if needle in line:
            return no
    raise AssertionError(f"{needle!r} not found")


CALC_SRC = (SHED_PY / CALC).read_text()
ADD = line_of(CALC_SRC, "def add")
DIVIDE = line_of(CALC_SRC, "def divide")
DIVIDE_BODY = {
    line_of(CALC_SRC, "if b == 0"),
    line_of(CALC_SRC, "raise ZeroDivisionError"),
    line_of(CALC_SRC, "return a / b"),
}
CLASSIFY = line_of(CALC_SRC, "def classify")
ELIF = line_of(CALC_SRC, "elif n == 0")
RETURN_ZERO = line_of(CALC_SRC, 'return "zero"')
CALC_END = len(CALC_SRC.splitlines())


def run_preset(repo: Path, patch: str, *extra: str) -> tuple[dict[str, Any], str]:
    return run_json(
        "run",
        "--repo",
        repo,
        "--patch",
        PATCHES / patch,
        "--preset",
        "pytest",
        "--python",
        sys.executable,
        *extra,
    )


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def counts(keys: tuple[str, ...], **nonzero: int) -> dict[str, int]:
    return {**dict.fromkeys(keys, 0), **nonzero}


def only_file(report: dict[str, Any], path: str) -> dict[str, Any]:
    assert [f["path"] for f in report["files"]] == [path]
    return report["files"][0]


def changed_lines(entry: dict[str, Any]) -> dict[int, tuple[str, list[int | None], int | None]]:
    """line -> (category, hits, base_line), kept_* lines left out."""
    return {
        ln["line"]: (ln["category"], ln["hits"], ln["base_line"])
        for ln in entry["lines"]
        if not ln["category"].startswith("kept_")
    }


def changed_branches(entry: dict[str, Any]) -> list[tuple[int, str, list[int | None]]]:
    return sorted(
        (b["line"], b["category"], b["taken"])
        for b in entry["branches"]
        if b["category"] in BRANCH_CHANGES
    )


def function_rows(entry: dict[str, Any]) -> list[tuple[str, str, int, int | None]]:
    return [(f["name"], f["status"], f["start"], f["end"]) for f in entry["functions"]]


def walk(node: dict[str, Any]) -> Iterator[dict[str, Any]]:
    yield node
    for child in node.get("children", ()):
        yield from walk(child)


def test_skip_divide(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    report, _ = run_preset(repo, "skip_divide.patch")

    assert report["schema"] == "sundgren.delta/1"
    assert report["calls"] is None
    meta = report["meta"]
    assert meta["mode"] == "run"
    assert meta["preset"] == "pytest"
    assert meta["repeat"] == 1
    assert meta["base_ref"] == git(repo, "rev-parse", "HEAD").strip()
    assert meta["patch"]["files"] == [TEST_FILE]
    assert meta["patch"]["sha256"] == sha256_file(PATCHES / "skip_divide.patch")
    assert meta["selection"]["tests"] == [TEST_FILE]
    assert meta["selection"]["run_all"] is False
    assert len(meta["inputs"]["base_lcov_sha256"]) == 1
    assert len(meta["inputs"]["patched_lcov_sha256"]) == 1

    assert report["tests"] == [
        {"id": "tests.test_calc::test_divide", "base": "passed", "patched": "skipped"}
    ]

    summary = report["summary"]
    assert summary["lines"] == counts(LINE_KEYS, lost=len(DIVIDE_BODY))
    assert summary["branches"] == counts(BRANCH_KEYS, lost=2)
    assert summary["functions"] == counts(FUNCTION_KEYS, went_dark=1)
    assert summary["coverage"] == {"base_pct": 100.0, "patched_pct": 75.0}
    assert summary["external_files"] == 0
    assert report["warnings"] == []

    calc = only_file(report, CALC)
    assert calc["base_path"] == CALC
    assert calc["source_changed"] is False
    assert calc["inferred_side"] is None
    digest = sha256_file(repo / CALC)
    assert calc["sha256"] == {"base": digest, "patched": digest}
    assert changed_lines(calc) == {ln: ("lost", [1, 0], ln) for ln in DIVIDE_BODY}
    # coverage.py writes '-' (never reached) for both arcs out of `if b == 0`.
    if_line = min(DIVIDE_BODY)
    assert changed_branches(calc) == [
        (if_line, "branch_lost", [1, None]),
        (if_line, "branch_lost", [1, None]),
    ]
    # add and classify are unchanged, so they are left out.
    assert function_rows(calc) == [("divide", "went_dark", DIVIDE, max(DIVIDE_BODY))]
    assert calc["functions"][0]["lines"]["lost"] == len(DIVIDE_BODY)
    assert calc["functions"][0]["lines"]["gained"] == 0

    tree = report["tree"]
    assert tree["path"] == "" and tree["kind"] == "dir"
    assert tree["stats"] == {
        **dict.fromkeys(("gained", "new_hit", "new_miss", "deleted_hit", "deleted_miss"), 0),
        "lost": len(DIVIDE_BODY),
        "executable_base": 12,
        "executable_patched": 12,
        "hit_base": 12,
        "hit_patched": 12 - len(DIVIDE_BODY),
    }
    nodes = {n["path"]: n for n in walk(tree)}
    assert nodes["shed"]["kind"] == "dir"
    assert nodes[CALC]["kind"] == "file"
    assert nodes[CALC]["stats"]["lost"] == len(DIVIDE_BODY)


def test_drop_zero_case(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    report, _ = run_preset(repo, "drop_zero_case.patch")

    assert report["tests"] == [
        {"id": "tests.test_calc::test_classify[0-zero]", "base": "passed", "patched": "not_run"}
    ]
    summary = report["summary"]
    assert summary["lines"] == counts(LINE_KEYS, lost=1)
    assert summary["branches"] == counts(BRANCH_KEYS, lost=1)
    assert summary["functions"] == counts(FUNCTION_KEYS, partial_loss=1)
    assert report["warnings"] == []

    calc = only_file(report, CALC)
    assert calc["source_changed"] is False
    assert changed_lines(calc) == {RETURN_ZERO: ("lost", [1, 0], RETURN_ZERO)}
    # Only the elif's jump into `return "zero"` loses; its fall-through stays taken.
    assert changed_branches(calc) == [(ELIF, "branch_lost", [1, 0])]
    assert function_rows(calc) == [("classify", "partial_loss", CLASSIFY, CALC_END)]


def check_two_lines_inserted(report: dict[str, Any], before: int) -> None:
    """--full report for two comment lines inserted above base line `before` + `negate` appended."""

    def new(line: int) -> int:
        return line + 2 if line >= before else line

    negate_def = CALC_END + 2 + 2  # after a blank line
    summary = report["summary"]
    # The def line runs at import (new_hit); only the body is new_miss.
    assert summary["lines"] == counts(LINE_KEYS, new_hit=1, new_miss=1)
    assert summary["functions"] == counts(FUNCTION_KEYS, added=1)

    assert [f["path"] for f in report["files"]] == ["shed/__init__.py", CALC]
    calc = report["files"][1]
    assert calc["base_path"] == CALC
    assert calc["source_changed"] is True
    assert changed_lines(calc) == {
        negate_def: ("new_hit", [None, 1], None),
        negate_def + 1: ("new_miss", [None, 0], None),
    }
    kept = [ln for ln in calc["lines"] if ln["category"].startswith("kept_")]
    nodes = {n["path"]: n for n in walk(report["tree"])}
    assert len(kept) == nodes[CALC]["stats"]["executable_base"]
    for ln in kept:
        assert (ln["category"], new(ln["base_line"])) == ("kept_hit", ln["line"])
    assert function_rows(calc) == [
        ("add", "unchanged", new(ADD), new(ADD + 1)),
        ("divide", "unchanged", new(DIVIDE), new(max(DIVIDE_BODY))),
        ("classify", "unchanged", new(CLASSIFY), new(CALC_END)),
        ("negate", "added", negate_def, negate_def + 1),
    ]

    # coverage.py ids name the target line ("jump to line 7"), so a shift renames every
    # branch below it; they must still match up.
    assert changed_branches(calc) == []
    assert summary["branches"] == counts(BRANCH_KEYS)
    assert len(calc["branches"]) == 6
    for br in calc["branches"]:
        assert (br["category"], new(br["base_line"])) == ("branch_kept_hit", br["line"])


def test_touch_source(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    # --full lists kept lines/branches and unchanged functions, so their mapping is checked too.
    report, _ = run_preset(repo, "touch_source.patch", "--full")

    assert report["meta"]["patch"]["files"] == [CALC]
    assert report["meta"]["selection"]["run_all"] is True
    assert len(report["tests"]) == 5
    assert all(t["base"] == t["patched"] == "passed" for t in report["tests"])
    assert report["warnings"] == []
    check_two_lines_inserted(report, before=1)
    calc = report["files"][1]
    assert calc["sha256"]["base"] == sha256_file(repo / CALC)
    assert calc["sha256"]["patched"] != calc["sha256"]["base"]


def test_no_op(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    report, _ = run_preset(repo, "no_op.patch")

    assert report["meta"]["patch"]["files"] == [TEST_FILE]
    assert report["tests"] == []
    assert report["files"] == []
    summary = report["summary"]
    assert summary["lines"] == counts(LINE_KEYS)
    assert summary["branches"] == counts(BRANCH_KEYS)
    assert summary["functions"] == counts(FUNCTION_KEYS)
    assert summary["coverage"]["base_pct"] == summary["coverage"]["patched_pct"]
    assert report["tree"]["stats"]["lost"] == 0
    assert report["tree"]["stats"]["gained"] == 0
    assert report["warnings"] == []


def test_bad_patch_exits_3(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    proc = run_sundgren(
        "run",
        "--repo",
        repo,
        "--patch",
        PATCHES / "bad.patch",
        "--preset",
        "pytest",
        "--python",
        sys.executable,
        "--format",
        "json",
    )
    assert proc.returncode == 3, proc.stderr
    assert TEST_FILE in proc.stderr  # git's message names the file


def test_adhoc_cmd_matches_preset(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    preset, _ = run_preset(repo, "skip_divide.patch")
    adhoc, _ = run_json(
        "run",
        "--repo",
        repo,
        "--patch",
        PATCHES / "skip_divide.patch",
        "--cmd",
        PYTEST_COMMANDS[0],
        "--cmd",
        PYTEST_COMMANDS[1],
        "--lcov",
        "{out}/coverage.lcov",
        "--junit",
        "{out}/junit.xml",
        "--env",
        "COVERAGE_FILE={out}/.coverage",
        "--env",
        "PYTHONHASHSEED=0",
        "--python",
        sys.executable,
    )
    # No selector for ad-hoc commands: the whole suite runs, which here is the same file.
    assert adhoc["meta"]["selection"]["run_all"] is True
    for key in ("summary", "tests", "files"):
        assert adhoc[key] == preset[key], key
    assert adhoc["files"]  # guard against comparing two empty reports


def test_deterministic(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    _, first = run_preset(repo, "skip_divide.patch")
    _, second = run_preset(repo, "skip_divide.patch")
    stamp = re.compile(r'("created_utc"\s*:\s*)"[^"]*"')
    assert len(stamp.findall(first)) == 1
    assert stamp.sub(r'\1"*"', first) == stamp.sub(r'\1"*"', second)


def test_wrong_tree_guard(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path).resolve()
    main_checkout_calc = repo / CALC
    synthetic = tmp_path / "wrong.lcov"
    synthetic.write_text(f"SF:{main_checkout_calc}\nDA:2,1\nDA:3,1\nend_of_record\n")
    hint = "SHED-HINT-7f3a: measure the worktree, not the main checkout"
    copy = "import shutil, sys; shutil.copyfile(sys.argv[1], sys.argv[2])"
    command = f"{{python}} -c {shlex.quote(copy)} {shlex.quote(str(synthetic))} {{lcov}}"
    # json.dumps of an ASCII string is a valid TOML basic string.
    toml = "\n".join(
        [
            "[presets.wrong-tree]",
            f"description = {json.dumps('copies an lcov that points at the main checkout')}",
            f"commands = [{json.dumps(command)}]",
            f"lcov = {json.dumps('{out}/coverage.lcov')}",
            'tests_style = "args"',
            f"hints = [{json.dumps(hint)}]",
            "",
        ]
    )
    commit_files(repo, {"sundgren.toml": toml}, "add wrong-tree preset")

    proc = run_sundgren(
        "run",
        "--repo",
        repo,
        "--patch",
        PATCHES / "skip_divide.patch",
        "--preset",
        "wrong-tree",
        "--python",
        sys.executable,
        "--format",
        "json",
    )
    assert proc.returncode == 1, proc.stderr
    assert str(main_checkout_calc) in proc.stderr
    assert hint in proc.stderr


OFFSET_WARNING = {
    "code": "patch_offset",
    "message": "patch applied at an offset in shed/calc.py; line maps use the applied diff",
}


def test_offset_patch(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    exact, _ = run_preset(repo, "touch_mid.patch", "--full")
    assert exact["warnings"] == []
    check_two_lines_inserted(exact, before=DIVIDE)

    # Same edit made against a base with 3 more lines on top: git applies it 3 lines up.
    offset, _ = run_preset(repo, "touch_mid_offset.patch", "--full")
    assert offset["warnings"] == [OFFSET_WARNING]
    assert offset["meta"]["patch"]["files"] == [CALC]
    for key in ("summary", "tests", "files"):
        assert offset[key] == exact[key], key


def test_offset_patch_hostile_gitconfig(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    gitconfig = tmp_path / "gitconfig"
    gitconfig.write_text(
        "[diff]\n\tmnemonicPrefix = true\n\tnoprefix = true\n\texternal = false\n"
        "\trenames = copies\n\tcontext = 1\n[color]\n\tui = always\n"
    )
    report, _ = run_json(
        "run",
        "--repo",
        repo,
        "--patch",
        PATCHES / "touch_mid_offset.patch",
        "--preset",
        "pytest",
        "--python",
        sys.executable,
        "--full",
        gitconfig=gitconfig,
    )
    assert report["warnings"] == [OFFSET_WARNING]
    check_two_lines_inserted(report, before=DIVIDE)


def test_mnemonic_prefix_patch(tmp_path: Path) -> None:
    repo = make_shed_py(tmp_path)
    plain, _ = run_preset(repo, "skip_divide.patch")
    # Made with diff.mnemonicPrefix=true: `i/tests/test_calc.py` and `w/tests/test_calc.py`.
    mnemonic, _ = run_preset(repo, "skip_divide_mnemonic.patch")
    assert mnemonic["warnings"] == []
    assert mnemonic["meta"]["patch"]["files"] == [TEST_FILE]
    assert mnemonic["meta"]["selection"]["tests"] == [TEST_FILE]
    assert mnemonic["meta"]["selection"]["run_all"] is False
    for key in ("summary", "tests", "files"):
        assert mnemonic[key] == plain[key], key
    assert mnemonic["files"]  # guard against comparing two empty reports
