import numpy as np


def vec(x, y):
    return np.array([x, y], dtype=float)


def norm(v):
    n = np.linalg.norm(v)
    return v / n if n != 0 else v


def reflect(v, n):
    return v - 2 * np.dot(v, n) * n
