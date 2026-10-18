"""Command templates: placeholder expansion, tokenwise {tests}."""

from __future__ import annotations

import re
import shlex
import shutil
from collections.abc import Mapping, Sequence

PLACEHOLDERS = ("python", "out", "worktree", "lcov", "junit", "tests")

_PLACEHOLDER_RE = re.compile(r"\{(" + "|".join(PLACEHOLDERS) + r")\}")


def expand_value(template: str, values: Mapping[str, str]) -> str:
    """Plain substitution of known placeholders; unknown braces are left alone."""
    # One pass, so a value that itself contains "{out}" is not expanded again.
    return _PLACEHOLDER_RE.sub(lambda m: values.get(m[1], m[0]), template)


def expand_command(
    command: str, values: Mapping[str, str], tests: Sequence[str], *, shell: bool
) -> list[str] | str:
    """argv list (shell=False) or a /bin/sh -c string (shell=True)."""
    if shell:
        return expand_value(command, {**values, "tests": " ".join(map(shlex.quote, tests))})
    inline = {**values, "tests": " ".join(tests)}
    argv: list[str] = []
    for token in shlex.split(command):
        if token == "{tests}":
            argv.extend(tests)
        else:
            argv.append(expand_value(token, inline))
    return argv


def default_python() -> str:
    """`python` on the user's PATH (falls back to python3), not sys.executable."""
    return shutil.which("python") or shutil.which("python3") or "python"
