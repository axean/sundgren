from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path

import pytest

from environments import describe, index, main, write

NOW = datetime(2026, 10, 31, 12, 0, tzinfo=UTC)


def site(directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "index.html").write_text("<html></html>")


def test_describe() -> None:
    assert describe(".") == {"path": "", "kind": "release"}
    assert describe("main") == {"path": "main/", "kind": "main"}
    assert describe("pr-preview/pr-23") == {"path": "pr-preview/pr-23/", "kind": "pr", "number": 23}
    with pytest.raises(ValueError):
        describe("pr-preview/other")


def test_write_takes_what_is_set(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.chdir(tmp_path)
    (tmp_path / "pr-preview" / "pr-7").mkdir(parents=True)
    environ = {"SUNDGREN_ENV_TITLE": "History graph", "SUNDGREN_ENV_REF": "", "X": "y"}
    entry = write("pr-preview/pr-7", environ, NOW)
    assert entry == {
        "path": "pr-preview/pr-7/",
        "kind": "pr",
        "number": 7,
        "title": "History graph",
        "updated": "2026-10-31T12:00:00Z",
    }
    saved = json.loads((tmp_path / "pr-preview" / "pr-7" / "environment.json").read_text())
    assert saved == entry


def test_index_lists_sites_live_first_then_main_then_newest_pr(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.chdir(tmp_path)
    for dest in [".", "main", "pr-preview/pr-3", "pr-preview/pr-12"]:
        site(tmp_path / dest)
    (tmp_path / "pr-preview" / "pr-5").mkdir()  # no site: left out
    write("pr-preview/pr-12", {"SUNDGREN_ENV_TITLE": "Newer"}, NOW)
    found = index(tmp_path)
    assert [e["path"] for e in found] == ["", "main/", "pr-preview/pr-12/", "pr-preview/pr-3/"]
    assert found[2]["title"] == "Newer"
    assert "title" not in found[3]


def test_main_writes_the_index(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.chdir(tmp_path)
    site(tmp_path / "main")
    assert main(["index"]) == 0
    saved = json.loads((tmp_path / "environments.json").read_text())
    assert saved == {"environments": [{"path": "main/", "kind": "main"}]}
    assert main(["bogus"]) == 2
