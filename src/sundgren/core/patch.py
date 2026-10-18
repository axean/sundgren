"""Git-style unified diff parser."""

from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Hunk:
    old_start: int
    old_count: int
    new_start: int
    new_count: int
    lines: tuple[str, ...]  # body lines with their ' ', '-' or '+' prefix; '\ No newline' dropped


@dataclass(frozen=True, slots=True)
class FilePatch:
    old_path: str | None  # None for a new file; a/ prefix stripped
    new_path: str | None  # None for a deleted file; b/ prefix stripped
    hunks: tuple[Hunk, ...]
    binary: bool = False

    @property
    def path(self) -> str:
        path = self.new_path if self.new_path is not None else self.old_path
        assert path is not None
        return path


@dataclass(frozen=True, slots=True)
class Patch:
    files: tuple[FilePatch, ...]
    warnings: tuple[str, ...] = ()

    def touched_paths(self) -> tuple[str, ...]:
        """Sorted old and new paths of every file in the patch."""
        paths = {p for f in self.files for p in (f.old_path, f.new_path) if p is not None}
        return tuple(sorted(paths))


_HUNK = re.compile(r"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
_BINARY = re.compile(r"Binary files (.+) and (.+) differ")
_HEADER_END = ("diff --git ", "--- ", "@@ ", "Binary files ", "GIT binary patch")
_ESCAPES = {"a": 7, "b": 8, "t": 9, "n": 10, "v": 11, "f": 12, "r": 13, '"': 34, "\\": 92}


def parse_patch(text: str) -> Patch:
    """Parse a git-style unified diff; raise ValueError on malformed input."""
    return _Parser(text).parse()


class _PathError(ValueError):
    pass


class _Parser:
    def __init__(self, text: str) -> None:
        self.lines = text.split("\n")
        if self.lines[-1] == "":
            self.lines.pop()
        self.i = 0
        self.files: list[FilePatch] = []
        self.warnings: list[str] = []
        self.seen: set[str] = set()

    def peek(self, offset: int = 0) -> str | None:
        j = self.i + offset
        return self.lines[j] if j < len(self.lines) else None

    def error(self, msg: str, lineno: int | None = None) -> ValueError:
        return ValueError(f"line {self.i + 1 if lineno is None else lineno}: {msg}")

    def parse(self) -> Patch:
        while (line := self.peek()) is not None:
            try:
                self.step(line)
            except _PathError as exc:
                raise self.error(str(exc)) from None
        if not self.files and any(line.strip() for line in self.lines):
            self.warnings.append("no file diffs found in patch")
        return Patch(tuple(self.files), tuple(self.warnings))

    def step(self, line: str) -> None:
        start = self.i + 1
        if line.startswith("diff --git "):
            self.add(self.git_file(), start)
        elif line.startswith(("diff --cc ", "diff --combined ")):
            raise self.error("combined diffs are not supported")
        elif (
            line.startswith("--- ")
            and (self.peek(1) or "").startswith("+++ ")
            and (self.peek(2) or "").startswith("@@ ")
        ):
            self.add(self.plain_file(), start)
        elif m := _BINARY.fullmatch(line.rstrip("\r")):
            old, new = _plain_names(_side_path(m[1]), _side_path(m[2]))
            self.i += 1
            self.add(self.binary(old, new, start), start)
        else:
            self.i += 1  # preamble, trailers, `diff -r` noise

    def add(self, fp: FilePatch, start: int) -> None:
        # One git diff names each target once; a repeat means a concatenated
        # series, whose line numbers chain and can't be keyed by path.
        if fp.new_path is not None:
            if fp.new_path in self.seen:
                raise self.error(
                    f"{fp.new_path} is patched twice (patch series?); use one combined diff", start
                )
            self.seen.add(fp.new_path)
        self.files.append(fp)

    def binary(self, old: str | None, new: str | None, start: int) -> FilePatch:
        if old is None and new is None:
            raise self.error("binary diff without a path", start)
        fp = FilePatch(old, new, (), binary=True)
        self.warnings.append(f"{fp.path}: binary diff ignored")
        return fp

    def git_file(self) -> FilePatch:
        start = self.i + 1
        old, new = _git_header_paths(self.lines[self.i][len("diff --git ") :].rstrip("\r"))
        created = deleted = copied = False
        self.i += 1
        # Extended header; unknown lines are skipped, as git apply does.
        line = (self.peek() or "").rstrip("\r")
        while self.i < len(self.lines) and not line.startswith(_HEADER_END):
            if line.startswith("new file mode "):
                created = True
            elif line.startswith("deleted file mode "):
                deleted = True
            elif line.startswith(("rename from ", "copy from ")):
                old = _unquote(line.split(" ", 2)[2])
                copied = line.startswith("copy")
            elif line.startswith(("rename to ", "copy to ")):
                new = _unquote(line.split(" ", 2)[2])
            self.i += 1
            line = (self.peek() or "").rstrip("\r")
        if created:
            old = None
        if deleted:
            new = None
        if line.startswith(("Binary files ", "GIT binary patch")):
            self.i += 1
            if line.startswith("GIT binary patch"):
                while (nxt := self.peek()) is not None and not nxt.startswith("diff --git "):
                    self.i += 1
            return self.binary(old, new, start)
        if line.startswith("@@ "):
            raise self.error("hunk without '---'/'+++' lines")
        hunks: tuple[Hunk, ...] = ()
        if line.startswith("--- "):
            minus, plus = self.side_paths()
            # The mode lines win over '---'/'+++', as in git apply.
            old = None if created else minus
            new = None if deleted else plus
            hunks = self.hunks(new or old or "")
        if old is None and new is None:
            raise self.error("cannot determine file path", start)
        if copied:
            self.warnings.append(f"{new}: copy of {old} treated as a rename")
        return self.checked(FilePatch(old, new, hunks), start)

    def plain_file(self) -> FilePatch:
        start = self.i + 1
        old, new = _plain_names(*self.side_paths())
        if old is None and new is None:
            raise self.error("both sides are /dev/null", start)
        return self.checked(FilePatch(old, new, self.hunks(new or old or "")), start)

    def side_paths(self) -> tuple[str | None, str | None]:
        if not (self.peek(1) or "").startswith("+++ "):
            raise self.error("'---' line not followed by '+++'")
        old = _side_path(self.lines[self.i][4:])
        self.i += 1
        new = _side_path(self.lines[self.i][4:])
        self.i += 1
        return old, new

    def checked(self, fp: FilePatch, start: int) -> FilePatch:
        if fp.old_path is None and any(h.old_count for h in fp.hunks):
            raise self.error("new file has a hunk with old lines", start)
        if fp.new_path is None and any(h.new_count for h in fp.hunks):
            raise self.error("deleted file has a hunk with new lines", start)
        return fp

    def hunks(self, path: str) -> tuple[Hunk, ...]:
        hunks: list[Hunk] = []
        old_end = new_end = 1  # first line not yet covered by a hunk
        delta = 0
        while (line := self.peek()) is not None and line.startswith("@@ "):
            lineno = self.i + 1
            hunk = self.hunk()
            # A zero-length range starts at the line before it.
            old_first = hunk.old_start + (hunk.old_count == 0)
            new_first = hunk.new_start + (hunk.new_count == 0)
            if old_first < old_end or new_first < new_end:
                raise self.error("hunk overlaps or precedes the previous one", lineno)
            if new_first - old_first != delta:
                self.warnings.append(f"{path}: hunk at line {lineno} is offset from its header")
            old_end, new_end = old_first + hunk.old_count, new_first + hunk.new_count
            delta += hunk.new_count - hunk.old_count
            hunks.append(hunk)
        line = (self.peek() or "").rstrip("\r")
        # '--- ' may start the next plain diff; '-- ' and '--' start mail signatures.
        if line[:1] in (" ", "+", "-") and not line.startswith(("--- ", "-- ")) and line != "--":
            raise self.error("hunk has more lines than its header says")
        return tuple(hunks)

    def hunk(self) -> Hunk:
        m = _HUNK.match(self.lines[self.i])
        if m is None:
            raise self.error("malformed hunk header")
        old_start, new_start = int(m[1]), int(m[3])
        old_count = 1 if m[2] is None else int(m[2])
        new_count = 1 if m[4] is None else int(m[4])
        if (old_count and not old_start) or (new_count and not new_start):
            raise self.error("hunk range starts at line 0")
        self.i += 1
        body: list[str] = []
        old_left, new_left = old_count, new_count
        while old_left or new_left:
            line = self.peek()
            if line is None:
                raise self.error("patch ends inside a hunk", len(self.lines))
            tag = line[:1]
            if tag == "\\":
                self.i += 1
                continue
            if tag == " " or line in ("", "\r"):  # some tools strip the space off empty context
                old_left, new_left = old_left - 1, new_left - 1
                line = " " + line[1:]
            elif tag == "-":
                old_left -= 1
            elif tag == "+":
                new_left -= 1
            else:
                raise self.error("unexpected line in hunk")
            if old_left < 0 or new_left < 0:
                raise self.error("hunk has more lines than its header says")
            body.append(line)
            self.i += 1
        if (line := self.peek()) is not None and line.startswith("\\"):
            self.i += 1
        return Hunk(old_start, old_count, new_start, new_count, tuple(body))


def _git_header_paths(rest: str) -> tuple[str | None, str | None]:
    if rest.startswith('"'):
        quoted = _quoted(rest)
        if quoted is None or not quoted[1].startswith((" ", "\t")):
            return None, None
        old, tail = quoted[0], quoted[1][1:]
    elif rest.endswith('"') and ' "' in rest:
        cut = rest.index(' "')
        old, tail = rest[:cut], rest[cut + 1 :]
    else:
        # Unquoted names may hold spaces; like git, accept only "X/P Y/P".
        name = _strip(rest)
        for i, c in enumerate(name):
            if c in " \t" and _strip(name[i + 1 :]) == name[:i]:
                return name[:i], name[:i]
        return None, None
    return _strip(old), _strip(_unquote(tail))


def _side_path(raw: str) -> str | None:
    raw = raw.rstrip("\r")
    path = _unquote(raw) if raw.startswith('"') else raw.split("\t", 1)[0]
    return None if path == "/dev/null" else _strip(path)


def _strip(path: str) -> str:
    """Drop the first path component like `git apply -p1`; a bare name stays."""
    return path.split("/", 1)[1] if "/" in path else path


def _plain_names(old: str | None, new: str | None) -> tuple[str | None, str | None]:
    """Plain diffs never rename: git apply uses the '+++' name, or '---' if it prefixes it."""
    if old is None or new is None:
        return old, new
    name = old if new.startswith(old) else new
    return name, name


def _unquote(raw: str) -> str:
    if not raw.startswith('"'):
        return raw
    quoted = _quoted(raw)
    if quoted is None:
        raise _PathError(f"bad quoted path {raw!r}")
    return quoted[0]


def _quoted(raw: str) -> tuple[str, str] | None:
    """Decode a leading C-quoted string; return it and the text after the closing quote."""
    out = bytearray()
    i = 1
    while i < len(raw):
        c = raw[i]
        if c == '"':
            return out.decode("utf-8", "surrogateescape"), raw[i + 1 :]
        if c != "\\":
            out += c.encode("utf-8", "surrogateescape")
            i += 1
            continue
        esc = raw[i + 1 : i + 2]
        if esc in _ESCAPES:
            out.append(_ESCAPES[esc])
            i += 2
        elif len(octal := raw[i + 1 : i + 4]) == 3 and all(ch in "01234567" for ch in octal):
            out.append(int(octal, 8) & 0xFF)
            i += 4
        else:
            return None
    return None
