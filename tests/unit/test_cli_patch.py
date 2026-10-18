"""cli: patch reading, run mode on the applied diff, git error exit codes."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

import pytest

from sundgren.cli import EXIT_FAILURE, EXIT_OK, EXIT_PATCH, EXIT_USAGE, main

LINES = "".join(f"line {i}\n" for i in range(1, 21))
LCOV = "printf 'SF:lines.txt\\nDA:1,1\\nend_of_record\\n' > cov.lcov"


def git(repo: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    )
    return proc.stdout


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    for key, value in [("user.name", "T"), ("user.email", "t@e"), ("commit.gpgsign", "false")]:
        git(repo, "config", key, value)
    (repo / "lines.txt").write_text(LINES)
    git(repo, "add", "lines.txt")
    git(repo, "commit", "-q", "-m", "init")
    return repo


def _patch(repo: Path, path: Path, *config: str) -> Path:
    (repo / "lines.txt").write_text(LINES.replace("line 12\n", "line 12\nnew a\nnew b\n"))
    path.write_text(git(repo, *config, "diff", "--no-color"))
    git(repo, "checkout", "--", "lines.txt")
    return path


def _run(repo: Path, patch: Path, out: Path, *extra: str) -> int:
    argv = ["run", "--repo", str(repo), "--patch", str(patch), "-o", str(out)]
    return main([*argv, "--cmd", LCOV, "--shell", "--lcov", "cov.lcov", *extra])


def _report(out: Path) -> dict[str, Any]:
    report: dict[str, Any] = json.loads(out.read_text())
    return report


def test_offset_patch_warns_and_uses_applied_hunks(repo: Path, tmp_path: Path) -> None:
    patch = _patch(repo, tmp_path / "exact.patch")
    offset = tmp_path / "offset.patch"
    offset.write_text(patch.read_text().replace("@@ -10,6 +10,8 @@", "@@ -13,6 +13,8 @@"))
    assert offset.read_text() != patch.read_text()
    assert _run(repo, offset, tmp_path / "r.json") == EXIT_OK
    report = _report(tmp_path / "r.json")
    assert report["warnings"] == [
        {
            "code": "patch_offset",
            "message": "patch applied at an offset in lines.txt; line maps use the applied diff",
        }
    ]
    sha = hashlib.sha256(offset.read_bytes()).hexdigest()
    assert report["meta"]["patch"] == {"path": str(offset), "sha256": sha, "files": ["lines.txt"]}

    assert _run(repo, patch, tmp_path / "exact.json") == EXIT_OK
    exact = _report(tmp_path / "exact.json")
    assert exact["warnings"] == []
    assert exact["files"] == report["files"]


def test_mnemonic_prefix_patch(repo: Path, tmp_path: Path) -> None:
    patch = _patch(repo, tmp_path / "iw.patch", "-c", "diff.mnemonicPrefix=true")
    assert "--- i/lines.txt" in patch.read_text()
    assert _run(repo, patch, tmp_path / "r.json") == EXIT_OK
    report = _report(tmp_path / "r.json")
    assert report["warnings"] == []
    assert report["meta"]["patch"]["files"] == ["lines.txt"]


def test_patch_that_does_not_apply(repo: Path, tmp_path: Path) -> None:
    bad = tmp_path / "bad.patch"
    bad.write_text(_patch(repo, bad).read_text().replace(" line 11\n", " line eleven\n"))
    assert _run(repo, bad, tmp_path / "r.json") == EXIT_PATCH


def test_bad_base_ref_is_usage_error(
    repo: Path, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    patch = _patch(repo, tmp_path / "p.patch")
    assert _run(repo, patch, tmp_path / "r.json", "--base", "no-such-ref") == EXIT_USAGE
    assert "no-such-ref" in capsys.readouterr().err


def test_other_git_failure_exits_1(
    repo: Path, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    patch = _patch(repo, tmp_path / "p.patch")
    workdir = tmp_path / "wd"
    (workdir / "base").mkdir(parents=True)
    (workdir / "base" / "taken").write_text("x")  # git worktree add refuses a non-empty dir
    assert _run(repo, patch, tmp_path / "r.json", "--workdir", str(workdir)) == EXIT_FAILURE
    assert "sundgren: git: " in capsys.readouterr().err


def test_diff_mode_patch_read_as_bytes(tmp_path: Path) -> None:
    lcov = tmp_path / "a.lcov"
    lcov.write_text("SF:a.txt\nDA:1,1\nDA:2,1\nend_of_record\n")
    patch = tmp_path / "crlf.patch"
    patch.write_bytes(
        b"diff --git a/a.txt b/a.txt\r\n--- a/a.txt\r\n+++ b/a.txt\r\n"
        b"@@ -1,2 +1,3 @@\r\n caf\xe9\r\n+new\r\n x\r\n"
    )
    out = tmp_path / "r.json"
    argv = ["diff", str(lcov), str(lcov), "--patch", str(patch), "-o", str(out)]
    assert main(argv) == EXIT_OK
    assert _report(out)["meta"]["patch"]["files"] == ["a.txt"]
