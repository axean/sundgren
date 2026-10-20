import pytest


def helper():
    return 1


def test_one():
    assert helper() == 1


class TestGroup:
    def test_two(self):
        assert True


@pytest.mark.parametrize("x", [1, 2])
def test_three(x):
    assert x
