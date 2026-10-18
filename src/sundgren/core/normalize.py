"""Path normalization: raw SF paths -> repo-relative posix paths."""

from __future__ import annotations

import os
from collections.abc import Sequence
from dataclasses import replace

from sundgren.core.model import FileCoverage, Snapshot, merge_file_coverage

# (anchor, parts); anchor is "" (relative), "/" or a drive like "C:/".
_Split = tuple[str, tuple[str, ...]]


def _split(path: str) -> _Split:
    """Lexically clean a path: backslashes to '/', drop '.', resolve '..'."""
    p = path.replace("\\", "/")
    if len(p) >= 2 and p[1] == ":" and p[0].isascii() and p[0].isalpha() and p[2:3] in ("", "/"):
        anchor, rest = p[0].upper() + ":/", p[2:]
    elif p.startswith("/"):
        anchor, rest = "/", p
    else:
        anchor, rest = "", p
    parts: list[str] = []
    for part in rest.split("/"):
        if part in ("", "."):
            continue
        if part != "..":
            parts.append(part)
        elif parts and parts[-1] != "..":
            parts.pop()
        elif not anchor:
            parts.append("..")  # escapes the tree; '..' above an absolute root is dropped
    return anchor, tuple(parts)


def _join(split: _Split) -> str:
    anchor, parts = split
    return anchor + "/".join(parts) if anchor or parts else "."


def _prepare(roots: Sequence[str]) -> list[_Split]:
    """Clean roots, add realpaths of absolute ones, longest first."""
    out: set[_Split] = set()
    for root in roots:
        split = _split(root)
        if split == ("", ()):
            continue
        out.add(split)
        if split[0] and os.path.isabs(root):
            out.add(_split(os.path.realpath(root)))
    return sorted(out, key=lambda r: (-len(r[1]), r))


def _strip(split: _Split, roots: list[_Split]) -> tuple[str, ...] | None:
    anchor, parts = split
    for r_anchor, r_parts in roots:
        n = len(r_parts)
        if r_anchor == anchor and len(parts) > n and parts[:n] == r_parts:
            return parts[n:]
    return None


def _normalize(raw: str, roots: list[_Split]) -> tuple[str, bool]:
    split = _split(raw)
    rest = _strip(split, roots)
    cleaned = _join(split)
    if rest is None and split[0] and os.path.isabs(cleaned):
        rest = _strip(_split(os.path.realpath(cleaned)), roots)
    if rest is not None:
        return "/".join(rest), False
    anchor, parts = split
    if not anchor and parts and parts[0] != "..":
        return cleaned, False
    return cleaned, True


def normalize_path(raw: str, roots: Sequence[str]) -> tuple[str, bool]:
    """Return (path, external). Strips the longest matching root; external if unresolvable."""
    return _normalize(raw, _prepare(roots))


def _merge(a: FileCoverage, b: FileCoverage) -> FileCoverage:
    m = merge_file_coverage(a, b)
    return replace(
        m,
        lines={ln: m.lines[ln] for ln in sorted(m.lines)},
        branches={key: m.branches[key] for key in sorted(m.branches)},
    )


def normalize_snapshot(snapshot: Snapshot, roots: Sequence[str]) -> Snapshot:
    """Re-key a Snapshot by normalized path, merging records that collide."""
    prepared = _prepare(roots)
    files: dict[str, FileCoverage] = {}
    for raw in sorted(snapshot.files):
        path, external = _normalize(raw, prepared)
        fc = replace(snapshot.files[raw], path=path, external=external)
        old = files.get(path)
        files[path] = fc if old is None else _merge(old, fc)
    return Snapshot(files={p: files[p] for p in sorted(files)}, test_names=snapshot.test_names)
