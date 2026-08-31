"""Equational proof search for one-law magma theories.

The hypothesis is a single equation L = R, universally quantified. The goal is
another equation, whose variables become Skolem constants. We look for a
rewrite sequence joining the two goal sides.

The laws here all have the shape `x = C[x, ...]`: the right side is bigger and
carries variables the left side does not. So the two directions behave very
differently.

  contracting (R -> L)   a match determines every variable. Deterministic,
                         shrinks the term, always worth doing.
  expanding   (L -> R)   the variables of R not in L are unconstrained, so one
                         match yields |pool|^k successors. Branching, but the
                         only way to introduce structure the goal needs.

Search is bidirectional BFS with contracting steps applied eagerly and
expanding steps rationed, meeting in the middle.
"""

import re
from itertools import product

# ── terms ────────────────────────────────────────────────────────────
# ('v', name)  variable (of the hypothesis; instantiable)
# ('c', name)  constant (a Skolemised goal variable; rigid)
# ('o', l, r)  l ◇ r


def parse(text, varnames):
    """Parse `x * (y * z)` into a term. Names in varnames become variables."""
    toks = re.findall(r"[A-Za-z_][A-Za-z_0-9]*|[()*◇]", text)
    pos = 0

    def atom():
        nonlocal pos
        if toks[pos] == "(":
            pos += 1
            t = expr()
            assert toks[pos] == ")", f"unbalanced in {text!r}"
            pos += 1
            return t
        name = toks[pos]
        pos += 1
        return ("v", name) if name in varnames else ("c", name)

    def expr():
        nonlocal pos
        t = atom()
        while pos < len(toks) and toks[pos] in "*◇":
            pos += 1
            t = ("o", t, atom())
        return t

    t = expr()
    assert pos == len(toks), f"trailing input in {text!r}"
    return t


def show(t):
    if t[0] in ("v", "c"):
        return t[1]
    l = show(t[1])
    r = show(t[2])
    if t[1][0] == "o":
        l = f"({l})"
    if t[2][0] == "o":
        r = f"({r})"
    return f"{l} ◇ {r}"


def size(t):
    return 1 if t[0] in ("v", "c") else size(t[1]) + size(t[2])


def variables(t, acc=None):
    acc = set() if acc is None else acc
    if t[0] == "v":
        acc.add(t[1])
    elif t[0] == "o":
        variables(t[1], acc)
        variables(t[2], acc)
    return acc


def subst(t, s):
    if t[0] == "v":
        return s.get(t[1], t)
    if t[0] == "c":
        return t
    return ("o", subst(t[1], s), subst(t[2], s))


def match(pat, t, s=None):
    """One-way matching: find s with subst(pat, s) == t. Constants are rigid."""
    s = {} if s is None else s
    if pat[0] == "v":
        prev = s.get(pat[1])
        if prev is None:
            s = dict(s)
            s[pat[1]] = t
            return s
        return s if prev == t else None
    if pat[0] == "c":
        return s if t == pat else None
    if t[0] != "o":
        return None
    s = match(pat[1], t[1], s)
    return match(pat[2], t[2], s) if s is not None else None


def positions(t, p=()):
    yield p, t
    if t[0] == "o":
        yield from positions(t[1], p + (0,))
        yield from positions(t[2], p + (1,))


def replace(t, p, new):
    if not p:
        return new
    if p[0] == 0:
        return ("o", replace(t[1], p[1:], new), t[2])
    return ("o", t[1], replace(t[2], p[1:], new))


# ── rewriting ────────────────────────────────────────────────────────

def steps(t, lhs, rhs, pool, max_size, expanding_cap):
    """Every one-step rewrite of `t` under lhs = rhs, both directions.

    Yields (new_term, position, direction, substitution). `direction` is
    'fwd' for lhs -> rhs and 'bwd' for rhs -> lhs.
    """
    for src, dst, tag in ((lhs, rhs, "fwd"), (rhs, lhs, "bwd")):
        extra = sorted(variables(dst) - variables(src))
        for p, sub in positions(t):
            s = match(src, sub, None)
            if s is None:
                continue
            if not extra:
                new = replace(t, p, subst(dst, s))
                if size(new) <= max_size:
                    yield new, p, tag, s
                continue
            # `dst` introduces variables `src` did not bind: enumerate them.
            n = 0
            for combo in product(pool, repeat=len(extra)):
                s2 = dict(s)
                s2.update(zip(extra, combo))
                new = replace(t, p, subst(dst, s2))
                if size(new) <= max_size:
                    yield new, p, tag, s2
                    n += 1
                    if n >= expanding_cap:
                        break


def search(lhs, rhs, goal_l, goal_r, pool, max_size=15,
           expanding_cap=24, max_expansions=3, max_nodes=200000, deadline=None):
    """Cost-bounded bidirectional search.

    Cost is the number of size-increasing steps taken. Contractions are free,
    so the queue drains every shrinking rewrite before spending an expansion.
    That matters because for a law `x = C[x,..]` the shrinking direction is
    nearly deterministic while the growing one branches on every unbound
    variable of C.
    """
    import heapq, time
    if goal_l == goal_r:
        return []
    fwd = {goal_l: (None, None, None, None, 0)}
    bwd = {goal_r: (None, None, None, None, 0)}
    # (cost, size, serial, term, side)  — side 0 = from goal_l, 1 = from goal_r
    q = [(0, size(goal_l), 0, goal_l, 0), (0, size(goal_r), 1, goal_r, 1)]
    heapq.heapify(q)
    serial = 2
    seen = 2

    def path_from(table, t):
        out = []
        while table[t][0] is not None:
            par, p, tag, s, _ = table[t]
            out.append((t, p, tag, s))
            t = par
        out.reverse()
        return out

    def join(meet):
        forward = path_from(fwd, meet)
        back = path_from(bwd, meet)
        inverted = []
        for (child, p, tag, s) in reversed(back):
            parent = bwd[child][0]
            inverted.append((parent, p, "bwd" if tag == "fwd" else "fwd", s))
        return forward + inverted

    while q:
        if deadline is not None and time.monotonic() > deadline:
            return None
        cost, _, _, t, side = heapq.heappop(q)
        table, other = (fwd, bwd) if side == 0 else (bwd, fwd)
        if table[t][4] < cost:
            continue
        for new, p, tag, s in steps(t, lhs, rhs, pool, max_size, expanding_cap):
            grew = size(new) > size(t)
            c2 = cost + (1 if grew else 0)
            if c2 > max_expansions:
                continue
            old = table.get(new)
            if old is not None and old[4] <= c2:
                continue
            table[new] = (t, p, tag, s, c2)
            seen += 1
            if seen > max_nodes:
                return None
            if new in other:
                return join(new)
            serial += 1
            heapq.heappush(q, (c2, size(new), serial, new, side))
    return None
