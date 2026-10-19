def limit(x, lo, hi):
    if x < lo:
        return lo
    if x > hi:
        return hi
    return x


def clamp(value, low, high):
    if value < low:
        return low
    if value > high:
        return high
    return value


def shout(text):
    words = text.split()
    return " ".join(w.upper() + "!" for w in words)


def yell(text):
    words = text.split()
    return " ".join(w.upper() + "!" for w in words)
