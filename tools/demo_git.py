"""What the extension reads from git about a demo repository, recorded by the docs build.

The docs demos can't run git. They get this output instead and parse it with the extension's own
parsers (editors/vscode/src/git-output.ts), so the commands here are the extension's
(editors/vscode/src/git.ts). Paths are shown as `paths` maps them, not as the temporary ones.
"""

from __future__ import annotations

import subprocess
from typing import Any

STATE_FORMAT = (
    "--format=%(refname)%00%(upstream:short)%00%(upstream:track,nobracket)%00%(worktreepath)"
)


def output(repo: str, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", repo, *args], check=True, capture_output=True, text=True
    ).stdout


def shown(text: str, paths: dict[str, str]) -> str:
    """`text` with each real path in `paths` replaced by how the demo shows it."""
    for real, display in sorted(paths.items(), key=lambda item: -len(item[0])):
        text = text.replace(real, display)
    return text


def recorded(
    repo: str,
    paths: dict[str, str],
    oids: list[str] | None = None,
    base: str | None = None,
) -> dict[str, Any]:
    """The branches' states and their work trees' changes; the stats of `oids`; the branches
    merged into `base`."""
    states = output(repo, "for-each-ref", STATE_FORMAT, "refs/heads", "refs/remotes")
    worktrees = [line.split("\0")[3] for line in states.splitlines() if line.split("\0")[3]]
    data: dict[str, Any] = {
        "states": shown(states, paths),
        "status": {shown(w, paths): output(w, "status", "--porcelain") for w in worktrees},
    }
    if oids:
        data["numstat"] = output(
            repo,
            "log",
            "--no-walk=unsorted",
            "--format=%x1e%H",
            "--raw",
            "--numstat",
            "--diff-merges=first-parent",
            *oids,
        )
    if base is not None:
        data["merged"] = output(
            repo,
            "for-each-ref",
            "--format=%(refname)",
            f"--merged={base}",
            "refs/heads",
            "refs/remotes",
        )
    return data
