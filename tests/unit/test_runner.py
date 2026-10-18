from __future__ import annotations

import shlex
import sys
from dataclasses import replace
from pathlib import Path
from typing import Any

import pytest

from sundgren.run.presets import Preset
from sundgren.run.runner import RunFailure, TemplateRunner, adhoc_preset
from sundgren.run.selectors import Selection

NONE = Selection(tests=(), run_all=True, reason="all")

WRITE_ARGS = "import sys; open(sys.argv[1], 'w').write(' '.join(sys.argv[2:]))"


def py(code: str, *args: str) -> str:
    """Command template running `code` under {python}; code must not contain braces."""
    return " ".join(["{python}", "-c", shlex.quote(code), *args])


BASE = Preset(
    name="t",
    description="",
    commands=(),
    env={},
    lcov="{out}/cov.lcov",
    junit=None,
    selector=None,
    tests_style="args",
)


def preset(*commands: str, **kw: Any) -> Preset:
    return replace(BASE, commands=commands, **kw)


def runner(p: Preset, env_overrides: dict[str, str] | None = None) -> TemplateRunner:
    return TemplateRunner(p, python=sys.executable, env_overrides=env_overrides)


@pytest.fixture
def worktree(tmp_path: Path) -> Path:
    path = tmp_path / "wt"
    path.mkdir()
    return path


def test_success(worktree: Path, tmp_path: Path) -> None:
    p = preset(
        py("print('hello from cmd')"),
        py(WRITE_ARGS, "{lcov}", "TN:"),
        py(WRITE_ARGS, "{junit}", "<testsuite/>"),
        junit="{out}/junit.xml",
    )
    [art] = runner(p).run(worktree, NONE, tmp_path / "out")
    run_dir = tmp_path / "out" / "run-1"
    assert art.lcov == run_dir / "cov.lcov"
    assert art.lcov.read_text() == "TN:"
    assert art.junit == run_dir / "junit.xml"
    assert art.returncode == 0
    assert art.log == run_dir / "log.txt"
    log = art.log.read_text()
    assert log.count("$ ") == 3
    assert "hello from cmd" in log


def test_junit_none_when_not_written(worktree: Path, tmp_path: Path) -> None:
    p = preset(py(WRITE_ARGS, "{lcov}"), junit="{out}/junit.xml")
    [art] = runner(p).run(worktree, NONE, tmp_path / "out")
    assert art.junit is None


def test_nonzero_exit_is_data(worktree: Path, tmp_path: Path) -> None:
    p = preset(
        py("import sys; print('first'); sys.exit(3)"),
        py(WRITE_ARGS, "{lcov}"),
        py("import sys; print('third'); sys.exit(5)"),
    )
    [art] = runner(p).run(worktree, NONE, tmp_path / "out")
    assert art.returncode == 3
    log = art.log.read_text()
    assert "first" in log
    assert "third" in log
    assert "[exit 5]" in log


def test_missing_lcov_raises_with_tail(worktree: Path, tmp_path: Path) -> None:
    p = preset(py("import sys; [print('line', i) for i in range(100)]; sys.exit(1)"))
    with pytest.raises(RunFailure, match="no lcov output") as info:
        runner(p).run(worktree, NONE, tmp_path / "out")
    tail = info.value.log_tail.splitlines()
    assert len(tail) == 40
    assert tail[-1] == "[exit 1]"
    assert tail[-2] == "line 99"


def test_missing_executable_is_127(worktree: Path, tmp_path: Path) -> None:
    p = preset("no-such-program-sundgren --x", py(WRITE_ARGS, "{lcov}"))
    [art] = runner(p).run(worktree, NONE, tmp_path / "out")
    assert art.returncode == 127
    assert "cannot run command" in art.log.read_text()


def test_repeats_get_separate_dirs(worktree: Path, tmp_path: Path) -> None:
    p = preset(py(WRITE_ARGS, "{lcov}", "{out}"))
    arts = runner(p).run(worktree, NONE, tmp_path / "out", repeat=3)
    for i, art in enumerate(arts, start=1):
        run_dir = tmp_path / "out" / f"run-{i}"
        assert art.lcov == run_dir / "cov.lcov"
        assert art.lcov.read_text() == str(run_dir)
        assert art.log.parent == run_dir


