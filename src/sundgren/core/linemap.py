"""Patch -> per-file line maps between the base and patched trees."""

from __future__ import annotations

from bisect import bisect_left, bisect_right
from collections.abc import Mapping
from dataclasses import dataclass, field

from sundgren.core.model import LineNo
from sundgren.core.patch import FilePatch, Patch


@dataclass(frozen=True, slots=True)
class LineMap:
    old_path: str | None  # None for a new file
    new_path: str | None  # None for a deleted file
    old_to_new: Mapping[LineNo, LineNo]  # explicit pairs for lines inside hunks; may be empty
    deleted_old: frozenset[LineNo]
    added_new: frozenset[LineNo]
    _deleted: tuple[LineNo, ...] = field(init=False, repr=False, compare=False)
    _added: tuple[LineNo, ...] = field(init=False, repr=False, compare=False)

    def __post_init__(self) -> None:
        object.__setattr__(self, "_deleted", tuple(sorted(self.deleted_old)))
        object.__setattr__(self, "_added", tuple(sorted(self.added_new)))

    def to_new(self, old: LineNo) -> LineNo | None:
        """Patched line for a base line, or None if it was deleted."""
        if old in self.deleted_old:
            return None
        if old in self.old_to_new:
            return self.old_to_new[old]
        return _shift(old, self._deleted, self._added)

    def to_old(self, new: LineNo) -> LineNo | None:
        """Base line for a patched line, or None if it was added."""
        if new in self.added_new:
            return None
        return _shift(new, self._added, self._deleted)

    @property
    def is_identity(self) -> bool:
        return not self.deleted_old and not self.added_new and self.old_path == self.new_path


def _shift(line: LineNo, removed: tuple[LineNo, ...], inserted: tuple[LineNo, ...]) -> LineNo:
    # Unchanged lines keep their order: rank among unchanged lines on one side
    # equals rank on the other. Solve n - #inserted(<= n) == rank by fixed point.
    rank = line - bisect_left(removed, line)
    shift = 0
    while True:
        n = bisect_right(inserted, rank + shift)
        if n == shift:
            return rank + shift
        shift = n


def identity(path: str) -> LineMap:
    return LineMap(path, path, {}, frozenset(), frozenset())


def build_linemaps(patch: Patch) -> dict[str, LineMap]:
    """LineMap for every file the patch touches, keyed by both its old and new path."""
    maps = [_linemap(fp) for fp in patch.files]
    by_path: dict[str, LineMap] = {}
    for lm in maps:
        if lm.old_path is not None:
            by_path[lm.old_path] = lm
    # New paths are unique within a diff, so on a clash (rename chains, swaps)
    # the map that produces the path wins and patched-side lookups stay exact.
    for lm in maps:
        if lm.new_path is not None:
            by_path[lm.new_path] = lm
    return dict(sorted(by_path.items()))


def _linemap(fp: FilePatch) -> LineMap:
    old_to_new: dict[LineNo, LineNo] = {}
    deleted: set[LineNo] = set()
    added: set[LineNo] = set()
    delta = 0
    for hunk in fp.hunks:
        # New positions follow from old ones and earlier hunks, as git apply
        # places them; new_start is only cross-checked by the parser.
        old = hunk.old_start + (hunk.old_count == 0)
        new = old + delta
        for line in hunk.lines:
            if line.startswith("-"):
                deleted.add(old)
                old += 1
            elif line.startswith("+"):
                added.add(new)
                new += 1
            else:
                old_to_new[old] = new
                old, new = old + 1, new + 1
        delta += hunk.new_count - hunk.old_count
    return LineMap(fp.old_path, fp.new_path, old_to_new, frozenset(deleted), frozenset(added))


def linemap_for(linemaps: Mapping[str, LineMap], path: str) -> LineMap:
    """The LineMap touching `path`, or the identity map."""
    return linemaps.get(path) or identity(path)
