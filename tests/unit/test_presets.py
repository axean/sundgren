from __future__ import annotations

from importlib.resources import files
from pathlib import Path

import pytest

from sundgren.run import presets
from sundgren.run.presets import (
    PresetError,
    all_presets,
    builtin_presets,
    register_builtin_presets,
    user_presets,
)

LANG = Path(presets.__file__).resolve().parents[1] / "lang"
PY_TOML = LANG / "python" / "presets.toml"
RS_TOML = LANG / "rust" / "presets.toml"

MINIMAL = 'commands = ["run"]\nlcov = "{out}/x.lcov"\n'


@pytest.fixture(autouse=True)
def fresh_registry(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(presets, "_BUILTINS", {})


def test_python_builtin() -> None:
    register_builtin_presets(PY_TOML)
    p = builtin_presets()["pytest"]
    assert p.commands[1] == "{python} -m coverage lcov --data-file={out}/.coverage -o {lcov}"
    assert p.env == {"COVERAGE_FILE": "{out}/.coverage", "PYTHONHASHSEED": "0"}
    assert (p.lcov, p.junit, p.selector) == (
        "{out}/coverage.lcov",
        "{out}/junit.xml",
        "python.pytest_files",
    )
    assert p.tests_style == "args"
    assert not p.shell
    assert len(p.hints) == 2
    assert p.source == "built-in"


def test_rust_builtins_have_no_junit_or_selector() -> None:
    register_builtin_presets(RS_TOML)
    got = builtin_presets()
    assert list(got) == ["cargo-llvm-cov", "cargo-tarpaulin"]
    assert all(p.junit is None and p.selector is None for p in got.values())
    assert all(p.tests_style == "filter" for p in got.values())


def test_register_is_idempotent() -> None:
    register_builtin_presets(PY_TOML)
    # files("sundgren.lang.python") would import the pack and register into the fake registry.
    register_builtin_presets(files("sundgren.lang") / "python" / "presets.toml")
    assert list(builtin_presets()) == ["pytest"]


def test_conflicting_builtin_rejected(tmp_path: Path) -> None:
    register_builtin_presets(PY_TOML)
    other = tmp_path / "presets.toml"
    other.write_text("[presets.pytest]\n" + MINIMAL)
    with pytest.raises(PresetError, match="'pytest'"):
        register_builtin_presets(other)
    assert builtin_presets()["pytest"].selector == "python.pytest_files"


def test_no_user_files(tmp_path: Path) -> None:
    assert user_presets(tmp_path) == {}


def test_user_presets_and_precedence(tmp_path: Path) -> None:
    (tmp_path / "pyproject.toml").write_text(
        "[project]\nname = 'x'\n"
        "[tool.sundgren.presets.a]\n" + MINIMAL + "description = 'from pyproject'\n"
        "[tool.sundgren.presets.b]\n" + MINIMAL
    )
    (tmp_path / "sundgren.toml").write_text(
        "[presets.a]\n" + MINIMAL + "description = 'from sundgren.toml'\n"
    )
    got = user_presets(tmp_path)
    assert list(got) == ["a", "b"]
    assert (got["a"].description, got["a"].source) == ("from sundgren.toml", "sundgren.toml")
    assert got["b"].source == "pyproject.toml"


def test_pyproject_without_sundgren_table(tmp_path: Path) -> None:
    (tmp_path / "pyproject.toml").write_text("[tool.ruff]\nline-length = 100\n")
    assert user_presets(tmp_path) == {}


def test_defaults_for_optional_keys(tmp_path: Path) -> None:
    (tmp_path / "sundgren.toml").write_text("[presets.mine]\n" + MINIMAL)
    p = user_presets(tmp_path)["mine"]
    assert (p.description, p.env, p.junit, p.selector) == ("", {}, None, None)
    assert (p.tests_style, p.shell, p.hints) == ("args", False, ())


def test_user_overrides_builtin(tmp_path: Path) -> None:
    register_builtin_presets(PY_TOML)
    register_builtin_presets(RS_TOML)
    (tmp_path / "sundgren.toml").write_text("[presets.pytest]\n" + MINIMAL + "shell = true\n")
    got = all_presets(tmp_path)
    assert list(got) == ["cargo-llvm-cov", "cargo-tarpaulin", "pytest"]
    assert got["pytest"].shell
    assert got["pytest"].source == "sundgren.toml"
    assert builtin_presets()["pytest"].source == "built-in"


@pytest.mark.parametrize(
    ("body", "problem"),
    [
        ('lcov = "x"\n', "missing required key 'commands'"),
        ('commands = ["x"]\n', "missing required key 'lcov'"),
        ('commands = []\nlcov = "x"\n', "'commands' must not be empty"),
        ('commands = "pytest"\nlcov = "x"\n', "'commands' must be a list of strings"),
        ('commands = ["x", 1]\nlcov = "x"\n', "'commands' must be a list of strings"),
        ('commands = ["x"]\nlcov = ""\n', "'lcov' must be a non-empty string"),
        (MINIMAL + "junit = 3\n", "'junit' must be a string"),
        (MINIMAL + "env = { A = 1 }\n", "'env' must be a table of strings"),
        (MINIMAL + "tests_style = 'names'\n", "'tests_style' must be 'args' or 'filter'"),
        (MINIMAL + "shell = 'yes'\n", "'shell' must be a boolean"),
        (MINIMAL + "hints = 'one'\n", "'hints' must be a list of strings"),
        (MINIMAL + "command = ['typo']\n", "unknown key(s) command"),
    ],
)
def test_validation_names_file_and_preset(tmp_path: Path, body: str, problem: str) -> None:
    path = tmp_path / "sundgren.toml"
    path.write_text("[presets.bad]\n" + body)
    with pytest.raises(PresetError) as info:
        user_presets(tmp_path)
    assert str(info.value) == f"{path}: preset 'bad': {problem}"


def test_preset_not_a_table(tmp_path: Path) -> None:
    (tmp_path / "sundgren.toml").write_text("[presets]\nbad = 1\n")
    with pytest.raises(PresetError, match="preset 'bad': must be a table"):
        user_presets(tmp_path)


def test_invalid_toml_names_file(tmp_path: Path) -> None:
    path = tmp_path / "pyproject.toml"
    path.write_text("[tool.sundgren\n")
    with pytest.raises(PresetError, match=str(path)):
        user_presets(tmp_path)
