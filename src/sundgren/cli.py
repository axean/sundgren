"""Command line: diff, run, presets, ingest."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Mapping, Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import sundgren.lang.python  # noqa: F401  (lang packs register themselves on import)
import sundgren.lang.rust  # noqa: F401
from sundgren import __version__
from sundgren.core.diff import diff
from sundgren.core.hashing import sha256_file
from sundgren.core.ingest.lcov import parse as parse_lcov
from sundgren.core.junit import parse_junit
from sundgren.core.linemap import LineMap, build_linemaps
from sundgren.core.model import Outcome, ReportWarning, Snapshot, snapshot_to_dict
from sundgren.core.normalize import normalize_snapshot
from sundgren.core.patch import Patch, parse_patch
from sundgren.core.report.json_report import build_report, dumps
from sundgren.core.report.text_report import render_text
from sundgren.run.guards import wrong_tree_paths
from sundgren.run.presets import Preset, PresetError, all_presets
from sundgren.run.runner import RunArtifacts, RunFailure, TemplateRunner, adhoc_preset
from sundgren.run.selectors import Selection, choose, restrict_to_tree
from sundgren.run.template import default_python
from sundgren.run.workspace import (
    BadRefError,
    GitError,
    PatchApplyError,
    Workspace,
    is_dirty,
    repo_root,
)

EXIT_OK, EXIT_FAILURE, EXIT_USAGE, EXIT_PATCH = 0, 1, 2, 3

PYTHON_MARKERS = ("pyproject.toml", "setup.cfg", "pytest.ini")
RUST_MARKERS = ("Cargo.toml",)


class CliError(Exception):
    def __init__(self, message: str, code: int) -> None:
        super().__init__(message)
        self.code = code


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
    parser = build_parser()
    args = parser.parse_args(argv)
    handlers = {"diff": _cmd_diff, "run": _cmd_run, "presets": _cmd_presets, "ingest": _cmd_ingest}
    if args.command is None:
        parser.print_help(sys.stderr)
        return EXIT_USAGE
    try:
        return handlers[args.command](args)
    except CliError as exc:
        print(f"sundgren: {exc}", file=sys.stderr)
        return exc.code


# --- shared helpers ---------------------------------------------------------


def _read_text(path: Path, what: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise CliError(f"cannot read {what} {path}: {exc.strerror}", EXIT_USAGE) from None


def _load_snapshot(path: Path, roots: Sequence[str]) -> Snapshot:
    try:
        raw = parse_lcov(_read_text(path, "lcov"))
    except ValueError as exc:
        raise CliError(f"{path}: {exc}", EXIT_FAILURE) from None
    return normalize_snapshot(raw, roots)


def _load_junit(path: Path | None, warnings: list[ReportWarning]) -> dict[str, Outcome] | None:
    if path is None:
        return None
    try:
        return parse_junit(_read_text(path, "JUnit file"))
    except ValueError as exc:
        warnings.append(ReportWarning("junit_unreadable", f"{path.name}: {exc}"))
        return None


def _load_patch(path: Path) -> tuple[Patch, dict[str, LineMap], list[ReportWarning]]:
    try:
        # Patches are bytes: keep CRs and undecodable path bytes as they are.
        text = path.read_bytes().decode("utf-8", "surrogateescape")
    except OSError as exc:
        raise CliError(f"cannot read patch {path}: {exc.strerror}", EXIT_USAGE) from None
    return _parse_patch(text, str(path))


def _parse_patch(text: str, where: str) -> tuple[Patch, dict[str, LineMap], list[ReportWarning]]:
    try:
        patch = parse_patch(text)
    except ValueError as exc:
        raise CliError(f"{where}: {exc}", EXIT_FAILURE) from None
    warnings = [ReportWarning("patch", w) for w in patch.warnings]
    return patch, build_linemaps(patch), warnings


def _patch_meta(arg: str, path: Path, patch: Patch) -> dict[str, Any]:
    return {"path": arg, "sha256": sha256_file(path), "files": list(patch.touched_paths())}


def _meta(**fields: Any) -> dict[str, Any]:
    meta: dict[str, Any] = {
        "sundgren_version": __version__,
        "created_utc": datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "mode": None,
        "base_ref": None,
        "patch": None,
        "preset": None,
        "commands": None,
        "selection": None,
        "repeat": None,
        "inputs": None,
    }
    meta.update(fields)
    return meta


def _emit(report: Mapping[str, Any], args: argparse.Namespace, hints: Sequence[str] = ()) -> None:
    text = dumps(report) if args.format == "json" else render_text(report, hints)
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)


# --- diff -------------------------------------------------------------------


def _cmd_diff(args: argparse.Namespace) -> int:
    base_root = Path(args.base_root).resolve() if args.base_root else None
    patched_root = Path(args.patched_root).resolve() if args.patched_root else None
    base_path, patched_path = Path(args.base_lcov), Path(args.patched_lcov)
    base = _load_snapshot(base_path, [*_roots(base_root), *args.strip_prefix])
    patched = _load_snapshot(patched_path, [*_roots(patched_root), *args.strip_prefix])

    warnings: list[ReportWarning] = []
    linemaps: dict[str, LineMap] = {}
    patch_meta = None
    if args.patch:
        patch, linemaps, patch_warnings = _load_patch(Path(args.patch))
        warnings += patch_warnings
        patch_meta = _patch_meta(args.patch, Path(args.patch), patch)
    base_junit = _load_junit(Path(args.base_junit) if args.base_junit else None, warnings)
    patched_junit = _load_junit(Path(args.patched_junit) if args.patched_junit else None, warnings)

    delta = diff(
        [base],
        [patched],
        linemaps,
        base_root=base_root,
        patched_root=patched_root,
        base_outcomes=base_junit,
        patched_outcomes=patched_junit,
    )
    meta = _meta(
        mode="diff",
        patch=patch_meta,
        inputs={
            "base_lcov_sha256": [sha256_file(base_path)],
            "patched_lcov_sha256": [sha256_file(patched_path)],
        },
    )
    report = build_report(delta, meta, full=args.full, counts=args.counts, extra_warnings=warnings)
    _emit(report, args)
    return EXIT_OK


def _roots(root: Path | None) -> list[str]:
    return [str(root)] if root is not None else []


# --- run --------------------------------------------------------------------


def _cmd_run(args: argparse.Namespace) -> int:
    if args.repeat < 1:
        raise CliError("--repeat must be >= 1", EXIT_USAGE)
    if not args.cmd and (args.lcov or args.junit or args.shell):
        raise CliError("--lcov, --junit and --shell only apply with --cmd", EXIT_USAGE)
    patch_path = Path(args.patch).resolve()
    if not patch_path.is_file():
        raise CliError(f"patch not found: {args.patch}", EXIT_USAGE)
    try:
        repo = repo_root(Path(args.repo))
    except (GitError, OSError) as exc:
        raise CliError(f"not a git repository: {args.repo} ({exc})", EXIT_USAGE) from None

    preset = _resolve_preset(args, repo)
    env = _parse_env(args.env)
    python = args.python or default_python()
    warnings: list[ReportWarning] = []
    runner = TemplateRunner(preset, python=python, env_overrides=env)
    workdir = Path(args.workdir).resolve() if args.workdir else None
    try:
        if is_dirty(repo):
            warnings.append(
                ReportWarning(
                    "dirty_tree", "working tree has uncommitted changes; the base is the ref"
                )
            )
        with Workspace(
            repo, args.base, patch_path, workdir=workdir, keep=args.keep_worktrees
        ) as ws:
            # Use what git applied, not the user's hunk headers or path prefixes.
            patch, linemaps, patch_warnings = _parse_patch(ws.applied_patch, "applied patch")
            warnings += patch_warnings
            if ws.offset_paths:
                warnings.append(
                    ReportWarning(
                        "patch_offset",
                        f"patch applied at an offset in {', '.join(ws.offset_paths)}; "
                        "line maps use the applied diff",
                    )
                )
            try:
                selection = choose(tuple(args.tests) or None, preset.selector, patch, ws.patched)
            except KeyError as exc:
                raise CliError(str(exc.args[0]), EXIT_FAILURE) from None
            out = ws.workdir / "out"
            base_snaps, base_junit, base_sha = _run_side(
                "base",
                runner,
                preset,
                selection,
                ws.base,
                out / "base",
                args.repeat,
                repo,
                warnings,
            )
            patched_snaps, patched_junit, patched_sha = _run_side(
                "patched",
                runner,
                preset,
                selection,
                ws.patched,
                out / "patched",
                args.repeat,
                repo,
                warnings,
            )
            delta = diff(
                base_snaps,
                patched_snaps,
                linemaps,
                base_root=ws.base,
                patched_root=ws.patched,
                base_outcomes=base_junit,
                patched_outcomes=patched_junit,
            )
            base_ref = ws.base_sha
            patched_selection = restrict_to_tree(selection, ws.patched) or selection
    except PatchApplyError as exc:
        raise CliError(f"patch does not apply:\n{exc}", EXIT_PATCH) from None
    except BadRefError as exc:
        raise CliError(str(exc), EXIT_USAGE) from None
    except GitError as exc:
        raise CliError(f"git: {exc}", EXIT_FAILURE) from None
    except RunFailure as exc:
        raise CliError(f"{exc}\n--- log tail ---\n{exc.log_tail}", EXIT_FAILURE) from None

    meta = _meta(
        mode="run",
        base_ref=base_ref,
        patch=_patch_meta(args.patch, patch_path, patch),
        preset=preset.name,
        commands=runner.display_commands(patched_selection),
        selection={
            "tests": list(patched_selection.tests),
            "run_all": patched_selection.run_all,
            "reason": patched_selection.reason,
        },
        repeat=args.repeat,
        inputs={"base_lcov_sha256": base_sha, "patched_lcov_sha256": patched_sha},
    )
    report = build_report(delta, meta, full=args.full, counts=args.counts, extra_warnings=warnings)
    _emit(report, args, preset.hints)
    return EXIT_OK


def _run_side(
    side: str,
    runner: TemplateRunner,
    preset: Preset,
    selection: Selection,
    worktree: Path,
    out: Path,
    repeat: int,
    repo: Path,
    warnings: list[ReportWarning],
) -> tuple[list[Snapshot], dict[str, Outcome] | None, list[str]]:
    """Run one side; returns its snapshots, first-run outcomes and lcov hashes."""
    side_selection: Selection | None = selection
    if preset.tests_style == "args":
        side_selection = restrict_to_tree(selection, worktree)
    if side_selection is None:
        # Every selected test is new in the patch: nothing ran on this side.
        warnings.append(
            ReportWarning(
                "nothing_to_run",
                f"no selected tests exist on the {side} side; its coverage is treated as empty",
            )
        )
        return [Snapshot(files={}, test_names=())], {}, []

    runs: list[RunArtifacts] = runner.run(worktree, side_selection, out, repeat)
    snapshots: list[Snapshot] = []
    hashes: list[str] = []
    for art in runs:
        try:
            raw = parse_lcov(art.lcov.read_text(encoding="utf-8"))
        except ValueError as exc:
            raise CliError(f"{side}: bad lcov output: {exc}", EXIT_FAILURE) from None
        wrong = wrong_tree_paths((fc.path for fc in raw.files.values()), worktree, repo)
        if wrong:
            raise CliError(_wrong_tree_message(side, wrong, preset), EXIT_FAILURE)
        snapshots.append(normalize_snapshot(raw, [str(worktree)]))
        hashes.append(sha256_file(art.lcov))
    junit = _load_junit(runs[0].junit, warnings)
    return snapshots, junit, hashes


def _wrong_tree_message(side: str, paths: Sequence[str], preset: Preset) -> str:
    lines = [
        f"the {side} run measured your main checkout instead of its worktree "
        "(editable install or stale build?):",
        *(f"  {p}" for p in paths[:20]),
    ]
    if len(paths) > 20:
        lines.append(f"  ... and {len(paths) - 20} more")
    if preset.hints:
        lines.append("hints:")
        lines.extend(f"  - {h}" for h in preset.hints)
    return "\n".join(lines)


def _resolve_preset(args: argparse.Namespace, repo: Path) -> Preset:
    if args.cmd:
        if not args.lcov:
            raise CliError("--cmd needs --lcov", EXIT_USAGE)
        return adhoc_preset(tuple(args.cmd), args.lcov, args.junit, shell=args.shell)
    try:
        presets = all_presets(repo)
    except PresetError as exc:
        raise CliError(str(exc), EXIT_USAGE) from None
    name = args.preset or _default_preset_name(repo)
    if name not in presets:
        known = ", ".join(sorted(presets))
        raise CliError(f"unknown preset {name!r} (known: {known})", EXIT_USAGE)
    return presets[name]


def _default_preset_name(repo: Path) -> str:
    python = any((repo / m).exists() for m in PYTHON_MARKERS)
    rust = any((repo / m).exists() for m in RUST_MARKERS)
    if python and not rust:
        return "pytest"
    if rust and not python:
        return "cargo-llvm-cov"
    raise CliError("can't pick a default preset here; pass --preset or --cmd", EXIT_USAGE)


def _parse_env(items: Sequence[str]) -> dict[str, str]:
    env: dict[str, str] = {}
    for item in items:
        key, sep, value = item.partition("=")
        if not sep or not key:
            raise CliError(f"--env expects K=V, got {item!r}", EXIT_USAGE)
        env[key] = value
    return env


# --- presets / ingest -------------------------------------------------------


def _cmd_presets(args: argparse.Namespace) -> int:
    try:
        presets = all_presets(Path(args.repo))
    except PresetError as exc:
        raise CliError(str(exc), EXIT_USAGE) from None
    for name in sorted(presets):
        p = presets[name]
        print(f"{name}  [{p.source}]  {p.description}")
        print(f"    tests: {p.tests_style}, selector: {p.selector or '-'}")
        for cmd in p.commands:
            print(f"    $ {cmd}")
    return EXIT_OK


def _cmd_ingest(args: argparse.Namespace) -> int:
    roots = _roots(Path(args.root).resolve() if args.root else None)
    snapshot = _load_snapshot(Path(args.lcov), roots)
    sys.stdout.write(json.dumps(snapshot_to_dict(snapshot), indent=2, sort_keys=True) + "\n")
    return EXIT_OK
