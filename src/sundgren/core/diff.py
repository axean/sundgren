"""(Snapshots, Snapshots, LineMaps, outcomes) -> Delta."""

from __future__ import annotations

import re
from collections import Counter
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Literal, TypeVar

from sundgren.core.hashing import sha256_file_or_none
from sundgren.core.linemap import LineMap, identity, linemap_for
from sundgren.core.model import (
    BranchKey,
    FileCoverage,
    LineNo,
    Outcome,
    ReportWarning,
    Snapshot,
)
from sundgren.core.rollup import FunctionDelta, Stats, TreeNode, build_tree, function_deltas
from sundgren.core.symbols import resolve_spans

Side = Literal["base", "patched"]
LineCategory = Literal[
    "kept_hit",
    "kept_miss",
    "lost",
    "gained",
    "unmeasured",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
]
BranchCategory = Literal[
    "branch_kept_hit",
    "branch_kept_miss",
    "branch_lost",
    "branch_gained",
    "branch_new",
    "branch_deleted",
    "branch_flaky",
    "branch_unmeasured",
]
Convert = Callable[[LineNo], LineNo | None]
_T = TypeVar("_T")


@dataclass(frozen=True, slots=True)
class LineDelta:
    line: LineNo | None  # patched line; None if deleted
    base_line: LineNo | None  # None if added
    category: LineCategory
    hits: tuple[int | None, int | None]  # (base, patched); None = absent side


@dataclass(frozen=True, slots=True)
class BranchDelta:
    line: LineNo | None
    base_line: LineNo | None
    block: str  # patched-side ids, base side if there is no patched branch
    branch: str
    category: BranchCategory
    taken: tuple[int | None, int | None]  # None = '-' or absent side


@dataclass(frozen=True, slots=True)
class FlakyLine:
    line: LineNo | None
    base_line: LineNo | None
    hits: tuple[int | None, int | None]  # from run 0
    sides: tuple[Side, ...]  # side(s) whose repeats disagreed


@dataclass(frozen=True, slots=True)
class FileDelta:
    path: str  # patched path, base path if deleted
    base_path: str | None  # None for a new file
    sha256: tuple[str | None, str | None]
    source_changed: bool
    inferred_side: Side | None
    lines: tuple[LineDelta, ...]  # kept_* included; flaky lines are only in `flaky`
    branches: tuple[BranchDelta, ...]
    functions: tuple[FunctionDelta, ...]
    flaky: tuple[FlakyLine, ...]
    stats: Stats

    @property
    def changed(self) -> bool:
        """Anything beyond kept_* lines and branches."""
        return (
            any(not e.category.startswith("kept_") for e in self.lines)
            or any(not b.category.startswith("branch_kept_") for b in self.branches)
            or any(f.status != "unchanged" for f in self.functions)
            or bool(self.flaky)
        )


@dataclass(frozen=True, slots=True)
class OutcomePair:
    id: str
    base: Outcome
    patched: Outcome


@dataclass(frozen=True, slots=True)
class Delta:
    files: tuple[FileDelta, ...]  # every non-external file on either side, sorted by path
    tests: tuple[OutcomePair, ...]  # every test, sorted by id
    tree: TreeNode
    external_files: int
    warnings: tuple[ReportWarning, ...]


class _State(Enum):
    MEASURED = "measured"
    INFERRED = "inferred"  # missing from coverage but the file exists: all MISS
    NONEXISTENT = "nonexistent"  # file not on this side
    UNMEASURED = "unmeasured"  # missing from coverage, existence unknown


@dataclass(frozen=True, slots=True)
class _Pair:
    base_path: str | None
    patched_path: str | None
    lm: LineMap


