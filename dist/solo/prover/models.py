"""Finite counterexample search for magma implications.

Given `eq1` and `eq2`, look for a Cayley table on Fin n satisfying eq1 and
violating eq2. Exhaustive for n <= 3, backtracking with propagation above.

The search fills cells in a fixed order and, after every assignment, checks the
instances of eq1 that have become fully determined. That is the whole trick:
a violated instance prunes the entire subtree, so the 4^16 tables on Fin 4 are
never enumerated.
"""

import time
from itertools import product

UNKNOWN = -1


def compile_term(term, varnames):
    """Term -> a closure over (table, n, assignment) returning an element or UNKNOWN."""
    if term[0] in ("v", "c"):
        i = varnames.index(term[1])
        return lambda tbl, n, a, i=i: a[i]
    left = compile_term(term[1], varnames)
    right = compile_term(term[2], varnames)

    def ev(tbl, n, a):
        x = left(tbl, n, a)
        if x is UNKNOWN or x < 0:
            return UNKNOWN
        y = right(tbl, n, a)
        if y is UNKNOWN or y < 0:
            return UNKNOWN
        return tbl[x * n + y]
    return ev


def holds(lhs, rhs, tbl, n, nvars):
    """True when every assignment satisfies lhs = rhs. Undetermined cells fail
    closed: a law is only reported as holding on a complete table."""
    for a in product(range(n), repeat=nvars):
        u, v = lhs(tbl, n, a), rhs(tbl, n, a)
        if u < 0 or v < 0 or u != v:
            return False
    return True


def fails(lhs, rhs, tbl, n, nvars):
    for a in product(range(n), repeat=nvars):
        u, v = lhs(tbl, n, a), rhs(tbl, n, a)
        if u >= 0 and v >= 0 and u != v:
            return True
    return False


def violated(lhs, rhs, tbl, n, nvars):
    """Some fully determined instance of the law is already false."""
    for a in product(range(n), repeat=nvars):
        u, v = lhs(tbl, n, a), rhs(tbl, n, a)
        if u >= 0 and v >= 0 and u != v:
            return True
    return False


def search_size(n, h, g, deadline):
    """Fill the n x n table cell by cell, pruning on any violated instance of h."""
    hl, hr, hn = h
    gl, gr, gn = g
    tbl = [UNKNOWN] * (n * n)
    cells = n * n

    def rec(k):
        if deadline is not None and time.monotonic() > deadline:
            raise TimeoutError
        if k == cells:
            return holds(hl, hr, tbl, n, hn) and fails(gl, gr, tbl, n, gn)
        for v in range(n):
            tbl[k] = v
            if not violated(hl, hr, tbl, n, hn) and rec(k + 1):
                return True
            tbl[k] = UNKNOWN
        return False

    try:
        return tbl if rec(0) else None
    except TimeoutError:
        return None


def find_counterexample(eq1_text, eq2_text, parse_vars, parse_term,
                        sizes=(2, 3, 4, 5, 6, 7), budget=45.0):
    """Smallest n in `sizes` carrying a model of eq1 that breaks eq2."""
    t0 = time.monotonic()
    v1, v2 = parse_vars(eq1_text), parse_vars(eq2_text)
    allv = v1 + [v for v in v2 if v not in v1]
    l1, r1 = [parse_term(s.strip(), set(allv)) for s in eq1_text.split("=")]
    l2, r2 = [parse_term(s.strip(), set(allv)) for s in eq2_text.split("=")]
    h = (compile_term(l1, allv), compile_term(r1, allv), len(allv))
    g = (compile_term(l2, allv), compile_term(r2, allv), len(allv))
    for n in sizes:
        # a bigger carrier costs n^(n*n) in the worst case; give the tail of the
        # budget to the sizes that can still finish
        left = budget - (time.monotonic() - t0)
        if left <= 0.5:
            break
        tbl = search_size(n, h, g, time.monotonic() + left)
        if tbl is not None:
            return n, [tbl[i * n:(i + 1) * n] for i in range(n)]
    return None, None