def test_relative_lcov_is_in_worktree_and_copied_per_repeat(worktree: Path, tmp_path: Path) -> None:
    p = preset(py(WRITE_ARGS, "{lcov}", "{out}"), lcov="cov.lcov")
    arts = runner(p).run(worktree, NONE, tmp_path / "out", repeat=2)
    assert [a.lcov.read_text() for a in arts] == [str(a.lcov.parent) for a in arts]
    assert arts[0].lcov != arts[1].lcov


def test_stale_lcov_does_not_count(worktree: Path, tmp_path: Path) -> None:
    (worktree / "cov.lcov").write_text("stale")
    p = preset(py("pass"), lcov="cov.lcov")
    with pytest.raises(RunFailure):
        runner(p).run(worktree, NONE, tmp_path / "out")


def test_env_templating_and_overrides(
    worktree: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("SUNDGREN_INHERITED", "yes")
    code = (
        "import os, sys; e = os.environ; open(sys.argv[1], 'w').write("
        "'|'.join([e['A'], e['B'], e['C'], e['D'], e['SUNDGREN_INHERITED'], os.getcwd()]))"
    )
    p = preset(py(code, "{lcov}"), env={"A": "{out}/a", "B": "preset", "C": "${NOPE}"})
    r = runner(p, env_overrides={"B": "{worktree}/b", "D": "{tests}"})
    [art] = r.run(worktree, Selection(("t1", "t2"), False, "x"), tmp_path / "out")
    run_dir = tmp_path / "out" / "run-1"
    assert art.lcov.read_text().split("|") == [
        f"{run_dir}/a",
        f"{worktree}/b",
        "${NOPE}",
        "t1 t2",
        "yes",
        str(worktree),
    ]


def test_tests_become_argv_entries(worktree: Path, tmp_path: Path) -> None:
    p = preset(
        py("import sys, json; open('args.json', 'w').write(json.dumps(sys.argv[1:]))", "{tests}"),
        py(WRITE_ARGS, "{lcov}"),
    )
    r = runner(p)
    r.run(worktree, Selection(("a b.py", "c.py"), False, "x"), tmp_path / "out")
    assert (worktree / "args.json").read_text() == '["a b.py", "c.py"]'
    r.run(worktree, Selection(("ignored.py",), True, "x"), tmp_path / "out")
    assert (worktree / "args.json").read_text() == "[]"


def test_shell_mode(worktree: Path, tmp_path: Path) -> None:
    p = preset("printf '%s\\n' {tests} > {lcov} && echo done", shell=True)
    [art] = runner(p).run(worktree, Selection(("a b.py", "c.py"), False, "x"), tmp_path / "out")
    assert art.lcov.read_text() == "a b.py\nc.py\n"
    assert "done" in art.log.read_text()


def test_display_commands_expand_only_python_and_tests() -> None:
    p = preset(
        "{python} -m pytest {tests} --junitxml={junit}",
        "{python} -m coverage lcov -o {lcov}",
    )
    r = TemplateRunner(p, python="/usr/bin/python3")
    assert r.display_commands(Selection(("t/test_a.py", "t/x y.py"), False, "x")) == [
        "/usr/bin/python3 -m pytest t/test_a.py 't/x y.py' --junitxml={junit}",
        "/usr/bin/python3 -m coverage lcov -o {lcov}",
    ]
    assert r.display_commands(NONE)[0] == "/usr/bin/python3 -m pytest --junitxml={junit}"


def test_display_commands_shell() -> None:
    p = preset("{python} -m pytest {tests} > {out}/log", shell=True)
    r = TemplateRunner(p, python="py")
    assert r.display_commands(Selection(("a b",), False, "x")) == ["py -m pytest 'a b' > {out}/log"]


def test_adhoc_preset() -> None:
    p = adhoc_preset(("make cov",), "{out}/l.info", None, shell=True)
    assert (p.name, p.description, p.selector, p.tests_style) == ("adhoc", "--cmd", None, "args")
    assert (p.commands, p.lcov, p.junit, p.shell, p.env) == (
        ("make cov",),
        "{out}/l.info",
        None,
        True,
        {},
    )


def test_repeat_must_be_positive(worktree: Path, tmp_path: Path) -> None:
    with pytest.raises(ValueError):
        runner(preset("true")).run(worktree, NONE, tmp_path / "out", repeat=0)
