"""e2e: run the real CLI against the shed fixtures."""

from __future__ import annotations

import pytest


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers", "slow: runs real coverage tools (deselect with -m 'not slow')"
    )
