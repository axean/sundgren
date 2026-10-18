"""pytest_files selector: run touched test files, or everything if config/source changed."""

from __future__ import annotations

from fnmatch import fnmatchcase
from pathlib import Path, PurePosixPath

from sundgren.core.patch import FilePatch, Patch
from sundgren.run.selectors import Selection

_CONFIG_FILES = frozenset({"pytest.ini", "pyproject.toml", "setup.cfg", "tox.ini"})


class PytestFilesSelector:
    def select(self, patch: Patch, repo: Path) -> Selection:
        selected = []
        for fp in sorted(patch.files, key=lambda f: f.path):
            names = [PurePosixPath(p).name for p in (fp.old_path, fp.new_path) if p is not None]
            if not any(n.endswith(".py") or n in _CONFIG_FILES for n in names):
                continue
            if not all(_is_test_file(n) for n in names):
                return _run_all(f"{fp.path} changed")
            # The same selection runs on both sides, so the file must exist in both.
            if fp.old_path != fp.new_path:
                return _run_all(f"test file {fp.path} {_change_kind(fp)}")
            if (repo / fp.path).is_file():
                selected.append(fp.path)
        if not selected:
            return _run_all("no touched test files")
        return Selection(tests=tuple(sorted(selected)), run_all=False, reason="touched test files")


def _is_test_file(name: str) -> bool:
    return fnmatchcase(name, "test_*.py") or fnmatchcase(name, "*_test.py")


def _change_kind(fp: FilePatch) -> str:
    if fp.old_path is None:
        return "added"
    if fp.new_path is None:
        return "deleted"
    return "renamed"


def _run_all(why: str) -> Selection:
    return Selection(tests=(), run_all=True, reason=f"{why}; running all tests")
