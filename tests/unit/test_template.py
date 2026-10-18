from __future__ import annotations

import pytest

from sundgren.run.template import default_python, expand_command, expand_value

VALUES = {"python": "/opt/py bin/python", "out": "/tmp/out", "worktree": "/wt", "lcov": "/tmp/l"}


def test_expand_value_known_placeholders() -> None:
    assert expand_value("{out}/x:{worktree}", VALUES) == "/tmp/out/x:/wt"


def test_expand_value_leaves_unknown_braces() -> None:
    assert expand_value("${HOME}/{foo}/{out}", VALUES) == "${HOME}/{foo}//tmp/out"


def test_expand_value_leaves_known_placeholder_without_value() -> None:
    assert expand_value("{junit}", VALUES) == "{junit}"


def test_expand_value_does_not_reexpand() -> None:
    assert expand_value("{python}", {"python": "{out}", "out": "x"}) == "{out}"


def test_tests_token_without_selection_vanishes() -> None:
    assert expand_command("pytest {tests}", VALUES, [], shell=False) == ["pytest"]


def test_tests_token_one_argv_entry_per_test() -> None:
    argv = expand_command("pytest {tests} -q", VALUES, ["a.py", "b c.py"], shell=False)
    assert argv == ["pytest", "a.py", "b c.py", "-q"]


def test_tests_inside_larger_token_is_space_joined() -> None:
    argv = expand_command("run --only={tests}", VALUES, ["a", "b"], shell=False)
    assert argv == ["run", "--only=a b"]


def test_split_before_expand_keeps_paths_with_spaces_whole() -> None:
    argv = expand_command("{python} -m x --o={out}/f", VALUES, [], shell=False)
    assert argv == ["/opt/py bin/python", "-m", "x", "--o=/tmp/out/f"]


def test_quoted_template_token() -> None:
    argv = expand_command("""echo "a {out}" '${HOME}'""", VALUES, [], shell=False)
    assert argv == ["echo", "a /tmp/out", "${HOME}"]


def test_shell_mode_quotes_tests() -> None:
    cmd = expand_command("pytest {tests} > {out}/log", VALUES, ["a b.py", "c.py"], shell=True)
    assert cmd == "pytest 'a b.py' c.py > /tmp/out/log"


def test_shell_mode_no_tests_and_unknown_braces() -> None:
    assert expand_command("echo ${HOME} {tests}", VALUES, [], shell=True) == "echo ${HOME} "


@pytest.mark.parametrize(
    ("found", "expected"),
    [
        ({"python": "/u/python", "python3": "/u/python3"}, "/u/python"),
        ({"python3": "/u/python3"}, "/u/python3"),
        ({}, "python"),
    ],
)
def test_default_python(
    monkeypatch: pytest.MonkeyPatch, found: dict[str, str], expected: str
) -> None:
    monkeypatch.setattr("shutil.which", found.get)
    assert default_python() == expected
