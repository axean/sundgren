"""Test selectors: pick which tests a patch should run. Registry keyed by id."""

from __future__ import annotations

from pathlib import Path
from typing import Protocol

from sundgren._native import run_selectors as _impl
from sundgren.core.patch import Patch

Selection = _impl.Selection


class Selector(Protocol):
    def select(self, patch: Patch, repo: Path) -> Selection: ...


# Filled by lang packs; the Rust functions read it at call time.
_REGISTRY: dict[str, Selector] = {}

register_selector = _impl.register_selector
get_selector = _impl.get_selector
RUN_ALL = _impl.RUN_ALL
choose = _impl.choose
restrict_to_tree = _impl.restrict_to_tree
