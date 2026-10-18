from __future__ import annotations

from pathlib import Path

from sundgren.run.guards import wrong_tree_paths


def test_flags_main_repo_paths_only(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    worktree = tmp_path / "work" / "patched"
    raw = [
        f"{worktree}/src/a.py",
        f"{repo}/src/a.py",
        "/usr/lib/python3/site.py",
        "src/b.py",
        f"{repo}/src/a.py",
    ]
    assert wrong_tree_paths(raw, worktree, repo) == [f"{repo}/src/a.py"]


def test_worktree_inside_main_repo(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    base = repo / ".sundgren" / "base"
    patched = repo / ".sundgren" / "patched"
    raw = [f"{base}/src/a.py", f"{patched}/src/a.py", f"{repo}/src/a.py"]
    assert wrong_tree_paths(raw, base, repo) == [f"{patched}/src/a.py", f"{repo}/src/a.py"]


def test_sibling_prefix_is_not_inside(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    raw = [f"{tmp_path}/repo2/a.py", f"{repo}/a.py"]
    assert wrong_tree_paths(raw, tmp_path / "wt", repo) == [f"{repo}/a.py"]


def test_symlinks_resolved(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    (repo / "src").mkdir(parents=True)
    worktree = tmp_path / "wt"
    worktree.mkdir()
    (tmp_path / "link-to-repo").symlink_to(repo)
    (tmp_path / "link-to-wt").symlink_to(worktree)
    raw = [f"{tmp_path}/link-to-repo/src/a.py", f"{tmp_path}/link-to-wt/src/a.py"]
    assert wrong_tree_paths(raw, worktree, repo) == [f"{tmp_path}/link-to-repo/src/a.py"]
    # A worktree given via a symlink still contains its real paths.
    assert wrong_tree_paths([f"{worktree}/a.py"], tmp_path / "link-to-wt", tmp_path) == []
