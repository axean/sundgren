"""The docs environments on the gh-pages branch: the live site, the preview of main, and one
preview per open pull request. The site's Development menu reads environments.json.

Usage, in a gh-pages checkout (tools/gh-pages.sh runs both):
  environments.py write DEST   write DEST/environment.json from SUNDGREN_ENV_* variables
  environments.py index        write environments.json for every environment there
"""

from __future__ import annotations

import json
import os
import re
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

PR = re.compile(r"pr-preview/pr-(\d+)")
# Variables the workflows set for `write`: what the environment shows.
FIELDS = {
    "title": "SUNDGREN_ENV_TITLE",
    "ref": "SUNDGREN_ENV_REF",
    "sha": "SUNDGREN_ENV_SHA",
    "link": "SUNDGREN_ENV_LINK",
}


def describe(dest: str) -> dict[str, Any]:
    """The kind of environment at DEST: "." (live), "main" or "pr-preview/pr-N"."""
    dest = dest.strip("/")
    if dest in ("", "."):
        return {"path": "", "kind": "release"}
    if dest == "main":
        return {"path": "main/", "kind": "main"}
    match = PR.fullmatch(dest)
    if match is None:
        raise ValueError(f"not a docs environment: {dest}")
    return {"path": f"{dest}/", "kind": "pr", "number": int(match[1])}


def write(dest: str, environ: dict[str, str], now: datetime) -> dict[str, Any]:
    entry = describe(dest)
    for field, variable in FIELDS.items():
        if environ.get(variable):
            entry[field] = environ[variable]
    entry["updated"] = now.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    path = Path(dest) / "environment.json"
    path.write_text(json.dumps(entry, indent=1) + "\n", "utf-8")
    return entry


def _order(entry: dict[str, Any]) -> tuple[int, int]:
    rank = {"release": 0, "main": 1, "pr": 2}[entry["kind"]]
    return rank, -entry.get("number", 0)


def index(root: Path) -> list[dict[str, Any]]:
    """Every environment under ROOT that has a site (an index.html), live site first."""
    found: list[dict[str, Any]] = []
    candidates = [root, root / "main", *sorted((root / "pr-preview").glob("pr-*"))]
    for directory in candidates:
        if not (directory / "index.html").is_file():
            continue
        dest = directory.relative_to(root).as_posix()
        try:
            entry = describe(dest)
        except ValueError:
            continue
        saved = directory / "environment.json"
        if saved.is_file():
            entry = {**json.loads(saved.read_text("utf-8")), **entry}
        found.append(entry)
    return sorted(found, key=_order)


def main(argv: list[str]) -> int:
    if argv[:1] == ["write"] and len(argv) == 2:
        write(argv[1], dict(os.environ), datetime.now(UTC))
    elif argv == ["index"]:
        out = {"environments": index(Path("."))}
        Path("environments.json").write_text(json.dumps(out, indent=1) + "\n", "utf-8")
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
