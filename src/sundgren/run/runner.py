"""TemplateRunner: run a preset's commands in a worktree, N repeats."""

from __future__ import annotations

import os
import re
import shlex
import shutil
import subprocess
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import IO

from sundgren.run.presets import Preset
from sundgren.run.selectors import Selection
from sundgren.run.template import expand_command, expand_value

LOG_TAIL_LINES = 40

_PLAIN_TOKEN = re.compile(r"[\w@%+=:,./{}-]+")


@dataclass(frozen=True, slots=True)
class RunArtifacts:
    lcov: Path
    junit: Path | None
    returncode: int  # first non-zero return code, else 0; test failures are data
    log: Path


class RunFailure(Exception):
    """lcov output missing after the commands ran."""

    def __init__(self, message: str, log_tail: str) -> None:
        super().__init__(message)
        self.log_tail = log_tail


def adhoc_preset(commands: tuple[str, ...], lcov: str, junit: str | None, *, shell: bool) -> Preset:
    return Preset(
        name="adhoc",
        description="--cmd",
        commands=commands,
        env={},
        lcov=lcov,
        junit=junit,
        selector=None,
        tests_style="args",
        shell=shell,
        source="--cmd",
    )


class TemplateRunner:
    def __init__(
        self, preset: Preset, *, python: str, env_overrides: Mapping[str, str] | None = None
    ) -> None:
        self.preset = preset
        self.python = python
        self.env_overrides = dict(env_overrides or {})

    def display_commands(self, selection: Selection) -> list[str]:
        """Commands with only {python} and {tests} expanded (deterministic, for meta)."""
        values = {"python": self.python}
        tests = _tests(selection)
        out = []
        for command in self.preset.commands:
            expanded = expand_command(command, values, tests, shell=self.preset.shell)
            out.append(expanded if isinstance(expanded, str) else _join(expanded))
        return out

    def run(
        self, worktree: Path, selection: Selection, out_dir: Path, repeat: int = 1
    ) -> list[RunArtifacts]:
        if repeat < 1:
            raise ValueError(f"repeat must be >= 1, got {repeat}")
        worktree = worktree.absolute()
        return [
            self._run_once(worktree, selection, out_dir.absolute() / f"run-{i}")
            for i in range(1, repeat + 1)
        ]

    def _run_once(self, worktree: Path, selection: Selection, run_dir: Path) -> RunArtifacts:
        run_dir.mkdir(parents=True, exist_ok=True)
        tests = _tests(selection)
        values = {"python": self.python, "out": str(run_dir), "worktree": str(worktree)}
        values["lcov"] = expand_value(self.preset.lcov, values)
        values["junit"] = expand_value(self.preset.junit, values) if self.preset.junit else ""
        values["tests"] = " ".join(tests)
        env = dict(os.environ)
        for name, template in (*self.preset.env.items(), *self.env_overrides.items()):
            env[name] = expand_value(template, values)

        # Relative output paths are relative to the commands' cwd.
        lcov = worktree / values["lcov"]
        junit = worktree / values["junit"] if values["junit"] else None
        for stale in (lcov, junit):  # left over from an earlier repeat or a reused --workdir
            if stale is not None and stale.is_file():
                stale.unlink()

        log = run_dir / "log.txt"
        returncode = 0
        with log.open("wb") as fh:
            for command in self.preset.commands:
                argv = expand_command(command, values, tests, shell=self.preset.shell)
                shown = argv if isinstance(argv, str) else shlex.join(argv)
                fh.write(f"$ {shown}\n".encode())
                fh.flush()
                code = _call(argv, worktree, env, fh)
                fh.write(f"[exit {code}]\n".encode())
                fh.flush()
                if returncode == 0:
                    returncode = code

        if not lcov.is_file():
            raise RunFailure(f"{run_dir.name}: no lcov output at {lcov}", _tail(log))
        lcov = _keep_in(lcov, run_dir)
        junit = _keep_in(junit, run_dir) if junit is not None and junit.is_file() else None
        return RunArtifacts(lcov=lcov, junit=junit, returncode=returncode, log=log)


def _tests(selection: Selection) -> tuple[str, ...]:
    return () if selection.run_all else selection.tests


def _join(argv: list[str]) -> str:
    return " ".join(t if _PLAIN_TOKEN.fullmatch(t) else shlex.quote(t) for t in argv)


def _call(argv: list[str] | str, cwd: Path, env: dict[str, str], fh: IO[bytes]) -> int:
    """Run one command with output into `fh`; a command that can't start is logged as 126/127."""
    if not argv:
        fh.write(b"sundgren: empty command\n")
        return 127
    try:
        proc = subprocess.run(
            argv,
            cwd=cwd,
            env=env,
            shell=isinstance(argv, str),
            stdin=subprocess.DEVNULL,
            stdout=fh,
            stderr=subprocess.STDOUT,
            check=False,
        )
    except OSError as exc:
        fh.write(f"sundgren: cannot run command: {exc}\n".encode())
        return 127 if isinstance(exc, FileNotFoundError) else 126
    return proc.returncode


def _keep_in(path: Path, run_dir: Path) -> Path:
    """Copy an output written outside the run dir into it, so repeats don't overwrite it."""
    if path.is_relative_to(run_dir):
        return path
    target = run_dir / path.name
    shutil.copyfile(path, target)
    return target


def _tail(log: Path) -> str:
    lines = log.read_text(encoding="utf-8", errors="replace").splitlines()
    return "\n".join(lines[-LOG_TAIL_LINES:])
