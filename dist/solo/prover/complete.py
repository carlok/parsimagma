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


def checked_replay(t, steps, L, R):
    """Replay, and verify each step was actually applicable.

    `replay` only performs the replacement: it never checks that the rule's
    source side matches the subterm it is replacing. A path can therefore
    arrive at the right endpoint while containing a step that means nothing,
    and the Lean built from it will not typecheck. In Solo the judge caught
    that; in marathon there is no judge, so the check has to be here.

    Returns the endpoint, or None if any step does not apply.
    """
    for (p, tag, s) in steps:
        src, dst = (L, R) if tag == "fwd" else (R, L)
        sub = t
        for d in p:
            if sub[0] != "o":
                return None
            sub = sub[d + 1]
        if subst(src, s) != sub:
            return None
        t = replace(t, p, subst(dst, s))
    return t


def invert(steps):
    """The inverse path: same positions, opposite directions, reversed order."""
    return [(p, "bwd" if tag == "fwd" else "fwd", s)
            for (p, tag, s) in reversed(steps)]


def under(steps, at):
    return [(tuple(at) + p, tag, s) for (p, tag, s) in steps]


def apply_match(steps, s):
    """Instantiate a path by a *matching* substitution.

    `apply_subst` resolves through a triangular unifier, which is right for a
    critical pair but wrong here: a match binds `x` to a term that may itself
    contain `x`, and resolving that walks forever. A match is already flat, so
    substitute once.
    """
    return [(p, tag, {k: subst(v, s) for k, v in sub.items()})
            for (p, tag, sub) in steps]


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
                        best = (new, under(apply_match(path, s), p))
        if best is None:
            break
        t, extra = best
        steps += extra
    return t, steps


def _complete_fifo(L, R, max_size, max_rules, deadline, dedup=None):
    """The discovery-order search. Kept because it is not dominated."""
    import time
    base = (L, R, [((), "fwd", {v: ("v", v) for v in variables(L) | variables(R)})])
    rules = [base]
    dedup = dedup or canonical
    seen = {dedup(L, R)}
    queue = [(base, base)]
    i = 0
    while i < len(queue) and len(rules) < max_rules:
        if deadline is not None and time.monotonic() > deadline:
            break
        e1, e2 = queue[i]
        i += 1
        for a, b, steps in critical_pairs(e1, e2, max_size):
            key = dedup(a, b)
            if key in seen:
                continue
            seen.add(key)
            eq = (a, b, steps)
            rules.append(eq)
            for other in rules:
                queue.append((eq, other))
            if len(rules) >= max_rules:
                break
    return rules


def literal(a, b):
    """Dedup on the printed form only, keeping alpha-variants apart.

    Logically redundant, but not operationally: two variants carry different
    proof paths, and a path's unbound variables get pinned to a default before
    emission, so one variant can reach the goal where the other cannot. At
    least one problem in the set is solved only under this key.
    """
    ka, kb = show(a), show(b)
    return (ka, kb) if ka <= kb else (kb, ka)


def canonical(a, b):
    """A key that collapses equations identical up to variable renaming, and up
    to which side is written first. Without it a tenth to a fifth of every
    budget goes on storing variants of what is already there."""
    m, ctr = {}, [0]

    def go(t):
        if t[0] == "v":
            if t[1] not in m:
                m[t[1]] = "v%d" % ctr[0]
                ctr[0] += 1
            return ("v", m[t[1]])
        if t[0] == "c":
            return t
        return ("o", go(t[1]), go(t[2]))

    ka, kb = show(go(a)), show(go(b))
    return (ka, kb) if ka <= kb else (kb, ka)


