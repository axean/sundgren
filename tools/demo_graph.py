"""The history demo: `sundgren graph`, the branch map, of a made-up repository.

Usage: uv run tools/demo_graph.py OUT. The docs build writes it (`_static/demo/graph.js`,
docs/_ext/sundgren_docs.py) with the sundgren it runs with. Commit shas are stable: fixed identity
and dates.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

# graph.js is this line, then the JSON. A script, not a .json fetch: it also works from file://.
PREFIX = "window.sundgrenDemoGraphs =\n"
# The demo can't run sundgren, so it carries every view the page can ask for at this width:
# 1 to BRANCHES branches, each frame from the tip back to the start of history.
COLUMNS = 24
BRANCHES = 4
SUNDGREN = os.environ.get("SUNDGREN_BIN") or str(Path(sys.executable).with_name("sundgren"))
START = 1_767_225_600  # 2026-01-01T00:00:00Z
PEOPLE = [
    ("Ada", "ada@example.com"),
    ("Grace", "grace@example.com"),
    ("Linus", "linus@example.com"),
]


class Repo:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.clock = START
        self.author = 0

    def git(self, *args: str) -> None:
        self.clock += 3600
        name, email = PEOPLE[self.author % len(PEOPLE)]
        date = f"@{self.clock} +0000"
        env = {
            **os.environ,
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_AUTHOR_NAME": name,
            "GIT_AUTHOR_EMAIL": email,
            "GIT_COMMITTER_NAME": name,
            "GIT_COMMITTER_EMAIL": email,
            "GIT_AUTHOR_DATE": date,
            "GIT_COMMITTER_DATE": date,
        }
        cmd = ["git", "-C", str(self.path), *args]
        subprocess.run(cmd, check=True, env=env, capture_output=True)

    def commit(self, message: str, author: int | None = None) -> None:
        if author is not None:
            self.author = author
        self.git("commit", "-q", "--allow-empty", "-m", message)

    def branch(self, name: str) -> None:
        self.git("switch", "-q", "-c", name)

    def switch(self, name: str) -> None:
        self.git("switch", "-q", name)

    def merge(self, name: str, into: str = "main") -> None:
        self.git("merge", "-q", "--no-ff", "-m", f"Merge branch '{name}' into {into}", name)


def build(path: Path) -> None:
    r = Repo(path)
    r.git("init", "-q", "-b", "main")
    r.commit("Initial commit", author=0)
    r.commit("Add README")
    r.git("tag", "v0.1.0")

    r.branch("parser")
    r.commit("Parse git log records", author=1)
    r.commit("Classify refs")
    r.switch("main")
    r.commit("CI: run tests on push", author=2)
    r.switch("parser")
    r.commit("Parser: handle detached HEAD", author=1)
    r.switch("main")
    r.merge("parser")

    r.branch("lanes")
    r.commit("Lay out lanes", author=0)
    r.switch("main")
    r.branch("hotfix")
    r.commit("Fix a crash on empty repositories", author=2)
    r.switch("main")
    r.merge("hotfix")
    r.switch("lanes")
    r.merge("main", into="lanes")
    r.commit("Reuse freed columns", author=0)
    r.commit("Colour each branch line")
    r.switch("main")
    r.merge("lanes")
    r.git("tag", "v0.2.0")

    r.branch("panel")
    r.commit("History panel", author=1)
    r.switch("main")
    r.branch("docs")
    r.commit("Docs: the history graph", author=2)
    r.switch("panel")
    r.commit("Panel: ref badges")
    r.switch("docs")
    r.commit("Docs: screenshots")
    r.switch("main")
    r.merge("docs")
    r.switch("panel")
    r.commit("Panel: brand colours", author=1)
    r.switch("main")
    r.merge("panel")
    r.git("update-ref", "refs/remotes/origin/main", "HEAD")
    r.commit("Bump version", author=0)
    r.git("tag", "v0.3.0")

    # Work in progress, for the map: pushed, ahead of its remote, behind it, remote only.
    r.branch("search")
    r.commit("Search commits by subject", author=1)
    r.git("update-ref", "refs/remotes/origin/search", "HEAD")
    r.commit("Search: highlight matches")
    r.switch("main")
    r.branch("export")
    r.commit("Export the graph as SVG", author=2)
    r.git("update-ref", "refs/remotes/origin/export", "HEAD")
    r.commit("Export: fit to page")
    r.git("update-ref", "refs/remotes/origin/export", "HEAD")
    r.git("reset", "-q", "--hard", "HEAD~1")
    r.switch("main")
    r.branch("themes")
    r.commit("Dark theme", author=0)
    r.git("update-ref", "refs/remotes/origin/themes", "HEAD")
    r.switch("main")
    r.git("branch", "-q", "-D", "themes")
    r.switch("search")


def sundgren_graph(repo: str, *args: str) -> dict[str, Any]:
    out = subprocess.run(
        [SUNDGREN, "graph", "--repo", repo, "--base", "main", "--target", "search", *args],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    graph: dict[str, Any] = json.loads(out)
    graph["repository"] = "/demo"
    return graph


def demo_graphs() -> dict[str, Any]:
    """Every view: {"graphs": [{"branches": N, "offset": K, "graph": ...}, ...]}."""
    views: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory() as tmp:
        build(Path(tmp))
        for branches in range(1, BRANCHES + 1):
            offset: int | None = 0
            while offset is not None:
                graph = sundgren_graph(
                    tmp,
                    "--columns",
                    str(COLUMNS),
                    "--branches",
                    str(branches),
                    "--offset",
                    str(offset),
                )
                views.append({"branches": branches, "offset": offset, "graph": graph})
                offset = graph["frame"]["older"]
    return {"columns": COLUMNS, "graphs": views}


def write(out: Path) -> None:
    """Write the views to `out`, as a script that sets window.sundgrenDemoGraphs."""
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(PREFIX + json.dumps(demo_graphs(), separators=(",", ":")) + ";\n", "utf-8")


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: demo_graph.py OUT")
    write(Path(sys.argv[1]))


if __name__ == "__main__":
    main()
