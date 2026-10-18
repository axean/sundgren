"""Function and directory-tree rollups."""

from __future__ import annotations

from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import astuple, dataclass
from typing import Literal

from sundgren.core.linemap import LineMap
from sundgren.core.model import LineNo
from sundgren.core.symbols import Span, spans_by_name

FunctionStatus = Literal[
    "went_dark",
    "lit_up",
    "partial_loss",
    "partial_gain",
    "mixed",
    "unchanged",
    "added",
    "removed",
]


@dataclass(frozen=True, slots=True)
class FunctionDelta:
    name: str
    start: LineNo  # patched-side span, base side for "removed"
    end: LineNo
    status: FunctionStatus
    lost: int
    gained: int
    executable: int  # executable body lines on the reported side


@dataclass(frozen=True, slots=True)
class Stats:
    lost: int = 0
    gained: int = 0
    new_hit: int = 0
    new_miss: int = 0
    deleted_hit: int = 0
    deleted_miss: int = 0
    executable_base: int = 0
    executable_patched: int = 0
    hit_base: int = 0
    hit_patched: int = 0

    def __add__(self, other: Stats) -> Stats:
        return Stats(*(a + b for a, b in zip(astuple(self), astuple(other), strict=True)))


@dataclass(frozen=True, slots=True)
class TreeNode:
    name: str
    path: str
    kind: Literal["dir", "file"]
    stats: Stats
    children: tuple[TreeNode, ...] = ()


def function_deltas(
    base_spans: Sequence[Span] | None,
    patched_spans: Sequence[Span] | None,
    lm: LineMap,
    base_lines: Mapping[LineNo, int],
    patched_lines: Mapping[LineNo, int],
    *,
    lost: frozenset[LineNo] = frozenset(),
    gained: frozenset[LineNo] = frozenset(),
    flaky_base: frozenset[LineNo] = frozenset(),
    flaky_patched: frozenset[LineNo] = frozenset(),
) -> tuple[FunctionDelta, ...]:
    """Per-function status; None spans are borrowed from the other side, [] means no file."""
    # lost/gained hold patched line numbers; the line dicts are each side's run-0 counts.
    if base_spans is None and patched_spans is None:
        return ()
    if base_spans is None:
        assert patched_spans is not None
        base_spans = _map_spans(patched_spans, lm.to_old)
    if patched_spans is None:
        patched_spans = _map_spans(base_spans, lm.to_new)

    out: list[FunctionDelta] = []
    for sb, sp in _match(base_spans, patched_spans, lm):
        if sp is None:
            assert sb is not None
            n = sum(1 for ln in sb.body_lines if ln in base_lines)
            out.append(FunctionDelta(sb.name, sb.start, sb.end, "removed", 0, 0, n))
            continue
        executable = sum(1 for ln in sp.body_lines if ln in patched_lines)
        if sb is None:
            out.append(FunctionDelta(sp.name, sp.start, sp.end, "added", 0, 0, executable))
            continue
        n_lost = sum(1 for ln in sp.body_lines if ln in lost)
        n_gained = sum(1 for ln in sp.body_lines if ln in gained)
        b_exec, b_hit = _body(sb, base_lines, flaky_base)
        p_exec, p_hit = _body(sp, patched_lines, flaky_patched)
        status: FunctionStatus
        if b_hit and p_exec and not p_hit:
            status = "went_dark"
        elif p_hit and b_exec and not b_hit:
            status = "lit_up"
        elif n_lost and n_gained:
            status = "mixed"
        elif n_lost:
            status = "partial_loss"
        elif n_gained:
            status = "partial_gain"
        else:
            status = "unchanged"
        out.append(FunctionDelta(sp.name, sp.start, sp.end, status, n_lost, n_gained, executable))
    return tuple(sorted(out, key=lambda f: (f.start, f.end, f.name, f.status)))


def _body(span: Span, lines: Mapping[LineNo, int], flaky: frozenset[LineNo]) -> tuple[bool, bool]:
    """(any executable, any hit) over non-flaky body lines."""
    counts = [lines[ln] for ln in span.body_lines if ln in lines and ln not in flaky]
    return bool(counts), any(c > 0 for c in counts)


def _match(
    base: Sequence[Span], patched: Sequence[Span], lm: LineMap
) -> list[tuple[Span | None, Span | None]]:
    """Pair spans by qualified name, then by largest line overlap after mapping."""
    b_rest = sorted(base, key=_key)
    p_rest = sorted(patched, key=_key)
    b_named = spans_by_name(b_rest)
    pairs: list[tuple[Span | None, Span | None]] = []
    for name, sp in spans_by_name(p_rest).items():
        sb = b_named.get(name)
        if sb is not None:
            pairs.append((sb, sp))
            b_rest.remove(sb)
            p_rest.remove(sp)
    for sp in p_rest:
        mapped = {o for ln in range(sp.start, sp.end + 1) if (o := lm.to_old(ln)) is not None}
        best: Span | None = None
        best_n = 0
        for sb in b_rest:
            n = sum(1 for o in mapped if sb.start <= o <= sb.end)
            if n > best_n:
                best, best_n = sb, n
        if best is not None:
            b_rest.remove(best)
        pairs.append((best, sp))
    pairs.extend((sb, None) for sb in b_rest)
    return pairs


def _map_spans(spans: Sequence[Span], convert: Callable[[LineNo], LineNo | None]) -> list[Span]:
    """Carry spans to the other side; spans with no surviving line are dropped."""
    out = []
    for s in spans:
        mapped = [m for ln in range(s.start, s.end + 1) if (m := convert(ln)) is not None]
        if not mapped:
            continue
        body_start = None
        if s.body_start is not None:
            body = [m for ln in s.body_lines if (m := convert(ln)) is not None]
            body_start = min(body) if body else max(mapped) + 1
        out.append(Span(s.name, min(mapped), max(mapped), body_start))
    return out


def _key(span: Span) -> tuple[int, int, str]:
    return (span.start, span.end, span.name)


def build_tree(files: Iterable[tuple[str, Stats]]) -> TreeNode:
    """Directory tree over repo-relative posix paths; dir stats are sums of their children."""
    return _dir("", (), [(tuple(path.split("/")), stats) for path, stats in files])


def _dir(
    name: str, prefix: tuple[str, ...], entries: list[tuple[tuple[str, ...], Stats]]
) -> TreeNode:
    children: list[TreeNode] = []
    groups: dict[str, list[tuple[tuple[str, ...], Stats]]] = {}
    for parts, stats in entries:
        if len(parts) == 1:
            children.append(TreeNode(parts[0], "/".join(prefix + parts), "file", stats))
        else:
            groups.setdefault(parts[0], []).append((parts[1:], stats))
    children.extend(_dir(n, (*prefix, n), sub) for n, sub in groups.items())
    children.sort(key=lambda c: (c.name, c.kind))
    total = sum((c.stats for c in children), Stats())
    return TreeNode(name, "/".join(prefix), "dir", total, tuple(children))