def complete(L, R, max_size=15, max_rules=1200, deadline=None, order="weight", dedup=None):
    """Grow a set of proof-carrying consequences of L = R.

    `order="weight"` selects the smallest equation next. That is the whole
    difference from a plain queue: overlapping in discovery order spends the
    budget deepening one branch, and what the goal usually needs is the compact
    consequences. Measured on the problems that defeated the queue, the size-15
    ceiling held 168 equations against 6 at size 2.

    `order="fifo"` keeps the discovery order. It is not strictly worse — it
    reaches derivations the weighted search never gets to, and at least one
    problem in the set is solved only by it. Try weight first and fall back.
    """
    import heapq, time
    dedup = dedup or canonical
    if order == "fifo":
        return _complete_fifo(L, R, max_size, max_rules, deadline, dedup)
    base = (L, R, [((), "fwd", {v: ("v", v) for v in variables(L) | variables(R)})])
    rules = [base]
    seen = {dedup(L, R)}
    queue = [(size(L) + size(R), 0, 0)]
    serial = 0
    processed = []
    while queue and len(rules) < max_rules:
        if deadline is not None and time.monotonic() > deadline:
            break
        _, _, i = heapq.heappop(queue)
        e1 = rules[i]
        for e2 in processed + [e1]:
            for a, b, steps in critical_pairs(e1, e2, max_size):
                key = dedup(a, b)
                if key in seen:
                    continue
                seen.add(key)
                rules.append((a, b, steps))
                serial += 1
                heapq.heappush(queue, (size(a) + size(b), serial, len(rules) - 1))
                if len(rules) >= max_rules:
                    break
            if len(rules) >= max_rules:
                break
        processed.append(e1)
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


def rule_steps(t, rules, max_size, cap, default=None):
    """One-step rewrites of t by any derived equation, with the proof spliced in.

    The side being introduced must be fully bound by the match — inventing a
    term for an unbound variable is what made the naive goal search explode.
    But a *path* may mention variables the match never sees, and those are
    genuinely arbitrary: the law holds for every value. Refusing them, as an
    earlier version did, threw away usable lemmas — including `x ◇ y = x`,
    which is the whole proof for one of these problems. Pin them instead, the
    way `join_by_instance` already does.
    """
    n = 0
    for (a, b, steps) in rules:
        for lhs, rhs, path in ((a, b, steps), (b, a, invert(steps))):
            for p, sub in positions(t):
                s = match(lhs, sub, None)
                if s is None:
                    continue
                if set(variables(rhs)) - set(s):
                    continue                    # would have to invent the result
                new = replace(t, p, subst(rhs, s))
                if size(new) > max_size:
                    continue
                free = set()
                for (_, _, ss) in path:
                    for v in ss.values():
                        free |= variables(v)
                s2 = dict(s)
                for v in free - set(s2):
                    s2[v] = default if default is not None else ("c", "x")
                concrete = [(pp, tag, {k: subst(v, s2) for k, v in ss.items()})
                            for (pp, tag, ss) in path]
                if any(variables(v) for (_, _, ss) in concrete for v in ss.values()):
                    continue
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
                for new, sub_path in rule_steps(t, rules, max_size, cap,
                                                default=GL if GL[0] == 'c' else None):
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


def join_by_pair(rules, GL, GR, L, R, deadline=None):
    """Combine two derived equations into one the goal can match.

    Almost every consequence of `x = C[x,..]` keeps a bare variable on one side,
    so `join_by_instance` — which matches a single equation against the goal —
    cannot see a goal whose two sides are both compound. But `v = T1` and
    `v = T2` together give `T1 = T2`, and that family can. The proof is the
    first path run backwards followed by the second.
    """
    import time
    halves = []
    for (a, b, steps) in rules:
        for lhs, rhs, path in ((a, b, steps), (b, a, invert(steps))):
            if lhs[0] == "v":
                halves.append((lhs[1], rhs, path))
    # only the halves that can supply the goal's left side are worth pairing
    left = [(v, T, p, s) for (v, T, p) in halves
            for s in [match(T, GL, None)] if s is not None]
    if not left:
        return None
    for (v1, T1, p1, s1) in left:
        if deadline is not None and time.monotonic() > deadline:
            return None
        for (v2, T2, p2) in halves:
            ren = {k: rename(x, "@") for k, x in [("_", ("v", v2))]}
            sub = {v2 + "@": ("v", v1)}
            T2r = subst(rename(T2, "@"), sub)
            s = match(T2r, GR, dict(s1))
            if s is None:
                continue
            p2r = [(p, tag, {k: subst(rename(x, "@"), sub) for k, x in ss.items()})
                   for (p, tag, ss) in p2]
            path = invert(p1) + p2r
            free = set()
            for (_, _, ss) in path:
                for x in ss.values():
                    free |= variables(x)
            s = dict(s)
            for x in free - set(s):
                s[x] = GL if GL[0] == "c" else ("c", "x")
            concrete = [(p, tag, {k: subst(x, s) for k, x in ss.items()})
                        for (p, tag, ss) in path]
            if any(variables(x) for (_, _, ss) in concrete for x in ss.values()):
                continue
            if replay(GL, concrete, L, R) == GR:
                return concrete
    return None


