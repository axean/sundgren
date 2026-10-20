"""Where would the tests import the checkout's packages from? Collects (doesn't run) the tests
with pytest, then resolves each top-level package of the checkout and looks at what collection
imported. Prints one `SUNDGREN-PROBE <json>` line: the [name, path] pairs that come from `repo`
but not from `worktree`. argv: worktree, repo, pytest args. Runs on any Python 3."""

from __future__ import annotations

import importlib.util
import json
import os
import sys
from typing import Any

MARKER = "SUNDGREN-PROBE "


def inside(path: str, root: str) -> bool:
    return path == root or path.startswith(root + os.sep)


def package_names(worktree: str) -> list[str]:
    """Packages at the top level and in src/, plus modules in src/."""
    names: set[str] = set()
    for base in (worktree, os.path.join(worktree, "src")):
        try:
            entries = os.listdir(base)
        except OSError:
            continue
        for entry in entries:
            path = os.path.join(base, entry)
            if entry.isidentifier() and os.path.isfile(os.path.join(path, "__init__.py")):
                names.add(entry)
            elif base != worktree and entry.endswith(".py") and entry[:-3].isidentifier():
                names.add(entry[:-3])
    return sorted(names)


class Probe:
    def __init__(self, worktree: str, repo: str) -> None:
        self.worktree = worktree
        self.repo = repo
        self.wrong: dict[str, str] = {}

    def check(self, name: str, origin: Any) -> None:
        if not isinstance(origin, str) or not os.path.isabs(origin):
            return
        path = os.path.realpath(origin)
        if not inside(path, self.repo) or inside(path, self.worktree):
            return
        # Only the checkout's own files: not a virtualenv or build output inside the repo.
        if os.path.exists(os.path.join(self.worktree, os.path.relpath(path, self.repo))):
            self.wrong.setdefault(name, path)

    def pytest_collection_finish(self, session: Any) -> None:
        # sys.path as the tests see it: pytest's pythonpath and rootdir insertions are in place.
        for name in package_names(self.worktree):
            try:
                spec = importlib.util.find_spec(name)
            except Exception:
                continue
            if spec is not None:
                self.check(name, spec.origin)
        for name, module in list(sys.modules.items()):
            self.check(name.partition(".")[0], getattr(module, "__file__", None))


def main() -> None:
    worktree, repo = (os.path.realpath(p) for p in sys.argv[1:3])
    probe = Probe(worktree, repo)
    import pytest

    pytest.main(["--collect-only", "-q", "-p", "no:cacheprovider", *sys.argv[3:]], plugins=[probe])
    print(MARKER + json.dumps(sorted(probe.wrong.items())))


main()
