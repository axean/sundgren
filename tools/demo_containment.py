"""Writes docs/_static/demo/containment-data.js: `sundgren contain` of each branch of a made-up
repository against its dev branch, for the containment demo.

Usage: uv run tools/demo_containment.py [OUT]. Stable output: fixed identity and dates.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from demo_graph import SUNDGREN, Repo

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "_static" / "demo" / "containment-data.js"
# containment-data.js is this line, then the JSON.
PREFIX = "window.sundgrenDemoContainment =\n"
BASE, TARGET = "main", "dev"
# The same made-up license header on every module: boilerplate that every branch shares.
HEADER = (
    "# Copyright 2026 Example Lab\n"
    "# Licensed under the Apache License, Version 2.0\n\n"
    '"""{}."""\n\n'
)


def module(name: str, functions: int) -> str:
    """A small made-up module: a header, then `functions` functions with names of its own."""
    body = "".join(
        f"def {name}_step_{i}({name}_values, scale={i}):\n"
        f"    {name}_total = sum(v * scale for v in {name}_values if v > {i})\n"
        f"    return round({name}_total / max(len({name}_values), 1), {i % 4})\n\n"
        for i in range(functions)
    )
    return HEADER.format(name.capitalize()) + body


class FileRepo(Repo):
    def write(self, path: str, text: str, message: str, author: int | None = None) -> None:
        file = self.path / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, "utf-8")
        self.git("add", "--", path)
        self.commit(message, author)


def build(path: Path) -> None:
    r = FileRepo(path)
    r.git("init", "-q", "-b", BASE)
    r.write("app/core.py", module("core", 4), "Start")

    # The dev branch: three features.
    r.branch(TARGET)
    r.write("app/fitting.py", module("fitting", 8), "Fitting", author=1)
    r.write("app/plots.py", module("plots", 6), "Plots")
    r.write("app/export.py", module("export", 5), "Export", author=2)

    # Extracted from dev as is, and extracted with review edits.
    r.switch(BASE)
    r.branch("fitting-extract")
    r.write("app/fitting.py", module("fitting", 8), "Fitting, extracted from dev", author=1)
    r.switch(BASE)
    r.branch("plots-reviewed")
    reviewed = module("plots", 6).replace("v * scale", "v*scale").replace("), 1)", "), 1) ")
    r.write("app/plots.py", reviewed, "Plots, extracted and reviewed", author=0)

    # A copy of dev rebased onto a newer main: the same work.
    r.switch(BASE)
    r.write("README.md", "# Example\n\nSee app/.\n", "README", author=2)
    r.git("switch", "-q", "-c", "dev-rebased", TARGET)
    r.git("rebase", "-q", BASE)

    # Half of export plus new work; and something unrelated.
    r.switch(BASE)
    r.branch("export-and-more")
    r.write("app/export.py", module("export", 2) + module("csv", 3), "Export to CSV", author=1)
    r.switch(BASE)
    r.branch("logging")
    r.write("app/log.py", module("log", 5), "Logging", author=2)

    # Other work going on at the same time, unrelated to dev but with the same license header.
    for i, name in enumerate(UNRELATED):
        r.switch(BASE)
        r.branch(name)
        r.write(
            f"app/{name.replace('-', '_')}.py",
            module(name.replace("-", "_"), 3 + i % 4),
            name.replace("-", " ").capitalize(),
            author=i % 3,
        )

    # Already merged into main: nothing ahead.
    r.switch(BASE)
    r.branch("readme-typo")
    r.switch(BASE)


def compare(repo: Path, branch: str) -> dict[str, Any]:
    out = subprocess.run(
        [SUNDGREN, "contain", "--repo", str(repo), "--base", BASE, "--", branch, TARGET],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    result: dict[str, Any] = json.loads(out)
    result["repository"] = "/demo"
    return result


UNRELATED = ["metrics", "config-loader", "cli-flags", "cache", "retry-policy", "units"]
BRANCHES = [
    "fitting-extract",
    "plots-reviewed",
    "dev-rebased",
    "export-and-more",
    "logging",
    *UNRELATED,
    "readme-typo",
]


def demo_containment() -> dict[str, Any]:
    with tempfile.TemporaryDirectory() as tmp:
        build(Path(tmp))
        results = [{"branch": b, "result": compare(Path(tmp), b)} for b in BRANCHES]
    return {"target": TARGET, "base": BASE, "results": results}


def main() -> None:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else OUT
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(PREFIX + json.dumps(demo_containment(), indent=1) + ";\n", "utf-8")


def read(path: Path = OUT) -> dict[str, Any]:
    """The demo data in a containment-data.js."""
    text = path.read_text("utf-8").removeprefix(PREFIX).removesuffix(";\n")
    data: dict[str, Any] = json.loads(text)
    return data


if __name__ == "__main__":
    main()
