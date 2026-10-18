"""Run the docs examples and check their recorded outputs, or re-record them with --update."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from sundgren.core.report.json_report import dumps

ROOT = Path(__file__).resolve().parent.parent
EXAMPLES = ROOT / "docs" / "examples"
MANIFEST = "example.toml"
COMMANDS = "commands.sh"
PROJECT = "project"
LANGUAGES = ("python", "rust")
REQUIRE_VAR = "SUNDGREN_DOCS_REQUIRE"
COMMAND_TIMEOUT = 1800

# Volatile report fields are replaced with these, so recordings are byte-stable.
CREATED_UTC = "2026-01-01T00:00:00Z"  # the example commit's date
VERSION = "<version>"
ZERO_SHA = "0" * 64

# Fixed identity and dates keep the example commit's sha stable; user/system config is ignored.
_GIT_ENV = {
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "sundgren-docs",
    "GIT_AUTHOR_EMAIL": "docs@example.invalid",
    "GIT_COMMITTER_NAME": "sundgren-docs",
    "GIT_COMMITTER_EMAIL": "docs@example.invalid",
    "GIT_AUTHOR_DATE": "2026-01-01T00:00:00+0000",
    "GIT_COMMITTER_DATE": "2026-01-01T00:00:00+0000",
}
# Variables that would redirect git, pytest, coverage or imports in the commands.
_LEAKY_PREFIXES = ("GIT_", "PYTEST_", "COVERAGE_", "COV_CORE_")
_LEAKY_NAMES = frozenset({"PYTHONPATH"})
JUNK = ("__pycache__", "*.pyc", ".pytest_cache", ".coverage", "target")
_IGNORE = shutil.ignore_patterns(*JUNK)
_MANIFEST_KEYS = {"language", "patch", "outputs", "requires"}


class ExampleError(Exception):
    """An example is malformed or its commands failed."""


@dataclass(frozen=True)
class Requirement:
    run: str
    expect: str


@dataclass(frozen=True)
class Example:
    path: Path
    language: str
    patch: str
    outputs: tuple[str, ...]
    commands: tuple[str, ...]
    requires: tuple[Requirement, ...]

    @property
    def name(self) -> str:
        return self.path.name


def example_dirs() -> list[Path]:
    """Every directory under docs/examples/, sorted."""
    return sorted(p for p in EXAMPLES.iterdir() if p.is_dir() and p.name not in JUNK)


def required_languages() -> set[str]:
    """Languages in SUNDGREN_DOCS_REQUIRE: their examples must run, not skip."""
    langs = {s.strip() for s in os.environ.get(REQUIRE_VAR, "").split(",") if s.strip()}
    if unknown := langs - set(LANGUAGES):
        raise ExampleError(f"{REQUIRE_VAR}: unknown languages {sorted(unknown)}")
    return langs


def _plain_name(value: Any, what: str) -> str:
    if not isinstance(value, str) or not value or "/" in value or value in (".", ".."):
        raise ExampleError(f"{what} must be a file name, got {value!r}")
    return value


def load(path: Path) -> Example:
    """Read and validate an example directory."""
    where = f"{path.name}/{MANIFEST}"
    try:
        data = tomllib.loads((path / MANIFEST).read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ExampleError(f"{where}: {exc}") from None
    if unknown := sorted(set(data) - _MANIFEST_KEYS):
        raise ExampleError(f"{where}: unknown keys {unknown}")
    if missing_keys := sorted({"language", "patch", "outputs"} - set(data)):
        raise ExampleError(f"{where}: missing keys {missing_keys}")
    language = data["language"]
    if language not in LANGUAGES:
        raise ExampleError(f"{where}: language must be one of {LANGUAGES}, got {language!r}")
    patch = _plain_name(data["patch"], f"{where}: patch")
    outputs = data["outputs"]
    if not isinstance(outputs, list) or not outputs:
        raise ExampleError(f"{where}: outputs must be a non-empty list")
    outputs = tuple(_plain_name(o, f"{where}: outputs entry") for o in outputs)
    if len(set(outputs)) != len(outputs) or {patch, MANIFEST, COMMANDS, PROJECT} & set(outputs):
        raise ExampleError(f"{where}: outputs must be unique and not clash with other files")
    requires = []
    for req in data.get("requires", []):
        if (
            not isinstance(req, dict)
            or set(req) != {"run", "expect"}
            or not all(isinstance(v, str) and v for v in req.values())
        ):
            raise ExampleError(f"{where}: each requires entry is {{run = str, expect = str}}")
        requires.append(Requirement(req["run"], req["expect"]))

    project = path / PROJECT
    if not project.is_dir() or not any(project.iterdir()):
        raise ExampleError(f"{path.name}: {PROJECT}/ is missing or empty")
    if not (path / patch).is_file():
        raise ExampleError(f"{path.name}: patch {patch} is missing")
    for name in (patch, *outputs):
        if (project / name).exists():
            raise ExampleError(f"{path.name}: {PROJECT}/{name} clashes with the example's {name}")
    commands = _commands(path)
    used = {_patch_arg(shlex.split(line)) for line in commands} - {None}
    if used != {patch}:
        raise ExampleError(f"{path.name}: commands must pass `--patch {patch}`, found {used}")
    return Example(path, language, patch, outputs, commands, tuple(requires))


def _patch_arg(argv: list[str]) -> str | None:
    for i, arg in enumerate(argv):
        if arg == "--patch" and i + 1 < len(argv):
            return argv[i + 1]
        if arg.startswith("--patch="):
            return arg.removeprefix("--patch=")
    return None


def _commands(path: Path) -> tuple[str, ...]:
    where = f"{path.name}/{COMMANDS}"
    try:
        text = (path / COMMANDS).read_text(encoding="utf-8")
    except OSError as exc:
        raise ExampleError(f"{where}: {exc}") from None
    lines = tuple(line for line in text.splitlines() if line.strip())
    if not lines:
        raise ExampleError(f"{where}: no commands")
    for line in lines:
        try:
            argv = shlex.split(line)
        except ValueError as exc:
            raise ExampleError(f"{where}: {line!r}: {exc}") from None
        if argv[0] != "sundgren":
            raise ExampleError(f"{where}: {line!r}: every command must start with `sundgren`")
    return lines


def tested_files(example: Example) -> set[Path]:
    """Files the docs tests run or compare, i.e. the ones a page may include."""
    files = {example.path / n for n in (MANIFEST, COMMANDS, example.patch, *example.outputs)}
    project = example.path / PROJECT
    for path in project.rglob("*"):
        parts = path.relative_to(project).parts
        if path.is_file() and not any(_IGNORE("", [part]) for part in parts):
            files.add(path)
    return files


def unexpected_files(example: Example) -> list[str]:
    """Top-level entries of the example directory that aren't part of the format."""
    known = {MANIFEST, COMMANDS, PROJECT, example.patch, *example.outputs}
    return sorted(
        p.name for p in example.path.iterdir() if p.name not in known and not _IGNORE("", [p.name])
    )


