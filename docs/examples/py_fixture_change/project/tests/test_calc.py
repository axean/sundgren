# tests/test_calc.py
import pytest
from mypackage.calc import add, divide, classify

def test_add():
    assert add(1, 2) == 3

def test_divide():
    assert divide(6, 3) == 2
    with pytest.raises(ZeroDivisionError):
        divide(1, 0)

def test_classify(samples):
    for n, expected in samples:
        assert classify(n) == expected
