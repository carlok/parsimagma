"""Proof-carrying completion for a one-law magma theory.

Rewriting the goal directly fails on laws of the shape `x = C[x,..]`: the
shrinking direction rarely matches and the growing one branches on every
unbound variable of C. What works is the same thing a human does — derive
consequences of the law first, then rewrite with those.

Every equation carries the sequence of primitive h-steps that proves it, so a
derived lemma is never an assumption. When the goal is finally joined, the
lemma paths are spliced in and what comes out is one flat chain of h-steps,
which is what the judge elaborates.
"""

import itertools
from eqprove import (parse, show, size, variables, subst, match,
                     positions, replace)


# ── unification ──────────────────────────────────────────────────────

def occurs(v, t, s):
    """Occurs check *through* the substitution — checking the raw term is not
    enough, since one of its variables may already be bound to something that
    contains v, and binding then builds a cyclic term."""
    t = walk(t, s)
    if t[0] == "v":
        return t[1] == v
    if t[0] == "c":
        return False
    return occurs(v, t[1], s) or occurs(v, t[2], s)


def unify(a, b, s=None):
    s = {} if s is None else s
    a, b = walk(a, s), walk(b, s)
    if a == b:
        return s
    if a[0] == "v":
        if occurs(a[1], b, s):
            return None
        s = dict(s); s[a[1]] = b; return s
    if b[0] == "v":
        if occurs(b[1], a, s):
            return None
        s = dict(s); s[b[1]] = a; return s
    if a[0] == "c" or b[0] == "c":
        return None
    s = unify(a[1], b[1], s)
    return unify(a[2], b[2], s) if s is not None else None


def walk(t, s):
    while t[0] == "v" and t[1] in s:
        t = s[t[1]]
    return t


def resolve(t, s):
    t = walk(t, s)
    if t[0] == "o":
        return ("o", resolve(t[1], s), resolve(t[2], s))
    return t


def rename(t, tag):
    if t[0] == "v":
        return ("v", t[1] + tag)
    if t[0] == "c":
        return t
    return ("o", rename(t[1], tag), rename(t[2], tag))


# ── proof-carrying equations ─────────────────────────────────────────
# A step is (pos, tag, subst): rewrite at `pos` with the hypothesis, `tag` in
# {'fwd','bwd'}. An equation is (lhs, rhs, steps) with the steps taking lhs to
# rhs when replayed.

def replay(t, steps, L, R):
    for (p, tag, s) in steps:
        src, dst = (L, R) if tag == "fwd" else (R, L)
        t = replace(t, p, subst(dst, s))
    return t


def invert(steps):
    """The inverse path: same positions, opposite directions, reversed order."""
    return [(p, "bwd" if tag == "fwd" else "fwd", s)
            for (p, tag, s) in reversed(steps)]


def under(steps, prefix):
    return [(tuple(prefix) + p, tag, s) for (p, tag, s) in steps]


def apply_subst(steps, sigma):
    out = []
    for (p, tag, s) in steps:
        out.append((p, tag, {k: resolve(v, sigma) if v[0] != "c" else v
                             for k, v in s.items()}))
    return out


# ── critical pairs ───────────────────────────────────────────────────

def orientations(eq):
    """An equation used as a rewrite rule, each way round."""
    l, r, steps = eq
    yield l, r, steps
    yield r, l, invert(steps)


def critical_pairs(e1, e2, max_size):
    """Overlap e2's left side into a non-variable subterm of e1's left side.

    Yields (a, b, steps) where steps replays a to b. Both come from rewriting
    the same overlap term two different ways, so the pair is a consequence of
    the two inputs and carries their proofs spliced together.
    """
    for l1, r1, s1 in orientations(e1):
        for l2raw, r2raw, s2raw in orientations(e2):
            l2 = rename(l2raw, "#")
            r2 = rename(r2raw, "#")
            # Rename the *values* only. The keys are the hypothesis's own
            # variables and index into L/R at replay time; renaming them would
            # make `subst` miss and the path stop replaying.
            s2 = [(p, tag, {k: rename(v, "#") for k, v in s.items()})
                  for (p, tag, s) in s2raw]
            for p, sub in positions(l1):
                if sub[0] != "o":          # overlapping at a variable is vacuous
                    continue
                sigma = unify(sub, l2)
                if sigma is None:
                    continue
                top = resolve(l1, sigma)
                a = resolve(r1, sigma)
                b = replace(top, p, resolve(r2, sigma))
                if a == b or size(a) > max_size or size(b) > max_size:
                    continue
                steps = invert(apply_subst(s1, sigma)) + under(apply_subst(s2, sigma), p)
                yield a, b, steps


def normalise(t, rules, L, R, max_size, rounds=60):
    """Shrink t with any equation that reduces its size. Records the steps."""
    steps = []
    for _ in range(rounds):
        best = None
        for (el, er, es) in rules:
            for src, dst, path in ((el, er, es), (er, el, invert(es))):
                for p, sub in positions(t):
                    s = match(src, sub, None)
                    if s is None:
                        continue
                    if set(variables(dst)) - set(s):
                        continue          # would need to invent a term
                    new = replace(t, p, subst(dst, s))
                    if size(new) < size(t) and (best is None or size(new) < size(best[0])):
                        best = (new, under(apply_subst(path, s), p))
        if best is None:
            break
        t, extra = best
        steps += extra
    return t, steps