def _clean_env(tmp: Path | None = None) -> dict[str, str]:
    env = {
        k: v
        for k, v in os.environ.items()
        if not k.startswith(_LEAKY_PREFIXES) and k not in _LEAKY_NAMES
    }
    # `{python}` is the `python` on PATH, so the interpreter running this comes first.
    env["PATH"] = os.pathsep.join([str(Path(sys.executable).parent), env.get("PATH", "")])
    env["GIT_CONFIG_GLOBAL"] = os.devnull
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    if tmp is not None:
        env["TMPDIR"] = str(tmp)
    return env


def missing(example: Example) -> str | None:
    """Why the example can't run here (a `requires` entry fails), or None."""
    env = _clean_env()
    for req in example.requires:
        try:
            proc = subprocess.run(
                shlex.split(req.run), env=env, capture_output=True, text=True, timeout=300
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            return f"requires `{req.run}`: {exc}"
        if proc.returncode != 0:
            return f"requires `{req.run}`: exit {proc.returncode}: {proc.stderr.strip()[:200]!r}"
        if req.expect not in proc.stdout:
            return f"requires `{req.run}` ~ {req.expect!r}: got {proc.stdout.strip()[:200]!r}"
    return None


def _git(repo: Path, *args: str) -> None:
    env = {**_clean_env(), **_GIT_ENV}
    proc = subprocess.run(["git", *args], cwd=repo, env=env, capture_output=True, text=True)
    if proc.returncode != 0:
        raise ExampleError(f"git {' '.join(args)} failed: {proc.stderr.strip()}")


def _make_repo(example: Example, dest: Path) -> None:
    shutil.copytree(example.path / PROJECT, dest, ignore=_IGNORE)
    _git(dest, "-c", "init.defaultBranch=main", "init", "-q", "--object-format=sha1")
    _git(dest, "add", "-A")
    _git(dest, "-c", "commit.gpgsign=false", "commit", "-q", "-m", example.name)
    shutil.copyfile(example.path / example.patch, dest / example.patch)


def normalize(name: str, text: str, python: str | None) -> str:
    """Replace the volatile fields of a JSON report; other outputs are returned as they are."""
    if not name.endswith(".json"):
        return text
    report = json.loads(text)
    meta = report.get("meta")
    if not isinstance(meta, dict):
        return text
    meta["created_utc"] = CREATED_UTC
    meta["sundgren_version"] = VERSION
    for key, shas in meta["inputs"].items():
        meta["inputs"][key] = [ZERO_SHA] * len(shas)
    if python and meta["commands"]:
        own = re.compile(re.escape(python) + r"(?![\w.-])")
        meta["commands"] = [own.sub("python", c) for c in meta["commands"]]
    return dumps(report)


def produce(example: Example) -> dict[str, str]:
    """Run the example's commands in a fresh repo; return its normalized outputs by name."""
    with tempfile.TemporaryDirectory(prefix="sundgren-docs-") as work_dir:
        work = Path(work_dir)
        repo, tmp = work / "repo", work / "tmp"
        tmp.mkdir()
        _make_repo(example, repo)
        env = _clean_env(tmp)
        python = shutil.which("python", path=env["PATH"]) or shutil.which(
            "python3", path=env["PATH"]
        )
        if example.language == "python" and python != str(Path(sys.executable).parent / "python"):
            raise ExampleError(f"no `python` next to {sys.executable}; run inside a venv")
        for line in example.commands:
            argv = [sys.executable, "-m", "sundgren", *shlex.split(line)[1:]]
            try:
                proc = subprocess.run(
                    argv,
                    cwd=repo,
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=COMMAND_TIMEOUT,
                )
            except subprocess.TimeoutExpired:
                raise ExampleError(f"{example.name}: `{line}` timed out") from None
            if proc.returncode != 0:
                raise ExampleError(
                    f"{example.name}: `{line}` exited {proc.returncode}\n{proc.stderr}"
                )
        outputs = {}
        for name in example.outputs:
            try:
                text = (repo / name).read_bytes().decode("utf-8")
            except OSError:
                raise ExampleError(f"{example.name}: the commands wrote no {name}") from None
            outputs[name] = normalize(name, text, python)
        for name, text in outputs.items():
            for path in {str(work), os.path.realpath(work), str(ROOT)}:
                if path in text:
                    raise ExampleError(f"{example.name}: {name} contains the local path {path}")
    return outputs


def diff(example: Example, produced: dict[str, str]) -> str:
    """Unified diff from the recordings to `produced`; empty when they match."""
    chunks: list[str] = []
    for name, text in produced.items():
        path = example.path / name
        recorded = path.read_bytes().decode("utf-8") if path.is_file() else ""
        if recorded != text:
            chunks.extend(
                difflib.unified_diff(
                    recorded.splitlines(keepends=True),
                    text.splitlines(keepends=True),
                    f"recorded/{example.name}/{name}",
                    f"actual/{example.name}/{name}",
                )
            )
    return "".join(c if c.endswith("\n") else c + "\n" for c in chunks)


def write(example: Example, produced: dict[str, str]) -> list[str]:
    """Write `produced` into the example directory; return the names that changed."""
    changed = []
    for name, text in produced.items():
        path = example.path / name
        if not path.is_file() or path.read_bytes() != text.encode("utf-8"):
            path.write_bytes(text.encode("utf-8"))
            changed.append(name)
    return changed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help="re-record instead of checking")
    parser.add_argument("names", nargs="*", metavar="NAME", help="example names (default: all)")
    args = parser.parse_args(argv)
    try:
        paths = [EXAMPLES / n for n in args.names] or example_dirs()
        examples = [load(p) for p in paths]
    except ExampleError as exc:
        print(exc, file=sys.stderr)
        return 2
    failed = False
    for example in examples:
        if reason := missing(example):
            print(f"{example.name}: skipped, {reason}", file=sys.stderr)
            failed |= bool(args.names)  # a named example must run
            continue
        try:
            produced = produce(example)
        except ExampleError as exc:
            print(exc, file=sys.stderr)
            failed = True
            continue
        if args.update:
            changed = write(example, produced)
            print(f"{example.name}: " + (f"updated {', '.join(changed)}" if changed else "same"))
        elif text := diff(example, produced):
            print(text, end="")
            print(f"{example.name}: differs (re-record: tools/docs_examples.py --update)")
            failed = True
        else:
            print(f"{example.name}: ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
