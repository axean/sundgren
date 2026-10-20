"""Release checks: tag vs pyproject version, CHANGELOG.md structure, release notes."""

from __future__ import annotations

import argparse
import datetime
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPO_URL = "https://github.com/axean/sundgren"
SUBSECTIONS = ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security")

_NUM = r"(?:0|[1-9]\d*)"
VERSION_RE = re.compile(rf"{_NUM}\.{_NUM}\.{_NUM}")
TAG_RE = re.compile(rf"v({VERSION_RE.pattern})")
RELEASE_RE = re.compile(r"## \[([^\]]*)\] - (\S+)")
LINK_DEF_RE = re.compile(r"\[([^\]]+)\]: (\S+)")
PR_REF_RE = re.compile(r"\[#(\d+)\](?![(:\[])")
BLOCK_START_RE = re.compile(r"[-*+] |\d+[.)] |[#>|]|```")
ALERT_RE = re.compile(r"> ?\[![A-Z]+\]\s*")


class Section:
    """One `## ` section: heading, sub-headings and body (without link definitions)."""

    def __init__(self, title: str, line: int) -> None:
        self.title = title
        self.line = line
        m = RELEASE_RE.fullmatch("## " + title)
        self.version: str | None = m[1] if m else None
        self.date: str | None = m[2] if m else None
        self.headings: list[tuple[int, str]] = []
        self.body: list[str] = []


class Changelog:
    """CHANGELOG.md split into `## ` sections and link definitions."""

    def __init__(self, text: str) -> None:
        self.lines = text.splitlines()
        self.sections: list[Section] = []
        self.links: dict[str, tuple[str, int]] = {}
        self.errors: list[str] = []
        fenced = False
        for no, line in enumerate(self.lines, 1):
            if line.lstrip().startswith("```"):
                fenced = not fenced
            if not fenced and (m := LINK_DEF_RE.fullmatch(line)):
                if m[1] in self.links:
                    self.errors.append(f"line {no}: duplicate link definition [{m[1]}]")
                self.links[m[1]] = (m[2], no)
                continue
            if not fenced and line.startswith("## "):
                self.sections.append(Section(line[3:], no))
                continue
            if not self.sections:
                continue
            if not fenced and line.startswith("#") and not line.startswith("####"):
                self.sections[-1].headings.append((no, line))
            self.sections[-1].body.append(line)

    def section(self, version: str) -> Section | None:
        """The `## [version]` section, if any."""
        return next((s for s in self.sections if s.version == version), None)


def _version_key(version: str) -> tuple[int, ...]:
    return tuple(int(p) for p in version.split("."))


def _valid_date(date: str) -> bool:
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", date):
        return False
    try:
        datetime.date.fromisoformat(date)
    except ValueError:
        return False
    return True


def _strip_blank(lines: list[str]) -> list[str]:
    start, end = 0, len(lines)
    while start < end and not lines[start].strip():
        start += 1
    while end > start and not lines[end - 1].strip():
        end -= 1
    return lines[start:end]


def _subsection_lines(lines: list[str], heading: int) -> list[str]:
    """Non-blank lines after the 1-based `heading` line, up to the next heading or link def."""
    out: list[str] = []
    for line in lines[heading:]:
        if line.startswith("#") or LINK_DEF_RE.fullmatch(line):
            break
        if line.strip():
            out.append(line)
    return out


def lint(text: str) -> list[str]:
    """Structural problems in a changelog, one message per problem."""
    log = Changelog(text)
    errors = list(log.errors)
    if not log.lines or log.lines[0] != "# Changelog":
        errors.append("line 1: must be '# Changelog'")
    if not log.sections or log.sections[0].title != "Unreleased":
        errors.append("the first '## ' section must be '## Unreleased'")
    previous: str | None = None
    for i, sec in enumerate(log.sections):
        where = f"line {sec.line}"
        if sec.version is None or sec.date is None:
            if not (i == 0 and sec.title == "Unreleased"):
                errors.append(f"{where}: expected '## [X.Y.Z] - YYYY-MM-DD', got '## {sec.title}'")
        elif not VERSION_RE.fullmatch(sec.version):
            errors.append(f"{where}: version '{sec.version}' is not X.Y.Z")
        else:
            if not _valid_date(sec.date):
                errors.append(f"{where}: date '{sec.date}' is not YYYY-MM-DD")
            if previous is not None and _version_key(sec.version) >= _version_key(previous):
                errors.append(f"{where}: {sec.version} is not lower than {previous} above it")
            previous = sec.version
            url = f"{REPO_URL}/releases/tag/v{sec.version}"
            got = log.links.get(sec.version)
            if got is None:
                errors.append(f"{where}: no link definition '[{sec.version}]: {url}'")
            elif got[0] != url:
                errors.append(f"line {got[1]}: [{sec.version}] should link to {url}")
            if not any(line.strip() for line in sec.body if not line.startswith("#")):
                errors.append(f"{where}: release {sec.version} has no entries")
        seen: set[str] = set()
        for no, line in sec.headings:
            name = line.removeprefix("### ")
            if name == line or name not in SUBSECTIONS:
                errors.append(f"line {no}: '{line}' is not one of ### {', '.join(SUBSECTIONS)}")
            elif name in seen:
                errors.append(f"line {no}: duplicate '{line}'")
            elif not _subsection_lines(log.lines, no):
                errors.append(f"line {no}: '{line}' is empty")
            seen.add(name)
    for no, line in enumerate(log.lines, 1):
        if LINK_DEF_RE.fullmatch(line):
            continue
        for pr in PR_REF_RE.findall(line):
            url = f"{REPO_URL}/pull/{pr}"
            got = log.links.get(f"#{pr}")
            if got is None:
                errors.append(f"line {no}: no link definition '[#{pr}]: {url}'")
            elif got[0] != url:
                errors.append(f"line {got[1]}: [#{pr}] should link to {url}")
    return errors