def complete(L, R, max_size=13, max_rules=140, deadline=None):
    """Grow a set of proof-carrying consequences of L = R."""
    import time
    base = (L, R, [((), "fwd", {v: ("v", v) for v in variables(L) | variables(R)})])
    rules = [base]
    seen = {(show(L), show(R)), (show(R), show(L))}
    queue = [(base, base)]
    i = 0
    while i < len(queue) and len(rules) < max_rules:
        if deadline is not None and time.monotonic() > deadline:
            break
        e1, e2 = queue[i]; i += 1
        for a, b, steps in critical_pairs(e1, e2, max_size):
            key = (show(a), show(b))
            if key in seen or (key[1], key[0]) in seen:
                continue
            seen.add(key)
            eq = (a, b, steps)
            rules.append(eq)
            for other in rules:
                queue.append((eq, other))
            if len(rules) >= max_rules:
                break
    return rules


# ── joining the goal ─────────────────────────────────────────────────

def join_by_instance(rules, GL, GR, L, R, default=None):
    """Look for a derived equation that *is* the goal under a substitution.

    These laws all read `x = C[x,..]`, so every consequence has a variable on
    one side. A goal `x = <compound>` is then discharged outright whenever some
    derived right-hand side matches the goal's compound side, with the variable
    landing on the goal's constant. No search over the goal at all — one match
    per derived equation.

    Variables the match leaves free are pinned to `default`. They are genuinely
    arbitrary (the law holds for every value), but they must become concrete:
    an unpinned variable would reach the emitter as a name Lean has never
    heard of.
    """
    if default is None:
        default = GL if GL[0] == "c" else ("c", "x")

    for (a, b, steps) in rules:
        for lhs, rhs, path in ((a, b, steps), (b, a, invert(steps))):
            s = match(lhs, GL, None)
            if s is None:
                continue
            s = match(rhs, GR, s)
            if s is None:
                continue
            free = set()
            for (_, _, sub) in path:
                for v in sub.values():
                    free |= variables(v)
            free |= variables(lhs) | variables(rhs)
            s = dict(s)
            for v in free - set(s):
                s[v] = default
            concrete = [(p, tag, {k: subst(v, s) for k, v in sub.items()})
                        for (p, tag, sub) in path]
            if any(variables(v) for (_, _, sub) in concrete for v in sub.values()):
                continue                      # a variable escaped; reject rather than emit it
            if replay(GL, concrete, L, R) == GR:
                return concrete
    return None


def rule_steps(t, rules, max_size, cap):
    """One-step rewrites of t by any derived equation, with the proof spliced in.

    A derived equation may only be used where the match binds every variable of
    the side being introduced. Inventing a term for an unbound variable is what
    made the naive goal search explode, and the derived set is large enough that
    it is not needed.
    """
    n = 0
    for (a, b, steps) in rules:
        for lhs, rhs, path in ((a, b, steps), (b, a, invert(steps))):
            for p, sub in positions(t):
                s = match(lhs, sub, None)
                if s is None:
                    continue
                free = set(variables(rhs)) - set(s)
                for (_, _, ss) in path:
                    for v in ss.values():
                        free |= variables(v)
                free -= set(s)
                if free:
                    continue
                new = replace(t, p, subst(rhs, s))
                if size(new) > max_size:
                    continue
                concrete = [(pp, tag, {k: subst(v, s) for k, v in ss.items()})
                            for (pp, tag, ss) in path]
                yield new, under(concrete, p)
                n += 1
                if n >= cap:
                    return


def join_by_rewriting(rules, GL, GR, L, R, max_size=17, cap=400,
                      max_steps=4, deadline=None):
    """Meet in the middle, rewriting the goal with the derived equations."""
    import time
    if GL == GR:
        return []
    fwd = {GL: (None, None)}
    bwd = {GR: (None, None)}
    fr, br = [GL], [GR]
    for _ in range(max_steps):
        for side, other, frontier, forward in ((fwd, bwd, fr, True), (bwd, fwd, br, False)):
            nxt = []
            for t in frontier:
                if deadline is not None and time.monotonic() > deadline:
                    return None
                for new, sub_path in rule_steps(t, rules, max_size, cap):
                    if new in side:
                        continue
                    side[new] = (t, sub_path)
                    if new in other:
                        return _splice(fwd, bwd, new)
                    nxt.append(new)
            if forward:
                fr = nxt
            else:
                br = nxt
        if not fr and not br:
            break
    return None


def _splice(fwd, bwd, meet):
    def chain(table, t):
        out = []
        while table[t][0] is not None:
            par, sub_path = table[t]
            out.append(sub_path)
            t = par
        out.reverse()
        return [st for seg in out for st in seg]
    head = chain(fwd, meet)
    tail = []
    t = meet
    segs = []
    while bwd[t][0] is not None:
        par, sub_path = bwd[t]
        segs.append(sub_path)
        t = par
    for seg in segs:
        tail += invert(seg)
    return head + tail
