"""Test selectors: pick which tests a patch should run. Registry keyed by id."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from sundgren.core.patch import Patch


@dataclass(frozen=True, slots=True)
class Selection:
    tests: tuple[str, ...]
    run_all: bool
    reason: str


class Selector(Protocol):
    def select(self, patch: Patch, repo: Path) -> Selection: ...


_REGISTRY: dict[str, Selector] = {}


def register_selector(selector_id: str, selector: Selector) -> None:
    _REGISTRY[selector_id] = selector


def get_selector(selector_id: str) -> Selector:
    try:
        return _REGISTRY[selector_id]
    except KeyError:
        raise KeyError(f"unknown selector {selector_id!r}") from None


RUN_ALL = Selection(tests=(), run_all=True, reason="no selector; running all tests")


def choose(
    tests: tuple[str, ...] | None, selector_id: str | None, patch: Patch, repo: Path
) -> Selection:
    """--tests beats the preset's selector, which beats run-all."""
    if tests:
        return Selection(tests=tests, run_all=False, reason="--tests given")
    if selector_id is not None:
        return get_selector(selector_id).select(patch, repo)
    return RUN_ALL


def restrict_to_tree(selection: Selection, tree: Path) -> Selection | None:
    """Drop path-style tests (`path[::id]`) missing from `tree`; None if none are left."""
    if selection.run_all or not selection.tests:
        return selection
    kept = tuple(t for t in selection.tests if (tree / t.split("::", 1)[0]).exists())
    if kept == selection.tests:
        return selection
    if not kept:
        return None
    dropped = len(selection.tests) - len(kept)
    return Selection(kept, False, f"{selection.reason}; {dropped} missing on this side")