def diff(
    base: Sequence[Snapshot],
    patched: Sequence[Snapshot],
    linemaps: Mapping[str, LineMap],
    *,
    base_root: Path | None = None,
    patched_root: Path | None = None,
    base_outcomes: Mapping[str, Outcome] | None = None,
    patched_outcomes: Mapping[str, Outcome] | None = None,
) -> Delta:
    """Classify every line, branch and function; [0] of each side is canonical, the rest repeats."""
    if not base or not patched:
        raise ValueError("diff needs at least one snapshot per side")
    warnings: list[ReportWarning] = []
    files = [
        _diff_file(pair, base, patched, base_root, patched_root, warnings)
        for pair in _pair_files(base[0], patched[0], linemaps, warnings)
    ]
    files.sort(key=lambda f: (f.path, f.base_path or ""))
    external = {p for s in (base[0], patched[0]) for p, fc in s.files.items() if fc.external}
    tests = _diff_outcomes(base_outcomes, patched_outcomes, warnings)
    return Delta(
        files=tuple(files),
        tests=tests,
        tree=build_tree((f.path, f.stats) for f in files),
        external_files=len(external),
        warnings=tuple(sorted(set(warnings), key=lambda w: (w.code, w.message))),
    )


def _internal(snapshot: Snapshot) -> list[str]:
    return sorted(p for p, fc in snapshot.files.items() if not fc.external)


def _pair_files(
    base: Snapshot,
    patched: Snapshot,
    linemaps: Mapping[str, LineMap],
    warnings: list[ReportWarning],
) -> list[_Pair]:
    pairs: dict[str, _Pair] = {}
    claimed: set[str] = set()
    for p in _internal(patched):
        lm = linemap_for(linemaps, p)
        if lm.new_path != p:  # the patch says p is gone; the snapshot wins
            lm = identity(p)
        pairs[p] = _Pair(lm.old_path, p, lm)
        if lm.old_path is not None:
            claimed.add(lm.old_path)
    for b in _internal(base):
        if b in claimed:
            continue
        lm = linemap_for(linemaps, b)
        if lm.old_path != b:
            lm = identity(b)
        key = lm.new_path if lm.new_path is not None else b
        if key in pairs:
            warnings.append(
                ReportWarning(
                    "unpaired_file", f"{b}: base file collides with patched {key}; ignored"
                )
            )
            continue
        pairs[key] = _Pair(b, lm.new_path, lm)
    return list(pairs.values())


def _get(snapshot: Snapshot, path: str | None) -> FileCoverage | None:
    if path is None:
        return None
    fc = snapshot.files.get(path)
    return None if fc is None or fc.external else fc


def _state(fc: FileCoverage | None, path: str | None, root: Path | None) -> _State:
    if fc is not None:
        return _State.MEASURED
    if path is None:
        return _State.NONEXISTENT
    if root is None:
        return _State.UNMEASURED
    return _State.INFERRED if (root / path).is_file() else _State.NONEXISTENT


def _unstable(
    runs: Sequence[Snapshot], path: str | None, on: Callable[[FileCoverage], set[_T]]
) -> frozenset[_T]:
    """Items "on" in some repeats but not all; a run without the file has nothing on."""
    if len(runs) < 2 or path is None:
        return frozenset()
    sets = [on(fc) if (fc := _get(run, path)) is not None else set() for run in runs]
    return frozenset(set().union(*sets) - sets[0].intersection(*sets[1:]))


def _hit_lines(fc: FileCoverage) -> set[LineNo]:
    return {ln for ln, c in fc.lines.items() if c > 0}


def _taken_branches(fc: FileCoverage) -> set[BranchKey]:
    return {k for k, t in fc.branches.items() if t}


def _never(_: LineNo) -> LineNo | None:
    return None


