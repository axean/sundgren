# tests/conftest.py
import pytest

@pytest.fixture
def samples():
    return [(-1, "neg"), (0, "zero"), (5, "pos")]
