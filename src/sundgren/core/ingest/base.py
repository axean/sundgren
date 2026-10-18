"""Ingestor protocol: coverage text -> Snapshot with raw (unnormalized) paths."""

from __future__ import annotations

from typing import Protocol

from sundgren.core.model import Snapshot


class Ingestor(Protocol):
    def ingest(self, text: str) -> Snapshot: ...
