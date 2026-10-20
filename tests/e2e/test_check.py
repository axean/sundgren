"""`sundgren run --check`: the plan and the problems as sundgren.check/1 JSON; nothing left."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest

from support import ROOT
from support.shed import SUNDGREN, clean_env, commit_files, git, make_shed_py, run_sundgren

SCHEMA = ROOT / "schema" / "check.schema.json"
CALC = "shed/calc.py"


def validate(report: Any) -> None:
    from jsonschema import Draft202012Validator

    schema = json.loads(SCHEMA.read_text())
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(report)  # pyright: ignore[reportUnknownMemberType]


def check(repo: Path, patch: Path, *args: str) -> tuple[int, dict[str, Any], str]:
    """Run --check; return (exit code, validated report, stderr)."""
    proc = run_sundgren("run", "--check", "--repo", repo, "--patch", patch, *args)
    report = json.loads(proc.stdout)
    validate(report)
    assert report["ok"] == (proc.returncode == 0) == (report["problems"] == [])
    return proc.returncode, report, proc.stderr


def codes(report: dict[str, Any]) -> list[str]:
    return [p["code"] for p in report["problems"]]


def edit_patch(repo: Path, rel: str, old: str, new: str) -> Path:
    path = repo / rel
    text = path.read_text()
    path.write_text(text.replace(old, new))
    patch = repo.parent / "p.patch"
    patch.write_text(git(repo, "diff"))
    path.write_text(text)
    return patch


@pytest.fixture
def shed(tmp_path: Path) -> tuple[Path, Path]:
    repo = make_shed_py(tmp_path)
    return repo, edit_patch(repo, CALC, "return a + b", "return b + a")


def fake_python(tmp_path: Path, stderr: str) -> Path:
    path = tmp_path / "python"
    path.write_text(f"#!/bin/sh\necho {stderr!r} >&2\nexit 1\n")
    path.chmod(0o755)
    return path


def test_ok_pytest(shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    code, report, err = check(repo, patch, "--preset", "pytest", "--python", sys.executable)
    assert (code, err) == (0, "")
    assert report["repo"] == str(repo.resolve())
    assert report["base_ref"] == git(repo, "rev-parse", "HEAD").strip()
    assert (report["preset"], report["python"]) == ("pytest", sys.executable)
    assert report["selection"]["run_all"]
    assert report["commands"][0].startswith(f"{sys.executable} -m coverage run")


def test_all_tests(shed: tuple[Path, Path]) -> None:
    repo, _ = shed
    patch = edit_patch(repo, "tests/test_calc.py", "import pytest", "import os\nimport pytest")
    args = ("--preset", "pytest", "--python", sys.executable)
    _, report, _ = check(repo, patch, *args)
    assert report["selection"]["tests"] == ["tests/test_calc.py"]
    _, report, _ = check(repo, patch, *args, "--all-tests")
    assert report["selection"] == {"tests": [], "run_all": True, "reason": "--all-tests given"}
    proc = run_sundgren("run", "--repo", repo, "--patch", patch, "--all-tests", "--tests", "x")
    assert proc.returncode == 2


def test_missing_module(tmp_path: Path, shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    python = fake_python(tmp_path, "ModuleNotFoundError: No module named 'coverage'")
    code, report, err = check(repo, patch, "--preset", "pytest", "--python", str(python))
    assert code == 1
    message = (
        f"coverage isn't installed for {python}: "
        "install it into that environment or use another interpreter"
    )
    assert report["problems"] == [{"code": "check_failed", "message": message}]
    assert err == f"sundgren: check failed: {message}\n"


def test_missing_tool(shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    _, report, _ = check(repo, patch, "--preset", "pytest", "--python", "/nonexistent/python")
    assert report["problems"] == [
        {
            "code": "check_failed",
            "message": "/nonexistent/python was not found, but the pytest preset needs it: "
            "install it or add it to PATH",
        }
    ]


def test_user_preset_checks(shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    checks = '["true", "sh -c \'echo nope; exit 4\'"]'
    preset = f'[presets.p]\ncommands = ["true"]\nlcov = "l"\nchecks = {checks}\n'
    commit_files(repo, {"sundgren.toml": preset})
    _, report, _ = check(repo, patch, "--preset", "p")
    assert report["python"] is None
    assert report["problems"] == [
        {
            "code": "check_failed",
            "message": "the p preset's check `sh -c 'echo nope; exit 4'` failed (exit 4): nope",
        }
    ]


def test_dirty_tree(shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    (repo / "pyproject.toml").write_text("changed\n")
    code, report, _ = check(repo, patch, "--cmd", "true", "--lcov", "l")
    assert code == 1
    assert codes(report) == ["dirty_tree"]
    assert report["base_ref"] is not None  # the other checks still ran


def test_patch_does_not_apply(shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    commit_files(repo, {CALC: "changed\n"})
    code, report, _ = check(repo, patch, "--cmd", "true", "--lcov", "l")
    assert code == 3
    assert codes(report) == ["patch_does_not_apply"]
    assert report["problems"][0]["message"].startswith("the patch does not apply to the base: ")
    assert report["selection"] is None


def test_usage_problems_win(tmp_path: Path, shed: tuple[Path, Path]) -> None:
    repo, patch = shed
    missing = tmp_path / "missing.patch"
    code, report, _ = check(repo, missing, "--preset", "nope", "--repeat", "0")
    assert code == 2
    assert codes(report) == ["usage", "patch_not_found", "preset"]
    code, report, _ = check(repo, patch, "--cmd", "true", "--lcov", "l", "--base", "nope")
    assert code == 2
    assert report["problems"] == [
        {"code": "bad_base", "message": "base ref 'nope' is not a commit in this repository"}
    ]


def test_not_a_repo(tmp_path: Path) -> None:
    patch = tmp_path / "p.patch"
    patch.write_text("")
    env = clean_env(GIT_CEILING_DIRECTORIES=str(tmp_path), GIT_CONFIG_GLOBAL=os.devnull)
    args = ["run", "--check", "--repo", str(tmp_path), "--patch", str(patch)]
    proc = subprocess.run([SUNDGREN, *args], capture_output=True, text=True, env=env)
    report = json.loads(proc.stdout)
    validate(report)
    assert proc.returncode == 2
    assert codes(report) == ["not_a_repo"]
    assert report["repo"] is None