def _diff_file(
    pair: _Pair,
    base_runs: Sequence[Snapshot],
    patched_runs: Sequence[Snapshot],
    base_root: Path | None,
    patched_root: Path | None,
    warnings: list[ReportWarning],
) -> FileDelta:
    lm = pair.lm
    fc_b = _get(base_runs[0], pair.base_path)
    fc_p = _get(patched_runs[0], pair.patched_path)
    state_b = _state(fc_b, pair.base_path, base_root)
    state_p = _state(fc_p, pair.patched_path, patched_root)
    base_path = None if state_b is _State.NONEXISTENT else pair.base_path
    patched_path = None if state_p is _State.NONEXISTENT else pair.patched_path
    path = patched_path if patched_path is not None else base_path
    assert path is not None  # one side is always measured
    to_new: Convert = lm.to_new if patched_path is not None else _never
    to_old: Convert = lm.to_old if base_path is not None else _never

    lines_b: dict[LineNo, int] = dict(fc_b.lines) if fc_b else {}
    lines_p: dict[LineNo, int] = dict(fc_p.lines) if fc_p else {}
    br_b: dict[BranchKey, int | None] = dict(fc_b.branches) if fc_b else {}
    br_p: dict[BranchKey, int | None] = dict(fc_p.branches) if fc_p else {}
    inferred_side: Side | None = None
    # Nothing executable on the measured side means nothing to infer: no flag, no warning.
    if state_b is _State.INFERRED and (lines_p or br_p):
        inferred_side = "base"
        lines_b, br_b = _inferred(lines_p, br_p, to_old)
    elif state_p is _State.INFERRED and (lines_b or br_b):
        inferred_side = "patched"
        lines_p, br_p = _inferred(lines_b, br_b, to_new)
    if inferred_side is not None:
        warnings.append(
            ReportWarning(
                "inferred_file",
                f"{path}: not in {inferred_side} coverage but present in the {inferred_side} "
                "tree; counted as never executed",
            )
        )

    flaky_b = _unstable(base_runs, base_path, _hit_lines)
    flaky_p = _unstable(patched_runs, patched_path, _hit_lines)
    entries, flaky = _diff_lines(lines_b, lines_p, to_new, to_old, flaky_b, flaky_p)
    branches = _diff_branches(
        br_b,
        br_p,
        to_new,
        to_old,
        _unstable(base_runs, base_path, _taken_branches),
        _unstable(patched_runs, patched_path, _taken_branches),
        inferred_side,
    )
    _warn_unmeasured(path, state_b, state_p, entries, branches, warnings)

    functions = function_deltas(
        [] if base_path is None else resolve_spans(base_path, base_root, fc_b),
        [] if patched_path is None else resolve_spans(patched_path, patched_root, fc_p),
        lm,
        lines_b,
        lines_p,
        lost=frozenset(e.line for e in entries if e.category == "lost" and e.line is not None),
        gained=frozenset(e.line for e in entries if e.category == "gained" and e.line is not None),
        flaky_base=flaky_b,
        flaky_patched=flaky_p,
    )

    sha_b = sha256_file_or_none(base_root / base_path if base_root and base_path else None)
    sha_p = sha256_file_or_none(
        patched_root / patched_path if patched_root and patched_path else None
    )
    if sha_b is not None and sha_p is not None:
        source_changed = sha_b != sha_p
    else:
        source_changed = (
            base_path is None or patched_path is None or bool(lm.deleted_old or lm.added_new)
        )

    cats = Counter(e.category for e in entries)
    stats = Stats(
        lost=cats["lost"],
        gained=cats["gained"],
        new_hit=cats["new_hit"],
        new_miss=cats["new_miss"],
        deleted_hit=cats["deleted_hit"],
        deleted_miss=cats["deleted_miss"],
        executable_base=len(lines_b),
        executable_patched=len(lines_p),
        hit_base=sum(1 for c in lines_b.values() if c > 0),
        hit_patched=sum(1 for c in lines_p.values() if c > 0),
    )
    return FileDelta(
        path=path,
        base_path=base_path,
        sha256=(sha_b, sha_p),
        source_changed=source_changed,
        inferred_side=inferred_side,
        lines=entries,
        branches=branches,
        functions=functions,
        flaky=flaky,
        stats=stats,
    )


