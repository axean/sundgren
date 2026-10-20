# tests/test_calc.py
import pytest
from shed.calc import add, divide, classify

def test_add():
    assert add(1, 2) == 3

def test_divide():
    assert divide(6, 3) == 2
    with pytest.raises(ZeroDivisionError):
        divide(1, 0)

@pytest.mark.parametrize("n,expected", [(-1, "neg"), (0, "zero"), (5, "pos")])
def test_classify(n, expected):
    assert classify(n) == expected
