from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from sundgren.core.linemap import build_linemaps
from sundgren.core.patch import FilePatch, parse_patch
from sundgren.run.workspace import (
    BadRefError,
    GitError,
    PatchApplyError,
    Workspace,
    is_dirty,
    repo_root,
)

GOOD_PATCH = """\
diff --git a/calc.py b/calc.py
--- a/calc.py
+++ b/calc.py
@@ -1,2 +1,2 @@
 def add(a, b):
-    return a + b
+    return b + a
"""

BAD_PATCH = GOOD_PATCH.replace("return a + b", "return a - b")


def git(repo: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    )
    return proc.stdout.strip()


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q")
    for key, value in [
        ("user.name", "Test"),
        ("user.email", "test@example.com"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", "/dev/null"),
    ]:
        git(repo, "config", key, value)
    (repo / "calc.py").write_text("def add(a, b):\n    return a + b\n")
    git(repo, "add", "calc.py")
    git(repo, "commit", "-q", "-m", "init")
    return repo


@pytest.fixture
def patches(tmp_path: Path) -> tuple[Path, Path]:
    good = tmp_path / "good.patch"
    good.write_text(GOOD_PATCH)
    bad = tmp_path / "bad.patch"
    bad.write_text(BAD_PATCH)
    return good, bad


def worktrees(repo: Path) -> list[str]:
    lines = git(repo, "worktree", "list", "--porcelain").splitlines()
    return [line.removeprefix("worktree ") for line in lines if line.startswith("worktree ")]


def test_patch_applies_and_cleans_up(repo: Path, patches: tuple[Path, Path]) -> None:
    with Workspace(repo, "HEAD", patches[0], workdir=None, keep=False) as ws:
        assert ws.base_sha == git(repo, "rev-parse", "HEAD")
        assert (ws.base / "calc.py").read_text().endswith("return a + b\n")
        assert (ws.patched / "calc.py").read_text().endswith("return b + a\n")
        assert ws.base.parent == ws.patched.parent == ws.workdir
        assert ws.workdir.name.startswith("sundgren-")
        assert len(worktrees(repo)) == 3
        assert git(ws.patched, "rev-parse", "HEAD") == ws.base_sha
    assert worktrees(repo) == [str(repo)]
    assert not ws.workdir.exists()
    assert not is_dirty(repo)


def test_patch_does_not_apply(repo: Path, patches: tuple[Path, Path], tmp_path: Path) -> None:
    workdir = tmp_path / "wd"
    with (
        pytest.raises(PatchApplyError, match="patch does not apply") as info,
        Workspace(repo, "HEAD", patches[1], workdir=workdir, keep=True),
    ):
        pass
    assert "Checking patch" not in str(info.value)
    assert worktrees(repo) == [str(repo)]
    assert list(workdir.iterdir()) == []


def test_temp_workdir_removed_on_apply_failure(
    repo: Path, patches: tuple[Path, Path], monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    temp = tmp_path / "tmp"
    temp.mkdir()
    monkeypatch.setenv("TMPDIR", str(temp))
    monkeypatch.setattr("tempfile.tempdir", None)
    with (
        pytest.raises(PatchApplyError),
        Workspace(repo, "HEAD", patches[1], workdir=None, keep=False),
    ):
        pass
    assert list(temp.iterdir()) == []


def test_keep_leaves_worktrees(repo: Path, patches: tuple[Path, Path]) -> None:
    with Workspace(repo, "HEAD", patches[0], workdir=None, keep=True) as ws:
        pass
    assert worktrees(repo) == [str(repo), str(ws.base), str(ws.patched)]
    assert (ws.patched / "calc.py").read_text().endswith("return b + a\n")
    git(repo, "worktree", "remove", "--force", str(ws.base))
    git(repo, "worktree", "remove", "--force", str(ws.patched))
    ws.workdir.rmdir()


def test_given_workdir_kept_with_its_other_contents(
    repo: Path, patches: tuple[Path, Path], tmp_path: Path
) -> None:
    workdir = tmp_path / "wd"
    with Workspace(repo, "HEAD", patches[0], workdir=workdir, keep=False) as ws:
        assert ws.base == workdir / "base"
        (workdir / "out").mkdir()
    assert worktrees(repo) == [str(repo)]
    assert sorted(p.name for p in workdir.iterdir()) == ["out"]


def test_cleanup_on_exception_in_body(repo: Path, patches: tuple[Path, Path]) -> None:
    with (
        pytest.raises(RuntimeError, match="boom"),
        Workspace(repo, "HEAD", patches[0], workdir=None, keep=False),
    ):
        raise RuntimeError("boom")
    assert worktrees(repo) == [str(repo)]


def test_base_is_the_ref_not_the_working_tree(repo: Path, patches: tuple[Path, Path]) -> None:
    first = git(repo, "rev-parse", "HEAD")
    (repo / "calc.py").write_text("def add(a, b):\n    return a + b\n\n# more\n")
    git(repo, "commit", "-qam", "second")
    with Workspace(repo, first, patches[0], workdir=None, keep=False) as ws:
        assert ws.base_sha == first
        assert "# more" not in (ws.base / "calc.py").read_text()


def test_bad_ref(repo: Path, patches: tuple[Path, Path]) -> None:
    with (
        pytest.raises(BadRefError, match="no-such-ref"),
        Workspace(repo, "no-such-ref", patches[0], workdir=None, keep=False),
    ):
        pass
    assert worktrees(repo) == [str(repo)]


def test_repo_root(repo: Path) -> None:
    sub = repo / "pkg"
    sub.mkdir()
    assert repo_root(sub) == repo


def test_repo_root_outside_git(tmp_path: Path) -> None:
    with pytest.raises(GitError):
        repo_root(tmp_path)


def test_is_dirty_tracked_changes_only(repo: Path) -> None:
    assert not is_dirty(repo)
    (repo / "untracked.txt").write_text("x")
    assert not is_dirty(repo)
    (repo / "calc.py").write_text("changed\n")
    assert is_dirty(repo)


LINES = "".join(f"line {i}\n" for i in range(1, 21))


def _commit_lines(repo: Path) -> None:
    (repo / "lines.txt").write_text(LINES)
    git(repo, "add", "lines.txt")
    git(repo, "commit", "-q", "-m", "lines")


def _changed_lines_patch(repo: Path, tmp_path: Path, name: str, *config: str) -> Path:
    """Real patch: line 10 changed, two lines inserted after line 12."""
    text = LINES.replace("line 10\n", "LINE 10\n").replace("line 12\n", "line 12\nnew a\nnew b\n")
    (repo / "lines.txt").write_text(text)
    patch = tmp_path / name
    patch.write_text(git(repo, *config, "diff", "--no-color") + "\n")
    git(repo, "checkout", "--", "lines.txt")
    return patch


def test_applied_patch_matches_exact_patch(repo: Path, tmp_path: Path) -> None:
    _commit_lines(repo)
    patch = _changed_lines_patch(repo, tmp_path, "exact.patch")
    with Workspace(repo, "HEAD", patch, workdir=None, keep=False) as ws:
        assert ws.offset_paths == ()
        assert build_linemaps(parse_patch(ws.applied_patch)) == build_linemaps(
            parse_patch(patch.read_text())
        )
        assert git(ws.patched, "status", "--porcelain") == "M  lines.txt"


def test_offset_patch_uses_applied_hunks(repo: Path, tmp_path: Path) -> None:
    _commit_lines(repo)
    exact = _changed_lines_patch(repo, tmp_path, "exact.patch")
    # The patch was made against a base with three more lines at the top.
    shifted = exact.read_text().replace("@@ -7,9 +7,11 @@", "@@ -10,9 +10,11 @@")
    assert shifted != exact.read_text()
    offset = tmp_path / "offset.patch"
    offset.write_text(shifted)
    with Workspace(repo, "HEAD", offset, workdir=None, keep=False) as ws:
        assert ws.offset_paths == ("lines.txt",)
        applied = build_linemaps(parse_patch(ws.applied_patch))["lines.txt"]
        assert (applied.deleted_old, applied.added_new) == ({10}, {10, 13, 14})
        assert applied.to_new(9) == 9 and applied.to_new(13) == 15
        assert (ws.patched / "lines.txt").read_text().count("new a") == 1
    # The user's headers would put the change three lines too low.
    assert build_linemaps(parse_patch(shifted))["lines.txt"].added_new == {13, 16, 17}


def test_mnemonic_prefix_patch(repo: Path, tmp_path: Path) -> None:
    _commit_lines(repo)
    patch = _changed_lines_patch(repo, tmp_path, "mnemonic.patch", "-c", "diff.mnemonicPrefix=true")
    assert "diff --git i/lines.txt w/lines.txt" in patch.read_text()
    with Workspace(repo, "HEAD", patch, workdir=None, keep=False) as ws:
        assert ws.offset_paths == ()
        assert "diff --git a/lines.txt b/lines.txt" in ws.applied_patch
        (fp,) = parse_patch(ws.applied_patch).files
        assert (fp.old_path, fp.new_path) == ("lines.txt", "lines.txt")


def test_user_git_config_cannot_change_applied_diff(
    repo: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    _commit_lines(repo)
    patch = _changed_lines_patch(repo, tmp_path, "exact.patch")
    with Workspace(repo, "HEAD", patch, workdir=None, keep=False) as ws:
        clean = ws.applied_patch
    hostile = tmp_path / "hostile.gitconfig"
    hostile.write_text(
        "[diff]\n\tnoprefix = true\n\tmnemonicPrefix = true\n\trelative = true\n"
        "\texternal = false\n\trenames = copies\n\tcontext = 1\n\talgorithm = histogram\n"
        "[color]\n\tui = always\n\tdiff = always\n[core]\n\tquotePath = false\n"
    )
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(hostile))
    with Workspace(repo, "HEAD", patch, workdir=None, keep=False) as ws:
        assert ws.applied_patch == clean


def test_new_deleted_renamed_binary_and_mode_only(repo: Path, tmp_path: Path) -> None:
    (repo / "gone.txt").write_text("bye\n")
    (repo / "old name.txt").write_text("".join(f"keep {i}\n" for i in range(10)))
    (repo / "run.sh").write_text("echo hi\n")
    (repo / "img.bin").write_bytes(bytes(range(256)))
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "more")
    (repo / "gone.txt").unlink()
    (repo / "old name.txt").rename(repo / "new name.txt")
    (repo / "new name.txt").write_text("".join(f"keep {i}\n" for i in range(10)) + "added\n")
    (repo / "run.sh").chmod(0o755)
    (repo / "img.bin").write_bytes(bytes(range(255, -1, -1)))
    (repo / "fresh.py").write_text("x = 1\n")
    git(repo, "add", "-A")
    patch = tmp_path / "kinds.patch"
    patch.write_bytes(
        subprocess.run(
            ["git", "-C", str(repo), "diff", "--cached", "--binary", "-M"],
            capture_output=True,
            check=True,
        ).stdout
    )
    git(repo, "reset", "-q", "--hard")
    with Workspace(repo, "HEAD", patch, workdir=None, keep=False) as ws:
        assert not (ws.patched / "gone.txt").exists()
        assert (ws.patched / "new name.txt").read_text().endswith("added\n")
        assert (ws.patched / "run.sh").stat().st_mode & 0o111
        assert (ws.patched / "img.bin").read_bytes()[0] == 255
        parsed = parse_patch(ws.applied_patch)
    by_path = {fp.path: fp for fp in parsed.files}
    assert sorted(by_path) == ["fresh.py", "gone.txt", "img.bin", "new name.txt", "run.sh"]
    assert (by_path["fresh.py"].old_path, by_path["gone.txt"].new_path) == (None, None)
    assert by_path["new name.txt"].old_path == "old name.txt"
    assert by_path["img.bin"].binary
    assert by_path["run.sh"] == FilePatch("run.sh", "run.sh", ())
    assert parsed.warnings == ("img.bin: binary diff ignored",)
