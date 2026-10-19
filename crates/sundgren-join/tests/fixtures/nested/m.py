def outer(x):
    def inner(y):
        return y + 1
    f, g = (lambda: 1), (lambda: 2)
    return inner(x) + f()