def _inferred(
    lines: Mapping[LineNo, int], branches: Mapping[BranchKey, int | None], convert: Convert
) -> tuple[dict[LineNo, int], dict[BranchKey, int | None]]:
    """The other side's executable lines and branches, carried over as never executed."""
    out_lines = {n: 0 for ln in lines if (n := convert(ln)) is not None}
    out_branches: dict[BranchKey, int | None] = {}
    for k in branches:
        n = convert(k.line)
        branch = _remap_branch_id(k.branch, convert)
        if n is not None and branch is not None:
            out_branches[BranchKey(n, k.block, branch)] = 0
    return out_lines, out_branches


def _diff_lines(
    lines_b: Mapping[LineNo, int],
    lines_p: Mapping[LineNo, int],
    to_new: Convert,
    to_old: Convert,
    flaky_b: frozenset[LineNo],
    flaky_p: frozenset[LineNo],
) -> tuple[tuple[LineDelta, ...], tuple[FlakyLine, ...]]:
    pairs: set[tuple[LineNo | None, LineNo | None]] = {
        (b, to_new(b)) for b in lines_b.keys() | flaky_b
    }
    pairs |= {(to_old(p), p) for p in lines_p.keys() | flaky_p}
    entries: list[LineDelta] = []
    flaky: list[FlakyLine] = []
    for b, p in pairs:
        hits = (None if b is None else lines_b.get(b), None if p is None else lines_p.get(p))
        sides: tuple[Side, ...] = ()
        if b in flaky_b:
            sides += ("base",)
        if p in flaky_p:
            sides += ("patched",)
        if sides:
            flaky.append(FlakyLine(p, b, hits, sides))
        else:
            entries.append(LineDelta(p, b, _line_category(b, p, *hits), hits))
    entries.sort(key=lambda e: _order(e.line, e.base_line))
    flaky.sort(key=lambda e: _order(e.line, e.base_line))
    return tuple(entries), tuple(flaky)


def _line_category(
    b: LineNo | None, p: LineNo | None, hb: int | None, hp: int | None
) -> LineCategory:
    if b is None:
        return "new_hit" if hp else "new_miss"
    if p is None:
        return "deleted_hit" if hb else "deleted_miss"
    if hb is None or hp is None:
        return "unmeasured"
    if hb > 0:
        return "kept_hit" if hp > 0 else "lost"
    return "gained" if hp > 0 else "kept_miss"


_LINE_REF = re.compile(r"\bline (\d+)\b")


def _remap_branch_id(branch: str, convert: Convert) -> str | None:
    """Map "line N" refs in a branch id (coverage.py "jump to line N"); None if one doesn't map."""
    targets = [convert(int(n)) for n in _LINE_REF.findall(branch)]
    if None in targets:
        return None
    it = iter(targets)
    return _LINE_REF.sub(lambda _: f"line {next(it)}", branch)