def unwrap(lines: list[str]) -> list[str]:
    """Join hard-wrapped lines; GitHub release notes render every newline as a line break."""
    out: list[str] = []
    fenced = False
    for line in lines:
        if line.lstrip().startswith("```"):
            fenced = not fenced
            out.append(line)
            continue
        prev = out[-1] if out else ""
        quote = line.startswith(">")
        content = line[1:].strip() if quote else line.strip()
        if (
            fenced
            or not content
            or not (prev[1:] if prev.startswith(">") else prev).strip()
            or prev.lstrip().startswith(("#", "|", "```"))
            or BLOCK_START_RE.match(content)
            or prev.startswith(">") != quote
            or ALERT_RE.fullmatch(prev)
        ):
            out.append(line)
        else:
            out[-1] = f"{prev} {content}"
    return out


def notes(text: str, version: str) -> str:
    """Body of the version's section, unwrapped, plus the link definitions it uses."""
    log = Changelog(text)
    sec = log.section(version)
    if sec is None:
        raise KeyError(f"CHANGELOG.md has no '## [{version}]' section")
    body = "\n".join(unwrap(_strip_blank(sec.body)))
    used = [f"[{k}]: {url}" for k, (url, _) in log.links.items() if f"[{k}]" in body]
    return body + ("\n\n" + "\n".join(used) if used else "") + "\n"


def strip_unreleased(text: str) -> str:
    """The changelog without its `## Unreleased` section, for the docs of a release."""
    log = Changelog(text)
    unreleased = next((s for s in log.sections if s.title == "Unreleased"), None)
    if unreleased is None:
        return text
    following = [s.line for s in log.sections if s.line > unreleased.line]
    end = following[0] - 1 if following else len(log.lines)
    lines = log.lines[: unreleased.line - 1] + log.lines[end:]
    return "\n".join(lines) + "\n"


def project_version(root: Path) -> str:
    """`[project] version` from pyproject.toml."""
    with (root / "pyproject.toml").open("rb") as f:
        version = tomllib.load(f).get("project", {}).get("version")
    if not isinstance(version, str):
        raise ValueError("pyproject.toml has no static [project] version")
    return version


def check(tag: str, root: Path) -> list[str]:
    """Problems that should stop releasing `tag` from the tree at `root`."""
    m = TAG_RE.fullmatch(tag)
    if not m:
        return [f"tag '{tag}' is not vX.Y.Z"]
    version = m[1]
    errors: list[str] = []
    try:
        if (pyproject := project_version(root)) != version:
            errors.append(f"tag {tag} does not match pyproject.toml version {pyproject}")
        cargo_toml = root / "Cargo.toml"
        if cargo_toml.is_file():
            with cargo_toml.open("rb") as f:
                package: dict[str, object] = tomllib.load(f).get("package", {})
                cargo = package.get("version")
            if cargo != pyproject:
                errors.append(
                    f"Cargo.toml version {cargo} does not match pyproject.toml {pyproject}"
                )
    except ValueError as e:
        errors.append(str(e))
    path = root / "CHANGELOG.md"
    if not path.is_file():
        return [*errors, f"{path} not found"]
    text = path.read_text(encoding="utf-8")
    errors += lint(text)
    log = Changelog(text)
    if log.section(version) is None:
        errors.append(f"CHANGELOG.md has no '## [{version}] - YYYY-MM-DD' section")
    unreleased = log.sections[0] if log.sections else None
    if unreleased and unreleased.title == "Unreleased" and _strip_blank(unreleased.body):
        errors.append(f"'## Unreleased' must be empty when releasing; move it to [{version}]")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repo root (default: %(default)s)")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("lint", help="check CHANGELOG.md structure")
    p = sub.add_parser("check", help="check a release tag against pyproject.toml and CHANGELOG.md")
    p.add_argument("tag", help="vX.Y.Z")
    p = sub.add_parser("notes", help="print the CHANGELOG.md section of a version")
    p.add_argument("version", help="X.Y.Z or vX.Y.Z")
    sub.add_parser("strip-unreleased", help="remove the Unreleased section from CHANGELOG.md")
    args = parser.parse_args(argv)
    changelog = args.root / "CHANGELOG.md"
    if args.command == "strip-unreleased":
        changelog.write_text(strip_unreleased(changelog.read_text(encoding="utf-8")), "utf-8")
        return 0
    if args.command == "notes":
        try:
            text = notes(changelog.read_text(encoding="utf-8"), args.version.removeprefix("v"))
        except KeyError as e:
            print(f"error: {e.args[0]}", file=sys.stderr)
            return 1
        sys.stdout.write(text)
        return 0
    if args.command == "lint":
        errors = lint(changelog.read_text(encoding="utf-8"))
    else:
        errors = check(args.tag, args.root)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
