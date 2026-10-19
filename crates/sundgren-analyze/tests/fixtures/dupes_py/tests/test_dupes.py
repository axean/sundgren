from dupes.geometry import clamp
from dupes.util import limit


def test_clamp() -> None:
    assert clamp(-1, 0, 5) == 0
    assert clamp(9, 0, 5) == 5
    assert clamp(3, 0, 5) == 3


def test_limit() -> None:
    assert limit(3, 0, 5) == 3
