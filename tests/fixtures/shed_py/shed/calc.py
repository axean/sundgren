# shed/calc.py
def add(a, b):
    return a + b

def divide(a, b):
    if b == 0:
        raise ZeroDivisionError("b is zero")
    return a / b

def classify(n):
    if n < 0:
        return "neg"
    elif n == 0:
        return "zero"
    return "pos"
