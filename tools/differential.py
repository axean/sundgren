"""Run a released sundgren and the working tree on the same inputs and compare the output.

Usage: uv run python tools/differential.py [--old VERSION] [-k SUBSTRING] [-j JOBS] [--show N]

The released version runs via `uvx --from sundgren==VERSION`, the working tree's binary from
SUNDGREN_BIN or next to this Python. Both run from the repo root on the fixtures in
tests/fixtures: `ingest`, and `diff` over lcov pairs, patches, JUnit files and output options.
The results must match: the exit code, whether there is an error, and stdout (the JSON report
without `meta.created_utc`, `meta.sundgren_version` and warning messages, or the text report).
Error wording may differ. Cases in KNOWN differ on purpose and are reported separately.
Exit status: 0 when every other case matches, 1 otherwise.
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import subprocess
import sys
from collections.abc import Iterator
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIX = Path("tests/fixtures")
RS_ROOTS = ["/home/runner/work/shed/shed_rs", "/builds/shed/shed_rs"]
OUTPUTS: tuple[list[str], ...] = (
    [],
    ["--full"],
    ["--counts"],
    ["--format", "text"],
    ["--full", "--counts", "--format", "text"],
)

# Case name -> why it differs from 0.1.2.
KNOWN: dict[str, str] = {}

_VOLATILE = re.compile(r'^(\s*"(?:created_utc|sundgren_version)": )".*"(,?)$', re.M)


@dataclass(frozen=True)
class Case:
    name: str
    args: tuple[str, ...]


@dataclass(frozen=True)
class Result:
    code: int
    stdout: str
    stderr: str


def _files(sub: str, pattern: str) -> list[str]:
    return sorted(str(p.relative_to(ROOT)) for p in (ROOT / FIX / sub).rglob(pattern))


def cases() -> Iterator[Case]:
    lcovs = _files("lcov", "*.lcov")
    runs = sorted(p.name for p in (ROOT / FIX / "shed_py_runs").iterdir() if p.is_dir())
    patches = _files("patches", "*.patch")
    junits = _files("junit", "*.xml")

    for lcov in [*lcovs, *_files("shed_py_runs", "*.lcov")]:
        yield Case(f"ingest/{lcov}", ("ingest", lcov))
        yield Case(f"ingest/{lcov}/root", ("ingest", lcov, "--root", RS_ROOTS[0]))

    for run in runs:
        d = FIX / "shed_py_runs" / run
        pair = (str(d / "base.lcov"), str(d / "patched.lcov"))
        junit = ("--base-junit", str(d / "base.xml"), "--patched-junit", str(d / "patched.xml"))
        patch = FIX / "patches" / f"{run}.patch"
        roots = ("--base-root", str(FIX / "shed_py"), "--patched-root", str(FIX / "shed_py"))
        for out in OUTPUTS:
            tag = " ".join(out) or "default"
            yield Case(f"run/{run}/{tag}", ("diff", *pair, "--patch", str(patch), *junit, *out))
        yield Case(f"run/{run}/roots", ("diff", *pair, "--patch", str(patch), *roots, "--full"))
        yield Case(f"run/{run}/no-patch", ("diff", *pair, "--full"))
        for p in patches:
            for out in ([], ["--full"]):
                tag = " ".join(out) or "default"
                yield Case(f"patch/{run}/{p}/{tag}", ("diff", *pair, "--patch", p, *out))
        for j in junits:
            yield Case(f"junit/{run}/{j}", ("diff", *pair, "--base-junit", j, "--patched-junit", j))
            yield Case(f"junit/{run}/{j}/base-only", ("diff", *pair, "--base-junit", j))

    d = FIX / "shed_py_runs" / runs[0]
    pair = (str(d / "base.lcov"), str(d / "patched.lcov"))
    yield Case(
        "junit/unreadable", ("diff", *pair, "--base-junit", lcovs[0], "--patched-junit", lcovs[0])
    )
    yield Case("junit/missing", ("diff", *pair, "--base-junit", "nope.xml"))
    yield Case("error/missing-lcov", ("diff", "nope.lcov", pair[1]))

    for a in lcovs:
        for b in lcovs:
            for out in ([], ["--full", "--counts"]):
                tag = " ".join(out) or "default"
                strip = ("--strip-prefix", *RS_ROOTS)
                yield Case(f"lcov/{Path(a).name}+{Path(b).name}/{tag}", ("diff", a, b, *out))
                yield Case(
                    f"lcov/{Path(a).name}+{Path(b).name}/strip/{tag}", ("diff", a, b, *strip, *out)
                )


def run(cmd: list[str]) -> Result:
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, errors="surrogateescape")
    return Result(proc.returncode, _VOLATILE.sub(r'\1"*"\2', proc.stdout), proc.stderr)


def compare(case: Case, old: list[str], new: list[str]) -> list[str]:
    """Differences between the two runs, as printable lines (empty when identical)."""
    a, b = run([*old, *case.args]), run([*new, *case.args])
    out: list[str] = []
    if a.code != b.code:
        out.append(f"exit code {a.code} != {b.code}")
    if bool(a.stderr) != bool(b.stderr):
        out.append(f"stderr on one side only: {a.stderr!r} vs {b.stderr!r}")
    x, y = _comparable(a.stdout), _comparable(b.stdout)
    if x != y:
        diff = difflib.unified_diff(
            x.splitlines(), y.splitlines(), "old stdout", "new stdout", lineterm="", n=2
        )
        out.extend(diff)
    return out


def _comparable(stdout: str) -> str:
    """A JSON report without its warning messages, re-dumped; any other output as it is."""
    try:
        report = json.loads(stdout)
    except ValueError:
        return stdout
    if isinstance(report, dict):
        for w in report.get("warnings", []):
            w.pop("message", None)
    return json.dumps(report, sort_keys=True, indent=2)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("--old", default="0.1.2", metavar="VERSION", help="released version (0.1.2)")
    ap.add_argument("-k", default="", metavar="SUBSTRING", help="only cases whose name has this")
    ap.add_argument("-j", type=int, default=4, metavar="JOBS", help="parallel cases (4)")
    ap.add_argument("--show", type=int, default=3, metavar="N", help="diffs to print (3)")
    ap.add_argument("--list", action="store_true", help="list the cases and exit")
    args = ap.parse_args(argv)

    selected = [c for c in cases() if args.k in c.name]
    if args.list:
        for c in selected:
            print(f"{c.name}: sundgren {' '.join(c.args)}")
        return 0
    old = ["uvx", "-q", "--from", f"sundgren=={args.old}", "sundgren"]
    new = [os.environ.get("SUNDGREN_BIN") or str(Path(sys.executable).with_name("sundgren"))]
    subprocess.run([*old, "--version"], check=True, capture_output=True)  # install once

    with ThreadPoolExecutor(args.j) as pool:
        results = list(pool.map(lambda c: (c, compare(c, old, new)), selected))
    failed = [(c, d) for c, d in results if d and c.name not in KNOWN]
    known = [(c, d) for c, d in results if d and c.name in KNOWN]
    for c, d in failed[: args.show]:
        print(f"--- {c.name}: sundgren {' '.join(c.args)}")
        print("\n".join(d[:60]))
    for c, _ in known:
        print(f"known difference: {c.name}: {KNOWN[c.name]}")
    for name in KNOWN:
        if any(name == c.name for c, d in results if not d):
            print(f"note: {name} now matches; drop it from KNOWN")
    by_kind: dict[str, list[int]] = {}
    for c, d in results:
        counts = by_kind.setdefault(c.name.split("/")[0], [0, 0])
        counts[0] += 1
        counts[1] += bool(d) and c.name not in KNOWN
    summary = ", ".join(f"{k} {n - bad}/{n}" for k, (n, bad) in sorted(by_kind.items()))
    print(f"{len(results) - len(failed) - len(known)}/{len(results)} identical ({summary})")
    print(f"{len(failed)} differ, {len(known)} known differences")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