def _diff_branches(
    br_b: Mapping[BranchKey, int | None],
    br_p: Mapping[BranchKey, int | None],
    to_new: Convert,
    to_old: Convert,
    flaky_b: frozenset[BranchKey],
    flaky_p: frozenset[BranchKey],
    inferred_side: Side | None,
) -> tuple[BranchDelta, ...]:
    """Match branches by (mapped line, block, id with "line N" mapped); classify the rest."""
    # A one-sided branch on an unchanged line is unmeasured when the other side has no branch
    # data on that line, unless that side was inferred (it has exactly the copied branches).
    keys_p = br_p.keys() | flaky_p
    lines_with_b = {k.line for k in br_b.keys() | flaky_b}
    lines_with_p = {k.line for k in keys_p}
    matched: set[BranchKey] = set()
    out: list[BranchDelta] = []
    for kb in sorted(br_b.keys() | flaky_b):
        p = to_new(kb.line)
        branch = None if p is None else _remap_branch_id(kb.branch, to_new)
        kp = None if p is None or branch is None else BranchKey(p, kb.block, branch)
        if kp is not None and kp not in keys_p:
            kp = None
        category: BranchCategory
        if kp is not None:
            matched.add(kp)
            category = _branch_category(kb, kp, br_b, br_p, flaky_b, flaky_p)
        elif kb in flaky_b:
            category = "branch_flaky"
        elif p is not None and p not in lines_with_p and inferred_side != "patched":
            category = "branch_unmeasured"
        else:
            category = "branch_deleted"
        key = kp if kp is not None else kb
        taken = (br_b.get(kb), None if kp is None else br_p.get(kp))
        out.append(BranchDelta(p, kb.line, key.block, key.branch, category, taken))
    for kp in sorted(keys_p - matched):
        b = to_old(kp.line)
        if kp in flaky_p:
            category = "branch_flaky"
        elif b is not None and b not in lines_with_b and inferred_side != "base":
            category = "branch_unmeasured"
        else:
            category = "branch_new"
        out.append(BranchDelta(kp.line, b, kp.block, kp.branch, category, (None, br_p.get(kp))))
    out.sort(
        key=lambda e: (
            *_order(e.line, e.base_line),
            _natural(e.block),
            _natural(e.branch),
            e.category,
        )
    )
    return tuple(out)


def _branch_category(
    kb: BranchKey,
    kp: BranchKey,
    br_b: Mapping[BranchKey, int | None],
    br_p: Mapping[BranchKey, int | None],
    flaky_b: frozenset[BranchKey],
    flaky_p: frozenset[BranchKey],
) -> BranchCategory:
    if kb in flaky_b or kp in flaky_p:
        return "branch_flaky"
    if br_b.get(kb):
        return "branch_kept_hit" if br_p.get(kp) else "branch_lost"
    return "branch_gained" if br_p.get(kp) else "branch_kept_miss"


_DIGITS = re.compile(r"(\d+)")


def _natural(s: str) -> tuple[str | int, ...]:
    """Sort key comparing digit runs as numbers ("line 9" < "line 10")."""
    return tuple(int(t) if i % 2 else t for i, t in enumerate(_DIGITS.split(s)))


def _order(line: LineNo | None, base_line: LineNo | None) -> tuple[int, int, int]:
    """Patched lines in order, then deleted lines (line None) by base line."""
    if line is None:
        return (1, base_line or 0, 0)
    return (0, line, base_line or 0)


def _warn_unmeasured(
    path: str,
    state_b: _State,
    state_p: _State,
    lines: Iterable[LineDelta],
    branches: Iterable[BranchDelta],
    warnings: list[ReportWarning],
) -> None:
    n_lines = sum(1 for e in lines if e.category == "unmeasured")
    n_branches = sum(1 for e in branches if e.category == "branch_unmeasured")
    if not n_lines and not n_branches:
        return
    why = "measured on one side only"
    if _State.UNMEASURED in (state_b, state_p):
        side = "base" if state_b is _State.UNMEASURED else "patched"
        why = f"not in {side} coverage and the {side} tree is unknown"
    warnings.append(
        ReportWarning(
            "unmeasured", f"{path}: {n_lines} line(s), {n_branches} branch(es) unmeasured: {why}"
        )
    )


def _diff_outcomes(
    base: Mapping[str, Outcome] | None,
    patched: Mapping[str, Outcome] | None,
    warnings: list[ReportWarning],
) -> tuple[OutcomePair, ...]:
    if base is None or patched is None:
        if base is not None or patched is not None:
            warnings.append(
                ReportWarning("tests_one_side", "test outcomes for one side only; tests[] is empty")
            )
        return ()
    return tuple(
        OutcomePair(t, base.get(t, "not_run"), patched.get(t, "not_run"))
        for t in sorted(base.keys() | patched.keys())
    )
