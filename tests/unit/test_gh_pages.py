"""tools/gh-pages.sh against a local gh-pages remote."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools" / "gh-pages.sh"
ENV = {**os.environ, "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(cwd), *args], check=True, capture_output=True, text=True, env=ENV
    ).stdout


def site(path: Path, text: str) -> Path:
    path.mkdir(parents=True)
    (path / "index.html").write_text(text)
    return path


def clone(remote: Path, path: Path) -> Path:
    git(remote.parent, "clone", "-q", "-b", "gh-pages", str(remote), str(path))
    return path


def deploy(checkout: Path, dest: str, built: Path | None = None, **env: str) -> None:
    args = [str(SCRIPT), str(checkout), dest, *([str(built)] if built else [])]
    subprocess.run(args, check=True, capture_output=True, env={**ENV, **env})


def published(remote: Path, tmp: Path) -> Path:
    return clone(remote, tmp / "check")


def test_deploys_and_indexes_every_environment(tmp_path: Path) -> None:
    remote = tmp_path / "remote.git"
    git(tmp_path, "init", "-q", "--bare", "-b", "gh-pages", str(remote))
    seed = tmp_path / "seed"
    git(tmp_path, "init", "-q", "-b", "gh-pages", str(seed))
    site(seed / "main", "old main")
    git(seed, "add", "-A")
    git(seed, "-c", "user.name=a", "-c", "user.email=a@b", "commit", "-q", "-m", "seed")
    git(seed, "push", "-q", str(remote), "gh-pages")

    # Two previews from checkouts made before either pushed, as concurrent jobs would.
    first = clone(remote, tmp_path / "first")
    second = clone(remote, tmp_path / "second")
    deploy(first, "pr-preview/pr-4", site(tmp_path / "s4", "four"), SUNDGREN_ENV_TITLE="Four")
    deploy(second, ".", site(tmp_path / "live", "live"), SUNDGREN_ENV_TITLE="0.3.0")

    out = published(remote, tmp_path)
    assert (out / "pr-preview" / "pr-4" / "index.html").read_text() == "four"
    assert (out / "index.html").read_text() == "live"
    assert (out / "main" / "index.html").read_text() == "old main"
    assert (out / ".nojekyll").is_file()
    index = json.loads((out / "environments.json").read_text())["environments"]
    assert [(e["path"], e.get("title")) for e in index] == [
        ("", "0.3.0"),
        ("main/", None),
        ("pr-preview/pr-4/", "Four"),
    ]

    deploy(first, "pr-preview/pr-4")
    out = clone(remote, tmp_path / "after")
    assert not (out / "pr-preview" / "pr-4").exists()
    index = json.loads((out / "environments.json").read_text())["environments"]
    assert [e["path"] for e in index] == ["", "main/"]
