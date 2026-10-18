from __future__ import annotations

from pathlib import Path

import pytest

from sundgren.core.patch import FilePatch, Patch
from sundgren.lang.python.selector import PytestFilesSelector
from sundgren.run import selectors
from sundgren.run.selectors import RUN_ALL, Selection, choose


def _patch(*paths: tuple[str | None, str | None]) -> Patch:
    return Patch(files=tuple(FilePatch(old, new, ()) for old, new in paths))


def _same(*paths: str) -> Patch:
    return _patch(*((p, p) for p in paths))


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    for rel in ("tests/test_calc.py", "tests/io_test.py", "tests/test_other.py", "src/calc.py"):
        (tmp_path / rel).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / rel).write_text("")
    return tmp_path


def select(patch: Patch, repo: Path) -> Selection:
    return PytestFilesSelector().select(patch, repo)


def test_touched_test_files_selected_sorted(repo: Path) -> None:
    sel = select(_same("tests/test_other.py", "tests/io_test.py", "tests/test_calc.py"), repo)
    assert sel == Selection(
        tests=("tests/io_test.py", "tests/test_calc.py", "tests/test_other.py"),
        run_all=False,
        reason="touched test files",
    )


def test_docs_alongside_tests_are_ignored(repo: Path) -> None:
    sel = select(_same("README.md", "tests/test_calc.py"), repo)
    assert sel.tests == ("tests/test_calc.py",)
    assert not sel.run_all


@pytest.mark.parametrize(
    "path",
    [
        "src/calc.py",
        "tests/conftest.py",
        "pytest.ini",
        "pyproject.toml",
        "sub/setup.cfg",
        "tox.ini",
    ],
)
def test_source_or_config_runs_all(repo: Path, path: str) -> None:
    sel = select(_same("tests/test_calc.py", path), repo)
    assert sel.run_all
    assert sel.tests == ()
    assert path in sel.reason


def test_reason_names_first_offending_file(repo: Path) -> None:
    sel = select(_same("tox.ini", "src/calc.py", "tests/test_calc.py"), repo)
    assert sel.reason == "src/calc.py changed; running all tests"


def test_only_docs_runs_all(repo: Path) -> None:
    sel = select(_same("README.md", "docs/index.rst"), repo)
    assert sel.run_all
    assert sel.reason == "no touched test files; running all tests"


@pytest.mark.parametrize(
    ("old", "new", "kind"),
    [
        (None, "tests/test_new.py", "added"),
        ("tests/test_gone.py", None, "deleted"),
        ("tests/test_old.py", "tests/test_calc.py", "renamed"),
    ],
)
def test_test_file_not_on_both_sides_runs_all(
    repo: Path, old: str | None, new: str | None, kind: str
) -> None:
    sel = select(_patch((old, new), ("tests/test_other.py", "tests/test_other.py")), repo)
    assert sel.run_all
    assert sel.reason.endswith(f"{kind}; running all tests")


def test_missing_in_patched_tree_not_selected(repo: Path) -> None:
    sel = select(_same("tests/test_missing.py"), repo)
    assert sel.run_all


def test_choose_precedence(repo: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(selectors._REGISTRY, "test.fixed", PytestFilesSelector())
    patch = _same("tests/test_calc.py")
    assert choose(("x",), "test.fixed", patch, repo).tests == ("x",)
    assert choose(None, "test.fixed", patch, repo).tests == ("tests/test_calc.py",)
    assert choose(None, None, patch, repo) == RUN_ALL
