"""Command line: diff, run, presets, ingest."""

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence

import sundgren.lang.python  # noqa: F401  (lang packs register themselves on import)
import sundgren.lang.rust  # noqa: F401
from sundgren import __version__
from sundgren._native import cli as _impl

EXIT_OK, EXIT_FAILURE, EXIT_USAGE, EXIT_PATCH = (
    _impl.EXIT_OK,
    _impl.EXIT_FAILURE,
    _impl.EXIT_USAGE,
    _impl.EXIT_PATCH,
)


def _add_output_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("--format", choices=("json", "text"), default="json")
    p.add_argument("-o", "--output", metavar="OUT", help="write the report here instead of stdout")
    p.add_argument("--full", action="store_true", help="include unchanged lines, functions, tests")
    p.add_argument("--counts", action="store_true", help="fill files[].count_changed")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="sundgren", description=__doc__)
    parser.add_argument("--version", action="version", version=f"sundgren {__version__}")
    sub = parser.add_subparsers(dest="command")

    d = sub.add_parser("diff", help="diff two existing lcov files")
    d.add_argument("base_lcov", metavar="BASE.lcov")
    d.add_argument("patched_lcov", metavar="PATCHED.lcov")
    d.add_argument("--patch", metavar="FILE", help="patch between the two trees, for line mapping")
    d.add_argument("--base-root", metavar="DIR")
    d.add_argument("--patched-root", metavar="DIR")
    d.add_argument("--strip-prefix", metavar="PREFIX", action="extend", nargs="+", default=[])
    d.add_argument("--base-junit", metavar="FILE")
    d.add_argument("--patched-junit", metavar="FILE")
    _add_output_args(d)

    r = sub.add_parser("run", help="run coverage on base and patched trees, then diff")
    r.add_argument("--patch", metavar="FILE", required=True)
    how = r.add_mutually_exclusive_group()
    how.add_argument("--preset", metavar="NAME")
    how.add_argument("--cmd", metavar="CMD", action="append", help="ad-hoc command (repeatable)")
    r.add_argument("--lcov", metavar="TEMPLATE", help="lcov output path template (with --cmd)")
    r.add_argument("--junit", metavar="TEMPLATE", help="JUnit output path template (with --cmd)")
    r.add_argument("--shell", action="store_true", help="run --cmd via /bin/sh -c")
    r.add_argument("--repo", metavar="DIR", default=".")
    r.add_argument("--base", metavar="REF", default="HEAD")
    r.add_argument("--tests", metavar="T", action="extend", nargs="+", default=[])
    r.add_argument("--python", metavar="PATH")
    r.add_argument("--env", metavar="K=V", action="append", default=[])
    r.add_argument("--repeat", metavar="N", type=int, default=1)
    r.add_argument("--workdir", metavar="DIR")
    r.add_argument("--keep-worktrees", action="store_true")
    _add_output_args(r)

    pr = sub.add_parser("presets", help="list built-in and user presets")
    pr.add_argument("--repo", metavar="DIR", default=".")

    i = sub.add_parser("ingest", help="debug: print a normalized lcov Snapshot as JSON")
    i.add_argument("lcov", metavar="FILE.lcov")
    i.add_argument("--root", metavar="DIR")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """Parse with argparse; the Rust handlers do the rest."""
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help(sys.stderr)
        return EXIT_USAGE
    return _impl.run_command(args)