def shorten(path, start, L, R):
    """Cut loops out of a derivation.

    Splicing two paths together routinely produces a chain that visits the same
    term twice; everything between is a detour. If terms[i] == terms[j] for
    j > i, the steps i..j-1 can go: whatever follows applies to the same term
    either way. It matters because the judge caps a certificate at 100,000
    bytes, and a spliced proof can run to 170 steps.
    """
    steps = list(path)
    terms = [start]
    for st in steps:
        terms.append(replay(terms[-1], [st], L, R))
    i = 0
    while i < len(terms):
        key = show(terms[i])
        last = i
        for j in range(len(terms) - 1, i, -1):
            if show(terms[j]) == key:
                last = j
                break
        if last > i:
            del terms[i + 1:last + 1]
            del steps[i:last]
        i += 1
    return steps


def interreduce(rules, L, R, max_size, window=120):
    """Normalise every derived equation against the smallest of the set.

    This is where a lemma like `x ◇ y = x` actually appears: not as a critical
    pair, but as what a critical pair becomes once its sides are reduced. The
    normalisation steps are spliced into the stored path, so the result is
    still proof-carrying.
    """
    small = sorted(rules, key=lambda e: size(e[0]) + size(e[1]))[:window]
    out = []
    for (a, b, steps) in rules:
        try:
            a2, sa = normalise(a, small, L, R, max_size)
            b2, sb = normalise(b, small, L, R, max_size)
        except Exception:
            continue
        if a2 == b2:
            continue
        path = invert(sa) + steps + sb
        if replay(a2, path, L, R) == b2:
            out.append((a2, b2, path))
    return out


def join_by_normalising(rules, GL, GR, L, R, max_size=25, rounds=8):
    """Reduce both sides of the goal with the derived equations and see if they
    meet.

    Cheaper and stronger than searching over the goal: when the theory collapses
    to something like left projection, the goal's big side simply reduces to its
    small one, and breadth-first rewriting never gets there because each rule
    application drags a long proof behind it.
    """
    a, sa = normalise(GL, rules, L, R, max_size, rounds=rounds)
    b, sb = normalise(GR, rules, L, R, max_size, rounds=rounds)
    if a != b:
        return None
    path = sa + invert(sb)
    return path if replay(GL, path, L, R) == GR else None


def normalising_route(rules, GL, GR, L, R, max_size=25, rounds=8):
    """Same as `join_by_normalising`, but keep the rule applications intact.

    Returning the flattened h-steps loses the fact that a goal is often one
    derived lemma applied three times. Inlined, that is hundreds of steps over
    huge terms — 108 KB against a 100 KB cap on one problem here. Kept as
    (equation, position, direction, substitution) the same proof states the
    lemma once and applies it, which is both smaller and what a person would
    write.

    Returns (uses, lemmas) or None, where each use names the lemma it applies.
    """
    def reduce_side(t):
        uses = []
        for _ in range(rounds):
            best = None
            for i, (el, er, es) in enumerate(rules):
                for src, dst, flip in ((el, er, False), (er, el, True)):
                    for p, sub in positions(t):
                        s = match(src, sub, None)
                        if s is None or set(variables(dst)) - set(s):
                            continue
                        new = replace(t, p, subst(dst, s))
                        if size(new) < size(t) and (best is None or size(new) < size(best[0])):
                            best = (new, i, p, flip, s)
            if best is None:
                break
            t, i, p, flip, s = best
            uses.append((i, p, flip, s))
        return t, uses

    a, ua = reduce_side(GL)
    b, ub = reduce_side(GR)
    if a != b:
        return None
    return ua, ub
