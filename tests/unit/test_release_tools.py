from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

import release

ROOT = Path(__file__).resolve().parents[2]
URL = "https://github.com/axean/sundgren"

GOOD = f"""# Changelog

Intro.

## Unreleased

## [0.2.0] - 2026-11-01

> [!NOTE]
> A note ([#3]).

### Added

- Thing,
  wrapped ([#3]).

#### Not a subsection, ignored

### Fixed

- Bug.

## [0.1.0] - 2026-10-18

### Added

- First.

[#3]: {URL}/pull/3
[0.1.0]: {URL}/releases/tag/v0.1.0
[0.2.0]: {URL}/releases/tag/v0.2.0
"""


def _write(root: Path, version: str = '"0.2.0"', changelog: str = GOOD) -> Path:
    (root / "pyproject.toml").write_text(f'[project]\nname = "x"\nversion = {version}\n')
    (root / "CHANGELOG.md").write_text(changelog)
    return root


def test_good_changelog_lints_clean() -> None:
    assert release.lint(GOOD) == []


@pytest.mark.parametrize(
    ("old", "new", "error"),
    [
        ("# Changelog", "# Change log", "line 1: must be '# Changelog'"),
        ("## Unreleased\n", "", "first '## ' section must be '## Unreleased'"),
        ("## Unreleased", "## [Unreleased]", "expected '## [X.Y.Z] - YYYY-MM-DD'"),
        ("## [0.2.0] - 2026-11-01", "## [0.2.0]", "expected '## [X.Y.Z] - YYYY-MM-DD'"),
        ("## [0.2.0] - 2026-11-01", "## [0.2] - 2026-11-01", "version '0.2' is not X.Y.Z"),
        ("## [0.2.0] - 2026-11-01", "## [0.2.0-rc1] - 2026-11-01", "is not X.Y.Z"),
        ("## [0.2.0] - 2026-11-01", "## [0.2.0] - 2026-13-01", "date '2026-13-01'"),
        ("## [0.2.0] - 2026-11-01", "## [0.2.0] - 20261101", "date '20261101'"),
        ("## [0.2.0] - 2026-11-01", "## [0.0.9] - 2026-11-01", "0.1.0 is not lower than 0.0.9"),
        ("## [0.2.0] - 2026-11-01", "## [0.1.0] - 2026-11-01", "0.1.0 is not lower than 0.1.0"),
        ("### Fixed", "### Improvements", "'### Improvements' is not one of ### Added"),
        ("### Fixed", "## Fixed", "expected '## [X.Y.Z] - YYYY-MM-DD', got '## Fixed'"),
        ("### Fixed", "# Fixed", "'# Fixed' is not one of"),
        ("### Fixed", "### Added", "duplicate '### Added'"),
        (f"[0.2.0]: {URL}/releases/tag/v0.2.0\n", "", "no link definition '[0.2.0]: "),
        ("tag/v0.2.0", "tag/v0.1.0", "[0.2.0] should link to"),
        (f"[#3]: {URL}/pull/3\n", "", "no link definition '[#3]: "),
        ("/pull/3", "/pull/4", "[#3] should link to"),
        ("- First.\n", "", "release 0.1.0 has no entries"),
        ("- Bug.\n", "", "'### Fixed' is empty"),
    ],
)
def test_lint_errors(old: str, new: str, error: str) -> None:
    assert old in GOOD
    errors = release.lint(GOOD.replace(old, new, 1))
    assert any(error in e for e in errors), errors


def test_lint_duplicate_link_definition() -> None:
    errors = release.lint(GOOD + f"[#3]: {URL}/pull/3\n")
    assert any("duplicate link definition [#3]" in e for e in errors), errors


def test_lint_ignores_fenced_headings() -> None:
    text = GOOD.replace("- Bug.", "- Bug.\n\n  ```sh\n# a comment\n## not a section\n  ```")
    assert release.lint(text) == []


def test_notes_is_section_body_with_used_links() -> None:
    assert release.notes(GOOD, "0.2.0") == (
        "> [!NOTE]\n> A note ([#3]).\n\n### Added\n\n- Thing, wrapped ([#3]).\n\n"
        "#### Not a subsection, ignored\n\n### Fixed\n\n- Bug.\n\n"
        f"[#3]: {URL}/pull/3\n"
    )
    assert release.notes(GOOD, "0.1.0") == "### Added\n\n- First.\n"


def test_unwrap_joins_soft_wrapped_lines_only() -> None:
    lines = [
        "> [!WARNING]",
        "> Quote",
        "> wraps.",
        ">",
        "> Second",
        "> paragraph.",
        "",
        "### Added",
        "",
        "- Item",
        "  wraps.",
        "  - nested",
        "    wraps.",
        "1. numbered",
        "   wraps.",
        "",
        "Plain",
        "wraps.",
        "```sh",
        "kept",
        "  as is",
        "```",
        "| a |",
        "|---|",
    ]
    assert release.unwrap(lines) == [
        "> [!WARNING]",
        "> Quote wraps.",
        ">",
        "> Second paragraph.",
        "",
        "### Added",
        "",
        "- Item wraps.",
        "  - nested wraps.",
        "1. numbered wraps.",
        "",
        "Plain wraps.",
        "```sh",
        "kept",
        "  as is",
        "```",
        "| a |",
        "|---|",
    ]


