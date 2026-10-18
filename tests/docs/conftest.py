"""Docs tests: run the examples under docs/examples/ once per session."""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

import pytest

import docs_examples
from docs_examples import Example


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers", "slow: runs real coverage tools (deselect with -m 'not slow')"
    )


@pytest.fixture(scope="session")
def produce() -> Callable[[Example], dict[str, str]]:
    """Run an example (cached per session); skip or fail when its `requires` don't hold."""
    cache: dict[Path, dict[str, str]] = {}
    required = docs_examples.required_languages()

    def run(example: Example) -> dict[str, str]:
        if example.path not in cache:
            if reason := docs_examples.missing(example):
                if example.language in required:
                    var = docs_examples.REQUIRE_VAR
                    pytest.fail(
                        f"{example.name}: {reason} ({var} has {example.language})", pytrace=False
                    )
                pytest.skip(f"{example.name}: {reason}")
            try:
                cache[example.path] = docs_examples.produce(example)
            except docs_examples.ExampleError as exc:
                pytest.fail(str(exc), pytrace=False)
        return cache[example.path]

    return run
