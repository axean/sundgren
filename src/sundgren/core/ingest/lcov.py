"""lcov tracefile parser and dumper."""

from __future__ import annotations

import logging
from collections.abc import Callable

from sundgren.core.model import (
    BranchKey,
    FileCoverage,
    FunctionRecord,
    Snapshot,
    merge_file_coverage,
)

_log = logging.getLogger(__name__)

# Summary records are recomputed by dump(); VER has no slot in the model.
_DROPPED = frozenset({"FNF", "FNH", "BRF", "BRH", "LF", "LH", "VER"})


def _add_taken(a: int | None, b: int | None) -> int | None:
    if a is None:
        return b
    if b is None:
        return a
    return a + b


class _Record:
    """Accumulator for one SF section."""

    __slots__ = ("path", "lines", "branches", "spans", "hits", "fn_index")

    def __init__(self, path: str) -> None:
        self.path = path
        self.lines: dict[int, int] = {}
        self.branches: dict[BranchKey, int | None] = {}
        self.spans: dict[str, tuple[int, int | None]] = {}
        self.hits: dict[str, int] = {}
        self.fn_index: dict[str, tuple[int, int | None]] = {}

    def da(self, value: str) -> None:
        line, count, *_checksum = value.split(",")
        ln = int(line)
        self.lines[ln] = self.lines.get(ln, 0) + int(count)

    def brda(self, value: str) -> None:
        # Branch ids may contain commas (lcov 2.x expressions), so split from both ends.
        line, block, rest = value.split(",", 2)
        branch, taken_s = rest.rsplit(",", 1)
        taken = None if taken_s == "-" else int(taken_s)
        key = BranchKey(int(line), block, branch)
        self.branches[key] = (
            _add_taken(self.branches[key], taken) if key in self.branches else taken
        )

    def fn(self, value: str) -> None:
        start, _, rest = value.partition(",")
        mid, sep, name = rest.partition(",")
        if sep and mid.isascii() and mid.isdigit():
            self._span(name, int(start), int(mid))  # lcov 2.x: FN:<start>,<end>,<name>
        else:
            self._span(rest, int(start), None)

    def fnda(self, value: str) -> None:
        count, _, name = value.partition(",")
        self._hit(name, int(count))

    def fnl(self, value: str) -> None:
        index, *span = value.split(",")
        if len(span) not in (1, 2):
            raise ValueError(value)
        self.fn_index[index] = (int(span[0]), int(span[1]) if len(span) == 2 else None)

    def fna(self, value: str) -> None:
        index, count, name = value.split(",", 2)
        start, end = self.fn_index[index]
        self._span(name, start, end)
        self._hit(name, int(count))

    def _span(self, name: str, start: int, end: int | None) -> None:
        if not name:
            raise ValueError("empty function name")
        old = self.spans.get(name)
        if old is None:
            self.spans[name] = (start, end)
        elif old[1] is None:
            self.spans[name] = (old[0], end)

    def _hit(self, name: str, count: int) -> None:
        if not name:
            raise ValueError("empty function name")
        self.hits[name] = self.hits.get(name, 0) + count

    def build(self) -> FileCoverage:
        orphans = sorted(self.hits.keys() - self.spans.keys())
        if orphans:
            _log.debug("%s: dropping FNDA without FN: %s", self.path, orphans)
        # TODO: demangle Rust names
        functions = [
            FunctionRecord(name=name, start=start, end=end, hits=self.hits.get(name))
            for name, (start, end) in self.spans.items()
        ]
        return _sorted(
            FileCoverage(
                path=self.path,
                lines=self.lines,
                branches=self.branches,
                functions=tuple(functions),
                external=False,
            )
        )


_HANDLERS: dict[str, Callable[[_Record, str], None]] = {
    "DA": _Record.da,
    "BRDA": _Record.brda,
    "FN": _Record.fn,
    "FNDA": _Record.fnda,
    "FNL": _Record.fnl,
    "FNA": _Record.fna,
}


def _sorted(fc: FileCoverage) -> FileCoverage:
    return FileCoverage(
        path=fc.path,
        lines={ln: fc.lines[ln] for ln in sorted(fc.lines)},
        branches={key: fc.branches[key] for key in sorted(fc.branches)},
        functions=tuple(sorted(fc.functions, key=lambda f: (f.start, f.name))),
        external=fc.external,
    )


def _store(files: dict[str, FileCoverage], fc: FileCoverage) -> None:
    old = files.get(fc.path)
    files[fc.path] = fc if old is None else _sorted(merge_file_coverage(old, fc))


def parse(text: str) -> Snapshot:
    """Parse an lcov tracefile. Paths stay raw; duplicate SF sections are merged."""
    files: dict[str, FileCoverage] = {}
    test_names: dict[str, None] = {}
    rec: _Record | None = None
    # split("\n"), not splitlines(): that also splits on \f, \x85, \u2028 etc. inside paths.
    for lineno, raw in enumerate(text.removeprefix("\ufeff").split("\n"), 1):
        line = raw.strip()
        if not line:
            continue
        if line == "end_of_record":
            if rec is not None:
                _store(files, rec.build())
            rec = None
            continue
        tag, sep, value = line.partition(":")
        if sep and tag == "TN":
            if value:
                test_names.setdefault(value, None)
        elif sep and tag == "SF":
            if rec is not None:
                _store(files, rec.build())
            rec = _Record(value)
        elif sep and tag in _DROPPED:
            pass
        elif not sep or tag not in _HANDLERS:
            _log.debug("lcov line %d: ignoring unknown record %r", lineno, line)
        elif rec is None:
            _log.debug("lcov line %d: ignoring %s record outside SF", lineno, tag)
        else:
            try:
                _HANDLERS[tag](rec, value)
            except (ValueError, KeyError) as exc:
                raise ValueError(f"lcov line {lineno}: malformed {tag} record: {line!r}") from exc
    if rec is not None:
        _store(files, rec.build())
    return Snapshot(files={p: files[p] for p in sorted(files)}, test_names=tuple(test_names))


def dump(snapshot: Snapshot) -> str:
    """Serialize a Snapshot back to lcov (summary records recomputed)."""
    out = [f"TN:{name}" for name in snapshot.test_names]
    for key in sorted(snapshot.files):
        fc = snapshot.files[key]
        out.append(f"SF:{fc.path}")
        if fc.functions:
            for f in fc.functions:
                span = f"{f.start}" if f.end is None else f"{f.start},{f.end}"
                out.append(f"FN:{span},{f.name}")
            out.extend(f"FNDA:{f.hits},{f.name}" for f in fc.functions if f.hits is not None)
            out.append(f"FNF:{len(fc.functions)}")
            out.append(f"FNH:{sum(1 for f in fc.functions if f.hits)}")
        if fc.branches:
            for bk in sorted(fc.branches):
                taken = fc.branches[bk]
                out.append(
                    f"BRDA:{bk.line},{bk.block},{bk.branch},{'-' if taken is None else taken}"
                )
            out.append(f"BRF:{len(fc.branches)}")
            out.append(f"BRH:{sum(1 for t in fc.branches.values() if t)}")
        if fc.lines:
            out.extend(f"DA:{ln},{fc.lines[ln]}" for ln in sorted(fc.lines))
            out.append(f"LF:{len(fc.lines)}")
            out.append(f"LH:{sum(1 for c in fc.lines.values() if c > 0)}")
        out.append("end_of_record")
    return "".join(f"{line}\n" for line in out)


class LcovIngestor:
    def ingest(self, text: str) -> Snapshot:
        return parse(text)