def test_notes_unknown_version() -> None:
    with pytest.raises(KeyError):
        release.notes(GOOD, "0.3.0")


def test_check_ok(tmp_path: Path) -> None:
    assert release.check("v0.2.0", _write(tmp_path)) == []


@pytest.mark.parametrize(
    "tag",
    ["0.2.0", "v0.2", "v0.2.0.0", "v02.0.0", "v0.2.0-rc1", "v0.2.0+local", "V0.2.0", "v0.2.0 "],
)
def test_check_rejects_non_strict_tag(tmp_path: Path, tag: str) -> None:
    assert release.check(tag, _write(tmp_path)) == [f"tag '{tag}' is not vX.Y.Z"]


def test_check_tag_must_match_pyproject(tmp_path: Path) -> None:
    errors = release.check("v0.2.0", _write(tmp_path, version='"0.2.1"'))
    assert errors == ["tag v0.2.0 does not match pyproject.toml version 0.2.1"]


def test_check_cargo_must_match_pyproject(tmp_path: Path) -> None:
    root = _write(tmp_path)
    (root / "Cargo.toml").write_text('[package]\nname = "x"\nversion = "0.1.9"\n')
    errors = release.check("v0.2.0", root)
    assert errors == ["Cargo.toml version 0.1.9 does not match pyproject.toml 0.2.0"]


def test_check_extension_must_match_tag(tmp_path: Path) -> None:
    root = _write(tmp_path)
    package_json = root / "editors" / "vscode" / "package.json"
    package_json.parent.mkdir(parents=True)
    package_json.write_text('{"name": "sundgren", "version": "0.2.0"}\n')
    assert release.check("v0.2.0", root) == []
    package_json.write_text('{"name": "sundgren", "version": "0.1.9"}\n')
    errors = release.check("v0.2.0", root)
    assert errors == ["tag v0.2.0 does not match editors/vscode/package.json version 0.1.9"]


def test_check_needs_static_version(tmp_path: Path) -> None:
    root = _write(tmp_path)
    (root / "pyproject.toml").write_text('[project]\nname = "x"\ndynamic = ["version"]\n')
    assert release.check("v0.2.0", root) == ["pyproject.toml has no static [project] version"]


def test_check_needs_changelog_section(tmp_path: Path) -> None:
    errors = release.check("v0.3.0", _write(tmp_path, version='"0.3.0"'))
    assert errors == ["CHANGELOG.md has no '## [0.3.0] - YYYY-MM-DD' section"]


def test_check_needs_empty_unreleased(tmp_path: Path) -> None:
    root = _write(tmp_path, changelog=GOOD.replace("## Unreleased\n", "## Unreleased\n\n- Wip.\n"))
    errors = release.check("v0.2.0", root)
    assert errors == ["'## Unreleased' must be empty when releasing; move it to [0.2.0]"]


def test_check_includes_lint(tmp_path: Path) -> None:
    root = _write(tmp_path, changelog=GOOD.replace("### Fixed", "### Bugs"))
    assert any("'### Bugs'" in e for e in release.check("v0.2.0", root))


def test_cli(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    root = _write(tmp_path)
    assert release.main(["--root", str(root), "check", "v0.2.0"]) == 0
    assert release.main(["--root", str(root), "check", "v0.1.0"]) == 1
    assert "does not match" in capsys.readouterr().err
    assert release.main(["--root", str(root), "notes", "v0.1.0"]) == 0
    assert capsys.readouterr().out == "### Added\n\n- First.\n"
    assert release.main(["--root", str(root), "notes", "0.9.0"]) == 1


def test_repo_changelog() -> None:
    """The repo's own changelog lints clean."""
    out = subprocess.run(
        [sys.executable, "-I", str(ROOT / "tools" / "release.py"), "lint"],
        capture_output=True,
        text=True,
    )
    assert out.returncode == 0, out.stderr


def test_strip_unreleased() -> None:
    text = GOOD.replace("## Unreleased\n", "## Unreleased\n\n### Added\n\n- Wip.\n")
    out = release.strip_unreleased(text)
    assert "Unreleased" not in out and "Wip." not in out
    assert out.startswith("# Changelog\n\nIntro.\n\n## [0.2.0] - 2026-11-01\n")
    assert release.notes(out, "0.2.0") == release.notes(text, "0.2.0")
    assert release.strip_unreleased(out) == out


def test_strip_unreleased_cli(tmp_path: Path) -> None:
    root = _write(tmp_path)
    assert release.main(["--root", str(root), "strip-unreleased"]) == 0
    assert "## Unreleased" not in (root / "CHANGELOG.md").read_text()
