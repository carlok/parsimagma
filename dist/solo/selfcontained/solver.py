"""A Stage 2 solver with nothing borrowed and nothing looked up.

No reference-solver code, no embedded tables, no LLM call. Two mechanisms,
both search:

  finite model search   fill a Cayley table cell by cell, pruning the moment a
                        determined instance of the hypothesis fails. Settles
                        every `verdict: false` problem this set contains, at
                        carrier 2 to 5.

  completion            overlap the hypothesis with itself, collect critical
                        pairs, and look for a derived equation that is the goal
                        under a substitution. Settles the `verdict: true` ones.

Every derived equation carries the sequence of hypothesis applications proving
it, so no lemma is ever assumed; paths are replayed internally before a line of
Lean is emitted, and one that does not replay is discarded rather than sent.
The proofs that come out depend on no axioms at all.
"""

import json
import os
import re
import sys
import time
from itertools import product


# ── proxy protocol ───────────────────────────────────────────────────

def read_message():
    line = sys.stdin.readline()
    if not line:
        raise SystemExit(0)
    return json.loads(line)


def send_message(msg):
    print(json.dumps(msg), flush=True)


def call_judge(verdict, code):
    send_message({"call": "judge", "verdict": verdict, "code": code})
    return read_message()


def parse_variables(text):
    seen, out = set(), []
    for v in re.findall(r"\b([a-z])\b", text):
        if v not in seen:
            seen.add(v)
            out.append(v)
    return out


# ── terms ────────────────────────────────────────────────────────────
# ('v', name)  variable (of the hypothesis; instantiable)
# ('c', name)  constant (a Skolemised goal variable; rigid)
# ('o', l, r)  l ◇ r


def pv_parse(text, varnames):
    """Parse `x * (y * z)` into a term. Names in varnames become pv_variables."""
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


def pv_show(t):
    if t[0] in ("v", "c"):
        return t[1]
    l = pv_show(t[1])
    r = pv_show(t[2])
    if t[1][0] == "o":
        l = f"({l})"
    if t[2][0] == "o":
        r = f"({r})"
    return f"{l} ◇ {r}"


def pv_size(t):
    return 1 if t[0] in ("v", "c") else pv_size(t[1]) + pv_size(t[2])


def pv_variables(t, acc=None):
    acc = set() if acc is None else acc
    if t[0] == "v":
        acc.add(t[1])
    elif t[0] == "o":
        pv_variables(t[1], acc)
        pv_variables(t[2], acc)
    return acc


def pv_subst(t, s):
    if t[0] == "v":
        return s.get(t[1], t)
    if t[0] == "c":
        return t
    return ("o", pv_subst(t[1], s), pv_subst(t[2], s))


def pv_match(pat, t, s=None):
    """One-way matching: find s with pv_subst(pat, s) == t. Constants are rigid."""
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
    s = pv_match(pat[1], t[1], s)
    return pv_match(pat[2], t[2], s) if s is not None else None


def pv_positions(t, p=()):
    yield p, t
    if t[0] == "o":
        yield from pv_positions(t[1], p + (0,))
        yield from pv_positions(t[2], p + (1,))


def pv_replace(t, p, new):
    if not p:
        return new
    if p[0] == 0:
        return ("o", pv_replace(t[1], p[1:], new), t[2])
    return ("o", t[1], pv_replace(t[2], p[1:], new))


# ── rewriting ────────────────────────────────────────────────────────

def pv_steps(t, lhs, rhs, pool, max_size, expanding_cap):
    """Every one-step rewrite of `t` pv_under lhs = rhs, both directions.

    Yields (new_term, position, direction, substitution). `direction` is
    'fwd' for lhs -> rhs and 'bwd' for rhs -> lhs.
    """
    for src, dst, tag in ((lhs, rhs, "fwd"), (rhs, lhs, "bwd")):
        extra = sorted(pv_variables(dst) - pv_variables(src))
        for p, sub in pv_positions(t):
            s = pv_match(src, sub, None)
            if s is None:
                continue
            if not extra:
                new = pv_replace(t, p, pv_subst(dst, s))
                if pv_size(new) <= max_size:
                    yield new, p, tag, s
                continue
            # `dst` introduces pv_variables `src` did not bind: enumerate them.
            n = 0
            for combo in product(pool, repeat=len(extra)):
                s2 = dict(s)
                s2.update(zip(extra, combo))
                new = pv_replace(t, p, pv_subst(dst, s2))
                if pv_size(new) <= max_size:
                    yield new, p, tag, s2
                    n += 1
                    if n >= expanding_cap:
                        break


def pv_search(lhs, rhs, goal_l, goal_r, pool, max_size=15,
           expanding_cap=24, max_expansions=3, max_nodes=200000, deadline=None):
    """Cost-bounded bidirectional pv_search.

    Cost is the number of pv_size-increasing pv_steps taken. Contractions are free,
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
    # (cost, pv_size, serial, term, side)  — side 0 = from goal_l, 1 = from goal_r
    q = [(0, pv_size(goal_l), 0, goal_l, 0), (0, pv_size(goal_r), 1, goal_r, 1)]
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
        for new, p, tag, s in pv_steps(t, lhs, rhs, pool, max_size, expanding_cap):
            grew = pv_size(new) > pv_size(t)
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
            heapq.heappush(q, (c2, pv_size(new), serial, new, side))
    return None


# ── unification ──────────────────────────────────────────────────────

def pv_occurs(v, t, s):
    """Occurs check *through* the substitution — checking the raw term is not
    enough, since one of its pv_variables may already be bound to something that
    contains v, and binding then builds a cyclic term."""
    t = pv_walk(t, s)
    if t[0] == "v":
        return t[1] == v
    if t[0] == "c":
        return False
    return pv_occurs(v, t[1], s) or pv_occurs(v, t[2], s)


def pv_unify(a, b, s=None):
    s = {} if s is None else s
    a, b = pv_walk(a, s), pv_walk(b, s)
    if a == b:
        return s
    if a[0] == "v":
        if pv_occurs(a[1], b, s):
            return None
        s = dict(s); s[a[1]] = b; return s
    if b[0] == "v":
        if pv_occurs(b[1], a, s):
            return None
        s = dict(s); s[b[1]] = a; return s
    if a[0] == "c" or b[0] == "c":
        return None
    s = pv_unify(a[1], b[1], s)
    return pv_unify(a[2], b[2], s) if s is not None else None


def pv_walk(t, s):
    while t[0] == "v" and t[1] in s:
        t = s[t[1]]
    return t


def pv_resolve(t, s):
    t = pv_walk(t, s)
    if t[0] == "o":
        return ("o", pv_resolve(t[1], s), pv_resolve(t[2], s))
    return t


def pv_rename(t, tag):
    if t[0] == "v":
        return ("v", t[1] + tag)
    if t[0] == "c":
        return t
    return ("o", pv_rename(t[1], tag), pv_rename(t[2], tag))


# ── proof-carrying equations ─────────────────────────────────────────
# A step is (pos, tag, pv_subst): rewrite at `pos` with the hypothesis, `tag` in
# {'fwd','bwd'}. An equation is (lhs, rhs, pv_steps) with the pv_steps taking lhs to
# rhs when replayed.

def pv_replay(t, pv_steps, L, R):
    for (p, tag, s) in pv_steps:
        src, dst = (L, R) if tag == "fwd" else (R, L)
        t = pv_replace(t, p, pv_subst(dst, s))
    return t


def pv_invert(pv_steps):
    """The inverse path: same pv_positions, opposite directions, reversed order."""
    return [(p, "bwd" if tag == "fwd" else "fwd", s)
            for (p, tag, s) in reversed(pv_steps)]


def pv_under(pv_steps, at):
    return [(tuple(at) + p, tag, s) for (p, tag, s) in pv_steps]


def pv_apply_match(pv_steps, s):
    """Instantiate a path by a *matching* substitution.

    `pv_apply_subst` resolves through a triangular unifier, which is right for a
    critical pair but wrong here: a pv_match binds `x` to a term that may itself
    contain `x`, and resolving that walks forever. A pv_match is already flat, so
    substitute once.
    """
    return [(p, tag, {k: pv_subst(v, s) for k, v in sub.items()})
            for (p, tag, sub) in pv_steps]


def pv_apply_subst(pv_steps, sigma):
    out = []
    for (p, tag, s) in pv_steps:
        out.append((p, tag, {k: pv_resolve(v, sigma) if v[0] != "c" else v
                             for k, v in s.items()}))
    return out


# ── critical pairs ───────────────────────────────────────────────────

def pv_orientations(eq):
    """An equation used as a rewrite rule, each way round."""
    l, r, pv_steps = eq
    yield l, r, pv_steps
    yield r, l, pv_invert(pv_steps)


def pv_critical_pairs(e1, e2, max_size):
    """Overlap e2's left side into a non-variable subterm of e1's left side.

    Yields (a, b, pv_steps) where pv_steps replays a to b. Both come from rewriting
    the same overlap term two different ways, so the pair is a consequence of
    the two inputs and carries their proofs spliced together.
    """
    for l1, r1, s1 in pv_orientations(e1):
        for l2raw, r2raw, s2raw in pv_orientations(e2):
            l2 = pv_rename(l2raw, "#")
            r2 = pv_rename(r2raw, "#")
            # Rename the *values* only. The keys are the hypothesis's own
            # pv_variables and index into L/R at pv_replay time; renaming them would
            # make `pv_subst` miss and the path stop replaying.
            s2 = [(p, tag, {k: pv_rename(v, "#") for k, v in s.items()})
                  for (p, tag, s) in s2raw]
            for p, sub in pv_positions(l1):
                if sub[0] != "o":          # overlapping at a variable is vacuous
                    continue
                sigma = pv_unify(sub, l2)
                if sigma is None:
                    continue
                top = pv_resolve(l1, sigma)
                a = pv_resolve(r1, sigma)
                b = pv_replace(top, p, pv_resolve(r2, sigma))
                if a == b or pv_size(a) > max_size or pv_size(b) > max_size:
                    continue
                pv_steps = pv_invert(pv_apply_subst(s1, sigma)) + pv_under(pv_apply_subst(s2, sigma), p)
                yield a, b, pv_steps


def pv_normalise(t, rules, L, R, max_size, rounds=60):
    """Shrink t with any equation that reduces its pv_size. Records the pv_steps."""
    pv_steps = []
    for _ in range(rounds):
        best = None
        for (el, er, es) in rules:
            for src, dst, path in ((el, er, es), (er, el, pv_invert(es))):
                for p, sub in pv_positions(t):
                    s = pv_match(src, sub, None)
                    if s is None:
                        continue
                    if set(pv_variables(dst)) - set(s):
                        continue          # would need to invent a term
                    new = pv_replace(t, p, pv_subst(dst, s))
                    if pv_size(new) < pv_size(t) and (best is None or pv_size(new) < pv_size(best[0])):
                        best = (new, pv_under(pv_apply_match(path, s), p))
        if best is None:
            break
        t, extra = best
        pv_steps += extra
    return t, pv_steps


def pv__complete_fifo(L, R, max_size, max_rules, deadline, dedup=None):
    """The discovery-order pv_search. Kept because it is not dominated."""
    import time
    base = (L, R, [((), "fwd", {v: ("v", v) for v in pv_variables(L) | pv_variables(R)})])
    rules = [base]
    dedup = dedup or pv_canonical
    seen = {dedup(L, R)}
    queue = [(base, base)]
    i = 0
    while i < len(queue) and len(rules) < max_rules:
        if deadline is not None and time.monotonic() > deadline:
            break
        e1, e2 = queue[i]
        i += 1
        for a, b, pv_steps in pv_critical_pairs(e1, e2, max_size):
            key = dedup(a, b)
            if key in seen:
                continue
            seen.add(key)
            eq = (a, b, pv_steps)
            rules.append(eq)
            for other in rules:
                queue.append((eq, other))
            if len(rules) >= max_rules:
                break
    return rules


def pv_literal(a, b):
    """Dedup on the printed form only, keeping alpha-variants apart.

    Logically redundant, but not operationally: two variants carry different
    proof paths, and a path's unbound pv_variables get pinned to a default before
    emission, so one variant can reach the goal where the other cannot. At
    least one problem in the set is solved only pv_under this key.
    """
    ka, kb = pv_show(a), pv_show(b)
    return (ka, kb) if ka <= kb else (kb, ka)


def pv_canonical(a, b):
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

    ka, kb = pv_show(go(a)), pv_show(go(b))
    return (ka, kb) if ka <= kb else (kb, ka)


def pv_complete(L, R, max_size=15, max_rules=1200, deadline=None, order="weight", dedup=None):
    """Grow a set of proof-carrying consequences of L = R.

    `order="weight"` selects the smallest equation next. That is the whole
    difference from a plain queue: overlapping in discovery order spends the
    budget deepening one branch, and what the goal usually needs is the compact
    consequences. Measured on the problems that defeated the queue, the pv_size-15
    ceiling held 168 equations against 6 at pv_size 2.

    `order="fifo"` keeps the discovery order. It is not strictly worse — it
    reaches derivations the weighted pv_search never gets to, and at least one
    problem in the set is solved only by it. Try weight first and fall back.
    """
    import heapq, time
    dedup = dedup or pv_canonical
    if order == "fifo":
        return pv__complete_fifo(L, R, max_size, max_rules, deadline, dedup)
    base = (L, R, [((), "fwd", {v: ("v", v) for v in pv_variables(L) | pv_variables(R)})])
    rules = [base]
    seen = {dedup(L, R)}
    queue = [(pv_size(L) + pv_size(R), 0, 0)]
    serial = 0
    processed = []
    while queue and len(rules) < max_rules:
        if deadline is not None and time.monotonic() > deadline:
            break
        _, _, i = heapq.heappop(queue)
        e1 = rules[i]
        for e2 in processed + [e1]:
            for a, b, pv_steps in pv_critical_pairs(e1, e2, max_size):
                key = dedup(a, b)
                if key in seen:
                    continue
                seen.add(key)
                rules.append((a, b, pv_steps))
                serial += 1
                heapq.heappush(queue, (pv_size(a) + pv_size(b), serial, len(rules) - 1))
                if len(rules) >= max_rules:
                    break
            if len(rules) >= max_rules:
                break
        processed.append(e1)
    return rules


# ── joining the goal ─────────────────────────────────────────────────

def pv_join_by_instance(rules, GL, GR, L, R, default=None):
    """Look for a derived equation that *is* the goal pv_under a substitution.

    These laws all read `x = C[x,..]`, so every consequence has a variable on
    one side. A goal `x = <compound>` is then discharged outright whenever some
    derived right-hand side matches the goal's compound side, with the variable
    landing on the goal's constant. No pv_search over the goal at all — one pv_match
    per derived equation.

    Variables the pv_match leaves free are pinned to `default`. They are genuinely
    arbitrary (the law holds for every value), but they must become concrete:
    an unpinned variable would reach the emitter as a name Lean has never
    heard of.
    """
    if default is None:
        default = GL if GL[0] == "c" else ("c", "x")

    for (a, b, pv_steps) in rules:
        for lhs, rhs, path in ((a, b, pv_steps), (b, a, pv_invert(pv_steps))):
            s = pv_match(lhs, GL, None)
            if s is None:
                continue
            s = pv_match(rhs, GR, s)
            if s is None:
                continue
            free = set()
            for (_, _, sub) in path:
                for v in sub.values():
                    free |= pv_variables(v)
            free |= pv_variables(lhs) | pv_variables(rhs)
            s = dict(s)
            for v in free - set(s):
                s[v] = default
            concrete = [(p, tag, {k: pv_subst(v, s) for k, v in sub.items()})
                        for (p, tag, sub) in path]
            if any(pv_variables(v) for (_, _, sub) in concrete for v in sub.values()):
                continue                      # a variable escaped; reject rather than emit it
            if pv_replay(GL, concrete, L, R) == GR:
                return concrete
    return None


def pv_rule_steps(t, rules, max_size, cap, default=None):
    """One-step rewrites of t by any derived equation, with the proof spliced in.

    The side being introduced must be fully bound by the pv_match — inventing a
    term for an unbound variable is what made the naive goal pv_search explode.
    But a *path* may mention pv_variables the pv_match never sees, and those are
    genuinely arbitrary: the law holds for every value. Refusing them, as an
    earlier version did, threw away usable lemmas — including `x ◇ y = x`,
    which is the whole proof for one of these problems. Pin them instead, the
    way `pv_join_by_instance` already does.
    """
    n = 0
    for (a, b, pv_steps) in rules:
        for lhs, rhs, path in ((a, b, pv_steps), (b, a, pv_invert(pv_steps))):
            for p, sub in pv_positions(t):
                s = pv_match(lhs, sub, None)
                if s is None:
                    continue
                if set(pv_variables(rhs)) - set(s):
                    continue                    # would have to invent the result
                new = pv_replace(t, p, pv_subst(rhs, s))
                if pv_size(new) > max_size:
                    continue
                free = set()
                for (_, _, ss) in path:
                    for v in ss.values():
                        free |= pv_variables(v)
                s2 = dict(s)
                for v in free - set(s2):
                    s2[v] = default if default is not None else ("c", "x")
                concrete = [(pp, tag, {k: pv_subst(v, s2) for k, v in ss.items()})
                            for (pp, tag, ss) in path]
                if any(pv_variables(v) for (_, _, ss) in concrete for v in ss.values()):
                    continue
                yield new, pv_under(concrete, p)
                n += 1
                if n >= cap:
                    return


def pv_join_by_rewriting(rules, GL, GR, L, R, max_size=17, cap=400,
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
                for new, sub_path in pv_rule_steps(t, rules, max_size, cap,
                                                default=GL if GL[0] == 'c' else None):
                    if new in side:
                        continue
                    side[new] = (t, sub_path)
                    if new in other:
                        return pv__splice(fwd, bwd, new)
                    nxt.append(new)
            if forward:
                fr = nxt
            else:
                br = nxt
        if not fr and not br:
            break
    return None


def pv__splice(fwd, bwd, meet):
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
        tail += pv_invert(seg)
    return head + tail


def pv_join_by_pair(rules, GL, GR, L, R, deadline=None):
    """Combine two derived equations into one the goal can pv_match.

    Almost every consequence of `x = C[x,..]` keeps a bare variable on one side,
    so `pv_join_by_instance` — which matches a single equation against the goal —
    cannot see a goal whose two sides are both compound. But `v = T1` and
    `v = T2` together give `T1 = T2`, and that family can. The proof is the
    first path run backwards followed by the second.
    """
    import time
    halves = []
    for (a, b, pv_steps) in rules:
        for lhs, rhs, path in ((a, b, pv_steps), (b, a, pv_invert(pv_steps))):
            if lhs[0] == "v":
                halves.append((lhs[1], rhs, path))
    # only the halves that can supply the goal's left side are worth pairing
    left = [(v, T, p, s) for (v, T, p) in halves
            for s in [pv_match(T, GL, None)] if s is not None]
    if not left:
        return None
    for (v1, T1, p1, s1) in left:
        if deadline is not None and time.monotonic() > deadline:
            return None
        for (v2, T2, p2) in halves:
            ren = {k: pv_rename(x, "@") for k, x in [("_", ("v", v2))]}
            sub = {v2 + "@": ("v", v1)}
            T2r = pv_subst(pv_rename(T2, "@"), sub)
            s = pv_match(T2r, GR, dict(s1))
            if s is None:
                continue
            p2r = [(p, tag, {k: pv_subst(pv_rename(x, "@"), sub) for k, x in ss.items()})
                   for (p, tag, ss) in p2]
            path = pv_invert(p1) + p2r
            free = set()
            for (_, _, ss) in path:
                for x in ss.values():
                    free |= pv_variables(x)
            s = dict(s)
            for x in free - set(s):
                s[x] = GL if GL[0] == "c" else ("c", "x")
            concrete = [(p, tag, {k: pv_subst(x, s) for k, x in ss.items()})
                        for (p, tag, ss) in path]
            if any(pv_variables(x) for (_, _, ss) in concrete for x in ss.values()):
                continue
            if pv_replay(GL, concrete, L, R) == GR:
                return concrete
    return None


def pv_shorten(path, start, L, R):
    """Cut loops out of a derivation.

    Splicing two paths together routinely produces a chain that visits the same
    term twice; everything between is a detour. If terms[i] == terms[j] for
    j > i, the pv_steps i..j-1 can go: whatever follows applies to the same term
    either way. It matters because the judge caps a certificate at 100,000
    bytes, and a spliced proof can run to 170 pv_steps.
    """
    pv_steps = list(path)
    terms = [start]
    for st in pv_steps:
        terms.append(pv_replay(terms[-1], [st], L, R))
    i = 0
    while i < len(terms):
        key = pv_show(terms[i])
        last = i
        for j in range(len(terms) - 1, i, -1):
            if pv_show(terms[j]) == key:
                last = j
                break
        if last > i:
            del terms[i + 1:last + 1]
            del pv_steps[i:last]
        i += 1
    return pv_steps


def pv_interreduce(rules, L, R, max_size, window=120):
    """Normalise every derived equation against the smallest of the set.

    This is where a lemma like `x ◇ y = x` actually appears: not as a critical
    pair, but as what a critical pair becomes once its sides are reduced. The
    normalisation pv_steps are spliced into the stored path, so the result is
    still proof-carrying.
    """
    small = sorted(rules, key=lambda e: pv_size(e[0]) + pv_size(e[1]))[:window]
    out = []
    for (a, b, pv_steps) in rules:
        try:
            a2, sa = pv_normalise(a, small, L, R, max_size)
            b2, sb = pv_normalise(b, small, L, R, max_size)
        except Exception:
            continue
        if a2 == b2:
            continue
        path = pv_invert(sa) + pv_steps + sb
        if pv_replay(a2, path, L, R) == b2:
            out.append((a2, b2, path))
    return out


def pv_join_by_normalising(rules, GL, GR, L, R, max_size=25, rounds=8):
    """Reduce both sides of the goal with the derived equations and see if they
    meet.

    Cheaper and stronger than searching over the goal: when the theory collapses
    to something like left projection, the goal's big side simply reduces to its
    small one, and breadth-first rewriting never gets there because each rule
    application drags a long proof behind it.
    """
    a, sa = pv_normalise(GL, rules, L, R, max_size, rounds=rounds)
    b, sb = pv_normalise(GR, rules, L, R, max_size, rounds=rounds)
    if a != b:
        return None
    path = sa + pv_invert(sb)
    return path if pv_replay(GL, path, L, R) == GR else None


def pv_normalising_route(rules, GL, GR, L, R, max_size=25, rounds=8):
    """Same as `pv_join_by_normalising`, but keep the rule applications intact.

    Returning the flattened h-pv_steps loses the fact that a goal is often one
    derived lemma applied three times. Inlined, that is hundreds of pv_steps over
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
                    for p, sub in pv_positions(t):
                        s = pv_match(src, sub, None)
                        if s is None or set(pv_variables(dst)) - set(s):
                            continue
                        new = pv_replace(t, p, pv_subst(dst, s))
                        if pv_size(new) < pv_size(t) and (best is None or pv_size(new) < pv_size(best[0])):
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

PV_UNKNOWN = -1

PV_STATS = {"instances_woken": 0, "node_evals": 0, "cell_trials": 0,
         "nodes": 0, "propagations": 0, "conflicts": 0}


def pv_flatten(lhs, rhs, varnames):
    """Both sides into one straight-line program, lhs nodes first.

    prog[i] = (0, varindex, 0) | (1, childA, childB), topologically ordered.
    """
    prog, memo = [], {}

    def go(t):
        key = t
        if key in memo:
            return memo[key]
        if t[0] in ("v", "c"):
            idx = len(prog)
            prog.append((0, varnames.index(t[1]), 0))
        else:
            a = go(t[1])
            b = go(t[2])
            idx = len(prog)
            prog.append((1, a, b))
        memo[key] = idx
        return idx

    lroot = go(lhs)
    rroot = go(rhs)
    return prog, lroot, rroot


def pv_square_order(n):
    """Cells grouped by max(i, j): keeps the set of elements mentioned by the
    partial table a short prefix for as long as possible, which is what makes
    the isomorphism cut bite."""
    out = []
    for d in range(n):
        out.append(d * n + d)
        for j in range(d):
            out.append(d * n + j)
        for i in range(d):
            out.append(i * n + d)
    return out


def pv_search_size(n, prog, lroot, rroot, nvars, gprog, glroot, grroot, gnvars,
                deadline, stats=False, iso=True):
    ncells = n * n
    order = pv_square_order(n) if iso else list(range(ncells))
    nprog = len(prog)
    tbl = [PV_UNKNOWN] * ncells
    insts = list(product(range(n), repeat=nvars))
    ninst = len(insts)
    val = [0] * nprog
    wcell = [-1] * ninst
    state = [0] * ninst          # 0 = still undetermined, 1 = retired
    watch = [[] for _ in range(ncells)]
    trail = []
    pending = [ninst]
    mx = [-1]              # largest element occurring anywhere in the partial table

    def setcell(c, v):
        tbl[c] = v
        trail.append((2, c, mx[0]))
        i, j = divmod(c, n)
        m = mx[0]
        if i > m: m = i
        if j > m: m = j
        if v > m: m = v
        mx[0] = m

    def evaluate(i):
        """(-1, 0) when fully determined, else (node index, cell) it blocks on."""
        base = insts[i]
        for idx in range(nprog):
            k, a, b = prog[idx]
            if k == 0:
                val[idx] = base[a]
            else:
                c = val[a] * n + val[b]
                t = tbl[c]
                if t < 0:
                    return idx, c
                val[idx] = t
        return -1, 0

    if stats:
        _ev = evaluate
        def evaluate(i, _ev=_ev):
            PV_STATS["node_evals"] += nprog
            return _ev(i)

    def process(c, queue):
        lst = watch[c]
        j = 0
        while j < len(lst):
            i = lst[j]; j += 1
            if state[i] or wcell[i] != c:
                continue
            if stats: PV_STATS["instances_woken"] += 1
            blk, cc = evaluate(i)
            if blk < 0:
                if val[lroot] != val[rroot]:
                    if stats: PV_STATS["conflicts"] += 1
                    return False
                state[i] = 1; wcell[i] = -1
                trail.append((1, i, c)); pending[0] -= 1
            elif blk == rroot and lroot < rroot:
                # evaluation reached the last node, so every earlier node -- the
                # other side's root included -- already has a value: this cell
                # has exactly one admissible value rather than n.
                setcell(cc, val[lroot]); queue.append(cc)
                if stats: PV_STATS["propagations"] += 1
                state[i] = 1; wcell[i] = -1
                trail.append((1, i, c)); pending[0] -= 1
            elif blk == lroot and rroot < lroot:
                setcell(cc, val[rroot]); queue.append(cc)
                if stats: PV_STATS["propagations"] += 1
                state[i] = 1; wcell[i] = -1
                trail.append((1, i, c)); pending[0] -= 1
            else:
                wcell[i] = cc; watch[cc].append(i)
                trail.append((0, i, c, cc))
        return True

    def propagate(queue):
        while queue:
            c = queue.pop()
            if not process(c, queue):
                return False
        return True

    def undo(mark):
        while len(trail) > mark:
            e = trail.pop()
            if e[0] == 0:
                _, i, old, new = e
                watch[new].pop(); wcell[i] = old
            elif e[0] == 1:
                _, i, c = e
                state[i] = 0; wcell[i] = c; pending[0] += 1
            else:
                tbl[e[1]] = PV_UNKNOWN; mx[0] = e[2]

    # park every instance on the cell it first blocks on
    for i in range(ninst):
        blk, cc = evaluate(i)
        if blk < 0:
            if val[lroot] != val[rroot]:
                return None
            state[i] = 1; pending[0] -= 1
        else:
            wcell[i] = cc; watch[cc].append(i)

    gn = len(gprog)
    gval = [0] * gn
    ginsts = list(product(range(n), repeat=gnvars))

    def goal_fails():
        for base in ginsts:
            for idx in range(gn):
                k, a, b = gprog[idx]
                gval[idx] = base[a] if k == 0 else tbl[gval[a] * n + gval[b]]
            if gval[glroot] != gval[grroot]:
                return True
        return False

    def rec(start):
        if stats: PV_STATS["nodes"] += 1
        if deadline is not None and time.monotonic() > deadline:
            raise TimeoutError
        p = start
        while p < ncells and tbl[order[p]] != PV_UNKNOWN:
            p += 1
        if p == ncells:
            return pending[0] == 0 and goal_fails()
        k = order[p]
        i, j = divmod(k, n)
        # mx[0] tracks every element the partial table mentions, including cells
        # written by propagation ahead of the cursor.  Elements above me+1 occur
        # nowhere, so they are interchangeable and one representative suffices.
        me = mx[0]
        if i > me: me = i
        if j > me: me = j
        top = min(n - 1, me + 1) if iso else n - 1
        for v in range(top + 1):
            if stats: PV_STATS["cell_trials"] += 1
            mark = len(trail)
            setcell(k, v)
            if propagate([k]) and rec(p + 1):
                return True
            undo(mark)
        return False

    try:
        return tbl[:] if rec(0) else None
    except TimeoutError:
        return None


def pv_find_counterexample(eq1_text, eq2_text, parse_vars, parse_term,
                        sizes=(2, 3, 4, 5, 6, 7), budget=45.0, stats=False,
                        per_size=None, iso=True):
    """Smallest n in `sizes` carrying a model of eq1 that breaks eq2.

    `per_size` gives each carrier its own budget instead of sharing one pool,
    so a hard small carrier cannot starve the larger ones.
    """
    t0 = time.monotonic()
    v1, v2 = parse_vars(eq1_text), parse_vars(eq2_text)
    allv = v1 + [v for v in v2 if v not in v1]
    l1, r1 = [parse_term(s.strip(), set(allv)) for s in eq1_text.split("=")]
    l2, r2 = [parse_term(s.strip(), set(allv)) for s in eq2_text.split("=")]
    prog, lroot, rroot = pv_flatten(l1, r1, allv)
    gprog, glroot, grroot = pv_flatten(l2, r2, allv)
    for n in sizes:
        left = budget - (time.monotonic() - t0)
        if left <= 0.5:
            break
        # per_size caps what one carrier may spend; the total budget still
        # binds, otherwise a long `sizes` list overruns the caller's deadline.
        dl = time.monotonic() + (min(left, per_size) if per_size is not None else left)
        tbl = pv_search_size(n, prog, lroot, rroot, len(allv),
                          gprog, glroot, grroot, len(allv), dl, stats, iso)
        if tbl is not None:
            return n, [tbl[i * n:(i + 1) * n] for i in range(n)]
    return None, None

# ── certificates ─────────────────────────────────────────────────────

TABLE_MAX_CARRIER = 10          # finOpTable reads one character per entry


def false_code(n, table):
    """A counterexample certificate.

    `decideFin!` evaluates the law at every point of the carrier, and past
    carrier 5 that overruns Lean's default recursion limit: the judge answers
    "maximum recursion depth has been reached" rather than rejecting the model,
    so a perfectly good counterexample is thrown away. Raising the limit is a
    `set_option` -- not banned, not an axiom, and the kernel still checks the
    same term.
    """
    rows = "[" + ", ".join("[" + ", ".join(str(v) for v in r) + "]" for r in table) + "]"
    opt = "set_option maxRecDepth 8000 in\n" if n >= 6 else ""
    return (
        "import JudgeProblem\n"
        "import JudgeDecide.DecideBang\n"
        "import JudgeFinOp.MemoFinOp\n"
        "open MemoFinOp\n\n"
        + opt +
        "def submission : Goal := by\n"
        "  let m : Magma (Fin %d) := {\n"
        "    op := finOpTable \"%s\"\n"
        "  }\n"
        "  refine ⟨Fin %d, m, ?_⟩\n"
        "  decideFin!\n" % (n, rows, n)
    )


def true_code(body):
    return ("import JudgeProblem\n\n"
            "def submission : Goal := by\n"
            "  intro G _ h\n"
            + "\n".join("  " + l for l in body.split("\n")) + "\n")


def context_lambda(term, p):
    if not p:
        return None
    return "fun t => " + pv_show(pv_replace(term, p, ("c", "\x00"))).replace("\x00", "t")


def step_proof(cur, p, tag, s, hv):
    if [v for v in hv if v not in s]:
        return None
    args = " ".join(pv_show(s[v]) if s[v][0] != "o" else "(%s)" % pv_show(s[v]) for v in hv)
    base = ("h %s" % args) if args else "h"
    if tag == "bwd":
        base = "(%s).symm" % base
    lam = context_lambda(cur, p)
    return base if lam is None else "congrArg (%s) (%s)" % (lam, base)


def emit(path, GL, L, R, hv, gv):
    """One `calc` line per rewrite step."""
    if not path:
        return "intro %s\nrfl" % " ".join(gv)
    lines = ["intro %s" % " ".join(gv), "calc"]
    cur, first = GL, True
    for (pos, tag, s) in path:
        proof = step_proof(cur, pos, tag, s, hv)
        if proof is None:
            return None
        nxt = pv_replay(cur, [(pos, tag, s)], L, R)
        lines.append("  %s = %s := %s" % (pv_show(cur) if first else "_", pv_show(nxt), proof))
        cur, first = nxt, False
    return "\n".join(lines)


def emit_collapse(path, L, R, hv, gv, GL, GR):
    """When the law forces a singleton, every goal follows from one lemma."""
    inner = emit(path, ("c", "a"), L, R, hv, ["a", "b"])
    if inner is None:
        return None
    inner = inner.split("\n", 1)[1] if inner.startswith("intro") else inner
    body = ["have collapse : ∀ a b : G, a = b := by", "  intro a b"]
    body += ["  " + l for l in inner.split("\n")]
    body.append("intro %s" % " ".join(gv))
    body.append("exact collapse (%s) (%s)" % (pv_show(GL), pv_show(GR)))
    return "\n".join(body)


# ── the two searches, wired to the goal ──────────────────────────────

def prep(eq1_text, eq2_text):
    hv, gv = parse_variables(eq1_text), parse_variables(eq2_text)
    L, R = [pv_parse(s.strip(), set(hv)) for s in eq1_text.split("=")]
    GL, GR = [pv_parse(s.strip(), set()) for s in eq2_text.split("=")]
    return L, R, GL, GR, hv, gv


def find_model(eq1_text, eq2_text, sizes, budget):
    n, table = pv_find_counterexample(eq1_text, eq2_text, parse_variables, pv_parse,
                                      sizes=sizes, budget=budget,
                                      per_size=max(3.0, budget / max(2, len(sizes))))
    if n is None or n > TABLE_MAX_CARRIER:
        return None
    return false_code(n, table)


MAX_CERT_BYTES = 96_000        # the judge caps a certificate at 100,000


MAX_CERT_BYTES = 96_000        # the judge caps a certificate at 100,000

# Two completion settings, tried in order. Smallest-first with variants folded
# together is much the stronger of the two, but it is not dominant: discovery
# order with variants kept apart reaches derivations it never gets to, and one
# problem in the set is solved only that way. Both are cheap, so try both.
SEARCH_ORDERS = (("weight", "canonical"), ("fifo", "literal"))


def prove(eq1_text, eq2_text, budget, rewriting=False):
    """A Lean proof body, or None."""
    L, R, GL, GR, hv, gv = prep(eq1_text, eq2_text)
    slice_ = budget / len(SEARCH_ORDERS)
    for order, dedup in SEARCH_ORDERS:
        t0 = time.monotonic()
        dd = pv_canonical if dedup == "canonical" else pv_literal
        rules = pv_complete(L, R, max_size=15 if not rewriting else 13,
                            max_rules=2000 if not rewriting else 700,
                            deadline=t0 + slice_ * (0.6 if not rewriting else 0.3),
                            order=order, dedup=dd)
        if rewriting:
            path = pv_join_by_rewriting(rules, GL, GR, L, R, max_size=17, cap=300,
                                        max_steps=3, deadline=t0 + slice_)
        else:
            path = (pv_join_by_instance(rules, GL, GR, L, R)
                    or pv_join_by_pair(rules, GL, GR, L, R, deadline=t0 + slice_))
        if path is None:
            continue
        # Splicing two derivations tends to revisit terms; drop the detours
        # before they cost bytes the certificate does not have.
        path = pv_shorten(path, GL, L, R)
        if pv_replay(GL, path, L, R) != GR:
            continue
        body = emit(path, GL, L, R, hv, gv)
        if body is not None and len(body.encode()) <= MAX_CERT_BYTES:
            return body
    return None


def prove_by_normalising(eq1_text, eq2_text, budget):
    """Interreduce the derived set, then reduce both goal sides to a common form.

    Kept as its own stage rather than folded into `prove`: interreducing a
    1,500-equation set is seconds of work on its own, and sharing `prove`'s
    slice starved it of the time it needs.

    This is where a lemma like `x ◇ y = x` shows up — not as a critical pair,
    but as what one becomes once its sides are reduced. The proof is then that
    lemma applied a few times, so it is emitted as a `have` and applied rather
    than inlined: on one problem here that is the difference between 108,238
    bytes and 4,567, against a 100,000-byte cap.
    """
    L, R, GL, GR, hv, gv = prep(eq1_text, eq2_text)
    t0 = time.monotonic()
    rules = pv_complete(L, R, max_size=15, max_rules=1500,
                        deadline=t0 + budget * 0.35)
    rules = rules + pv_interreduce(rules, L, R, 15)
    route = pv_normalising_route(rules, GL, GR, L, R)
    if route is None:
        return None
    body = emit_with_lemmas(route[0], route[1], rules, GL, GR, L, R, hv, gv)
    if body is None or len(body.encode()) > MAX_CERT_BYTES:
        return None
    return body


def prove_collapse(eq1_text, eq2_text, budget):
    """Prove the law forces a singleton, then read the goal off that."""
    L, R, GL, GR, hv, gv = prep(eq1_text, eq2_text)
    SL, SR = ("c", "a"), ("c", "b")
    t0 = time.monotonic()
    rules = pv_complete(L, R, max_size=15, max_rules=1200, deadline=t0 + budget * 0.7)
    path = pv_join_by_instance(rules, SL, SR, L, R)
    if path is None or pv_replay(SL, path, L, R) != SR:
        return None
    return emit_collapse(path, L, R, hv, gv, GL, GR)


def attempt(fn, *a):
    """Every strategy is optional: a bug in one must not end the run."""
    try:
        return fn(*a)
    except Exception:
        return None


# Marathon has no judge in the loop: answers are appended to a file and scored
# after the process exits. `submit` is the single place that knows which track
# is running, so the ladder below is written once.
#
# This matters more than it looks. `read_message` raises SystemExit on EOF, and
# marathon launches the solver with stdin on /dev/null — so one stray
# `call_judge` would raise SystemExit(0), sail through every `except Exception`
# in the file, exit the process with status 0, and turn every remaining problem
# into `not_attempted` with nothing in any log to say why.
MARATHON = {"on": False, "answer": None}


def submit(verdict, code):
    """True if the answer is (or is assumed) accepted."""
    if MARATHON["on"]:
        MARATHON["answer"] = (verdict, code)
        return True
    return call_judge(verdict, code).get("status") == "accepted"


def main():
    main_body(read_message()["problem"])


def main_body(problem):
    eq1 = problem["equation1"].replace("*", "◇")
    eq2 = problem["equation2"].replace("*", "◇")

    # A counterexample at carrier 2 or 3 is nearly free, so look before proving.
    code = attempt(find_model, eq1, eq2, (2, 3), 5.0)
    if code and submit("false", code):
        return

    # Proving is cheap; the wider carriers are not. Exhaust every proof route
    # before paying for a table search that a true implication can never end.
    for args in ((prove, eq1, eq2, 14.0),
                 (prove_collapse, eq1, eq2, 10.0),
                 (prove_by_normalising, eq1, eq2, 60.0),
                 (prove, eq1, eq2, 22.0, True)):
        body = attempt(*args)
        if body and submit("true", true_code(body)):
            return

    body = attempt(prove_superposition, problem, eq1, eq2, 30.0)
    if body and submit("true", true_code_op(body)):
        return

    code = attempt(find_model, eq1, eq2, (4, 5, 6, 7), 240.0)
    if code:
        submit("false", code)



def emit_with_lemmas(uses_l, uses_r, rules, GL, GR, L, R, hv, gv):
    """A proof that states each derived lemma once and then applies it.

    `emit` inlines every hypothesis application, which is right for a short
    chain and ruinous when the same lemma is used three times over large terms.
    Here each lemma becomes a `have` proved by its own calc chain, and the goal
    is a calc chain over lemma applications.
    """
    if not uses_l and not uses_r:
        # Both goal sides already coincide. Emitting a `calc` with no lines is
        # not Lean; it only ever got past here because the Solo judge rejected
        # it and the ladder moved on. With no judge — marathon — it would be
        # the final answer for that problem.
        return "intro %s\nrfl" % " ".join(gv) if gv else "rfl"
    used = sorted({i for (i, _, _, _) in uses_l + uses_r})
    lines, names = [], {}
    for k, i in enumerate(used):
        a, b, path = rules[i]
        lv = sorted(pv_variables(a) | pv_variables(b))
        sub = {v: ("c", "a%d" % j) for j, v in enumerate(lv)}
        # The path may mention variables neither side of the lemma binds. They
        # are arbitrary — the law holds for every value — but they must become
        # something Lean has heard of, so pin them to the lemma's first binder.
        loose = set()
        for (_, _, ss) in path:
            for v in ss.values():
                loose |= pv_variables(v)
        pin = ("c", "a0") if lv else ("c", "_a")
        for v in loose - set(sub):
            sub[v] = pin
        aa, bb = pv_subst(a, sub), pv_subst(b, sub)
        cpath = [(p, tag, {kk: pv_subst(v, sub) for kk, v in ss.items()}) for (p, tag, ss) in path]
        if any(pv_variables(v) for (_, _, ss) in cpath for v in ss.values()):
            return None
        cpath = pv_shorten(cpath, aa, L, R)
        if pv_replay(aa, cpath, L, R) != bb:
            return None
        inner = emit(cpath, aa, L, R, hv, [])
        if inner is None:
            return None
        inner = inner.split("\n", 1)[1] if inner.startswith("intro") else inner
        nm = "lem%d" % k
        names[i] = (nm, lv)
        args = " ".join("a%d" % j for j in range(len(lv)))
        head = "have %s : ∀ %s : G, %s = %s := by" % (
            nm, " ".join("a%d" % j for j in range(len(lv))) or "_a", pv_show(aa), pv_show(bb))
        lines.append(head)
        lines.append("  intro %s" % (args or "_a"))
        lines += ["  " + l for l in inner.split("\n")]

    body = ["intro %s" % " ".join(gv), "calc"]
    cur, first = GL, True
    for (i, p, flip, s) in uses_l:
        a, b, _ = rules[i]
        src, dst = (b, a) if flip else (a, b)
        nm, lv = names[i]
        nxt = pv_replace(cur, p, pv_subst(dst, s))
        app = "%s %s" % (nm, " ".join(_arg(s.get(v)) for v in lv))
        if flip:
            app = "(%s).symm" % app
        lam = context_lambda(cur, p)
        proof = app if lam is None else "congrArg (%s) (%s)" % (lam, app)
        body.append("  %s = %s := %s" % (pv_show(cur) if first else "_", pv_show(nxt), proof))
        cur, first = nxt, False
    tail = []
    for (i, p, flip, s) in reversed(uses_r):
        a, b, _ = rules[i]
        src, dst = (b, a) if flip else (a, b)
        nm, lv = names[i]
        prev = pv_replace(cur, p, pv_subst(src, s))
        app = "%s %s" % (nm, " ".join(_arg(s.get(v)) for v in lv))
        if not flip:
            app = "(%s).symm" % app
        lam = context_lambda(cur, p)
        proof = app if lam is None else "congrArg (%s) (%s)" % (lam, app)
        tail.append("  %s = %s := %s" % (pv_show(cur) if first else "_", pv_show(prev), proof))
        cur, first = prev, False
    body += tail
    if cur != GR:
        return None
    return "\n".join(lines + body)


def _arg(t):
    if t is None:
        return "x"
    return pv_show(t) if t[0] != "o" else "(%s)" % pv_show(t)


from collections import Counter as _Counter, deque as _deque
import heapq as _heapq
import itertools as _itertools

def sb_parse(text, varnames):
    """Parse a parenthesised magma term.

    Names in ``varnames`` are sb_variables; every other name is a rigid constant.
    ``*`` and ``◇`` are accepted as the binary operation.
    """

    toks = re.findall(r"[A-Za-z_][A-Za-z_0-9#@]*|[()*◇]", text)
    pos = 0

    def atom():
        nonlocal pos
        if pos >= len(toks):
            raise ValueError("unexpected end of term: %r" % text)
        if toks[pos] == "(":
            pos += 1
            term = expr()
            if pos >= len(toks) or toks[pos] != ")":
                raise ValueError("unbalanced term: %r" % text)
            pos += 1
            return term
        name = toks[pos]
        pos += 1
        return ("v", name) if name in varnames else ("c", name)

    def expr():
        nonlocal pos
        term = atom()
        while pos < len(toks) and toks[pos] in ("*", "◇"):
            pos += 1
            term = ("o", term, atom())
        return term

    result = expr()
    if pos != len(toks):
        raise ValueError("trailing input in term: %r" % text)
    return result

def sb_show(term):
    if term[0] in ("v", "c"):
        return term[1]
    left = sb_show(term[1])
    right = sb_show(term[2])
    if term[1][0] == "o":
        left = "(" + left + ")"
    if term[2][0] == "o":
        right = "(" + right + ")"
    return left + " ◇ " + right

def sb_size(term):
    """Reference-compatible sb_size: number of leaves, not AST nodes."""

    if term[0] in ("v", "c"):
        return 1
    return sb_size(term[1]) + sb_size(term[2])

def sb_term_weight(term):
    """KBO weight.  Every variable, constant, and operation has weight one."""

    if term[0] in ("v", "c"):
        return 1
    return 1 + sb_term_weight(term[1]) + sb_term_weight(term[2])

def sb_variables(term, acc=None):
    acc = set() if acc is None else acc
    if term[0] == "v":
        acc.add(term[1])
    elif term[0] == "o":
        sb_variables(term[1], acc)
        sb_variables(term[2], acc)
    return acc

def sb_variable_counts(term, out=None):
    out = _Counter() if out is None else out
    if term[0] == "v":
        out[term[1]] += 1
    elif term[0] == "o":
        sb_variable_counts(term[1], out)
        sb_variable_counts(term[2], out)
    return out

def sb_subst(term, sigma):
    """Simultaneous, one-level substitution.

    A replacement is returned as supplied rather than substituted again.  This
    is important for matching: a pattern variable named ``x`` may legitimately
    sb_match a target term containing a target variable also named ``x``.
    """

    if term[0] == "v":
        return sigma.get(term[1], term)
    if term[0] == "c":
        return term
    return ("o", sb_subst(term[1], sigma), sb_subst(term[2], sigma))

def sb_match(pattern, term, sigma=None):
    """One-way matching: return sigma with ``pattern sigma == term``."""

    sigma = {} if sigma is None else sigma
    if pattern[0] == "v":
        old = sigma.get(pattern[1])
        if old is None:
            result = dict(sigma)
            result[pattern[1]] = term
            return result
        return sigma if old == term else None
    if pattern[0] == "c":
        return sigma if pattern == term else None
    if term[0] != "o":
        return None
    sigma = sb_match(pattern[1], term[1], sigma)
    return sb_match(pattern[2], term[2], sigma) if sigma is not None else None

def sb_positions(term, position=()):
    yield position, term
    if term[0] == "o":
        yield from sb_positions(term[1], position + (0,))
        yield from sb_positions(term[2], position + (1,))

def sb_subterm(term, position):
    for direction in position:
        if term[0] != "o":
            raise IndexError("position %r does not exist" % (position,))
        term = term[direction + 1]
    return term

def sb_replace(term, position, new):
    if not position:
        return new
    if term[0] != "o":
        raise IndexError("position %r does not exist" % (position,))
    if position[0] == 0:
        return ("o", sb_replace(term[1], position[1:], new), term[2])
    return ("o", term[1], sb_replace(term[2], position[1:], new))

def sb_rename(term, suffix):
    if term[0] == "v":
        return ("v", term[1] + suffix)
    if term[0] == "c":
        return term
    return ("o", sb_rename(term[1], suffix), sb_rename(term[2], suffix))

def sb_walk(term, sigma):
    seen = set()
    while term[0] == "v" and term[1] in sigma:
        if term[1] in seen:
            raise ValueError("cyclic substitution")
        seen.add(term[1])
        term = sigma[term[1]]
    return term

def sb_occurs(name, term, sigma):
    term = sb_walk(term, sigma)
    if term[0] == "v":
        return term[1] == name
    if term[0] == "c":
        return False
    return sb_occurs(name, term[1], sigma) or sb_occurs(name, term[2], sigma)

def sb_unify(left, right, sigma=None):
    sigma = {} if sigma is None else sigma
    left, right = sb_walk(left, sigma), sb_walk(right, sigma)
    if left == right:
        return sigma
    if left[0] == "v":
        if sb_occurs(left[1], right, sigma):
            return None
        result = dict(sigma)
        result[left[1]] = right
        return result
    if right[0] == "v":
        if sb_occurs(right[1], left, sigma):
            return None
        result = dict(sigma)
        result[right[1]] = left
        return result
    if left[0] == "c" or right[0] == "c":
        return None
    sigma = sb_unify(left[1], right[1], sigma)
    return sb_unify(left[2], right[2], sigma) if sigma is not None else None

def sb_resolve(term, sigma):
    term = sb_walk(term, sigma)
    if term[0] == "o":
        return ("o", sb_resolve(term[1], sigma), sb_resolve(term[2], sigma))
    return term

def sb__root_precedence(term):
    # Variables have no precedence.  The operation is above constants; names
    # give a deterministic total precedence between rigid constants.
    if term[0] == "c":
        return (0, term[1])
    if term[0] == "o":
        return (1, "o")
    return (-1, term[1])

def sb_kbo_gt(left, right):
    """Strict KBO with unit weights and lexicographic status for ``◇``.

    The variable condition is checked before weights.  The result is a partial
    order on nonground terms: for example ``x ◇ y`` and ``y ◇ x`` are
    incomparable.  That is deliberate; both sides remain eligible in ordered
    superposition when neither is smaller.
    """

    if left == right:
        return False
    lc = sb_variable_counts(left)
    rc = sb_variable_counts(right)
    if any(lc[name] < count for name, count in rc.items()):
        return False
    lw, rw = sb_term_weight(left), sb_term_weight(right)
    if lw != rw:
        return lw > rw
    if left[0] == "v":
        return False
    if right[0] == "v":
        # With positive unit weights this is normally excluded by weight or
        # the variable condition, but retaining the standard case is harmless.
        return True
    lp, rp = sb__root_precedence(left), sb__root_precedence(right)
    if lp != rp:
        return lp > rp
    if left[0] == "c":
        return left[1] > right[1]
    if left[1] != right[1]:
        return sb_kbo_gt(left[1], right[1])
    return sb_kbo_gt(left[2], right[2])

def sb_kbo_cmp(left, right):
    if left == right:
        return 0
    if sb_kbo_gt(left, right):
        return 1
    if sb_kbo_gt(right, left):
        return -1
    return None

class sb_ProofError(ValueError):
    pass

def sb_replay(term, steps, hypothesis_left, hypothesis_right):
    """Strictly sb_replay primitive hypothesis steps.

    Unlike a blind replacement routine, this checks that the instantiated
    source is exactly the sb_subterm at the recorded position.  Search bugs are
    therefore rejected at admission instead of becoming false certificates.
    """

    for index, (position, direction, sigma) in enumerate(steps):
        source, target = ((hypothesis_left, hypothesis_right)
                          if direction == "fwd"
                          else (hypothesis_right, hypothesis_left))
        if direction not in ("fwd", "bwd"):
            raise sb_ProofError("step %d has invalid direction %r" %
                             (index, direction))
        try:
            actual = sb_subterm(term, position)
        except IndexError as exc:
            raise sb_ProofError("step %d: %s" % (index, exc)) from exc
        expected = sb_subst(source, sigma)
        if actual != expected:
            raise sb_ProofError(
                "step %d at %r: expected %s, found %s" %
                (index, position, sb_show(expected), sb_show(actual)))
        term = sb_replace(term, position, sb_subst(target, sigma))
    return term

def sb_proof_replays(equation, hypothesis_left, hypothesis_right):
    left, right, steps = equation
    try:
        return sb_replay(left, steps, hypothesis_left, hypothesis_right) == right
    except (sb_ProofError, IndexError, RecursionError, ValueError):
        return False

def sb_invert(steps):
    return [(position, "bwd" if direction == "fwd" else "fwd", sigma)
            for position, direction, sigma in reversed(steps)]

def sb_under(steps, at):
    return [(tuple(at) + position, direction, sigma)
            for position, direction, sigma in steps]

def sb_apply_match(steps, sigma):
    return [(position, direction,
             {name: sb_subst(value, sigma) for name, value in step_sigma.items()})
            for position, direction, step_sigma in steps]

def sb_apply_subst(steps, sigma):
    return [(position, direction,
             {name: sb_resolve(value, sigma)
              for name, value in step_sigma.items()})
            for position, direction, step_sigma in steps]

def sb_orientations(equation):
    left, right, steps = equation
    yield left, right, steps
    yield right, left, sb_invert(steps)

def sb_base_equation(left, right):
    names = sb_variables(left) | sb_variables(right)
    identity = {name: ("v", name) for name in names}
    return left, right, [((), "fwd", identity)]

def sb_oriented_rule_info(equation):
    left, right, steps = equation
    if sb_kbo_gt(left, right):
        return left, right, steps, False
    if sb_kbo_gt(right, left):
        return right, left, sb_invert(steps), True
    return None

def sb_oriented_rule(equation):
    result = sb_oriented_rule_info(equation)
    return result[:3] if result is not None else None

def sb_ordered_superpositions(target_equation, source_equation, max_size,
                           fresh_suffix="#", target_id=None, source_id=None):
    """Generate ordered superpositions of ``source`` into ``target``.

    Both equation sides are considered.  After applying the MGU, the chosen
    source and target sides must not be smaller than their opposite sides.
    Overlaps at sb_variables are excluded.  Every result is a primitive proof
    from the target branch back through the overlap and down the source branch.
    """

    for target_reversed, (target_left, target_right, target_path) in enumerate(
            sb_orientations(target_equation)):
        for source_reversed, (source_left_raw, source_right_raw, source_path_raw) in enumerate(
                sb_orientations(source_equation)):
            source_left = sb_rename(source_left_raw, fresh_suffix)
            source_right = sb_rename(source_right_raw, fresh_suffix)
            source_path = [
                (position, direction,
                 {name: sb_rename(value, fresh_suffix)
                  for name, value in sigma.items()})
                for position, direction, sigma in source_path_raw
            ]
            for position, overlap in sb_positions(target_left):
                if overlap[0] != "o":
                    continue
                sigma = sb_unify(overlap, source_left)
                if sigma is None:
                    continue
                tl = sb_resolve(target_left, sigma)
                tr = sb_resolve(target_right, sigma)
                sl = sb_resolve(source_left, sigma)
                sr = sb_resolve(source_right, sigma)
                # Ordered-superposition maximality restrictions.
                if sb_kbo_gt(tr, tl) or sb_kbo_gt(sr, sl):
                    continue
                left = tr
                right = sb_replace(tl, position, sr)
                if left == right:
                    continue
                if sb_size(left) > max_size or sb_size(right) > max_size:
                    continue
                path = (sb_invert(sb_apply_subst(target_path, sigma)) +
                        sb_under(sb_apply_subst(source_path, sigma), position))
                equation = (left, right, path)
                if target_id is None or source_id is None:
                    yield equation
                    continue
                target_sigma = {
                    name: sb_resolve(("v", name), sigma)
                    for name in sb_variables(target_equation[0]) |
                    sb_variables(target_equation[1])
                }
                source_sigma = {
                    name: sb_resolve(sb_rename(("v", name), fresh_suffix), sigma)
                    for name in sb_variables(source_equation[0]) |
                    sb_variables(source_equation[1])
                }
                route = [
                    (target_id, (), not bool(target_reversed), target_sigma),
                    (source_id, position, bool(source_reversed), source_sigma),
                ]
                yield sb__Candidate(equation, route)

def sb__rewrite_candidates(term, rules, max_size):
    for rule_index, equation in enumerate(rules):
        oriented = sb_oriented_rule(equation)
        if oriented is None:
            continue
        source, target, path = oriented
        for position, actual in sb_positions(term):
            sigma = sb_match(source, actual, None)
            if sigma is None:
                continue
            if sb_variables(target) - set(sigma):
                continue
            result = sb_replace(term, position, sb_subst(target, sigma))
            if result == term or sb_size(result) > max_size:
                continue
            # This assertion also catches implementation errors in KBO.
            if not sb_kbo_gt(term, result):
                continue
            concrete = sb_under(sb_apply_match(path, sigma), position)
            priority = (sb_size(result), sb_term_weight(result), sb_show(result),
                        rule_index, len(position), position)
            yield priority, result, concrete

def sb_normalise(term, rules, hypothesis_left, hypothesis_right, max_size,
              rounds=256):
    """KBO-sb_normalise ``term`` and return its primitive proof path."""

    path = []
    for _ in range(rounds):
        best = None
        for candidate in sb__rewrite_candidates(term, rules, max_size):
            if best is None or candidate[0] < best[0]:
                best = candidate
        if best is None:
            break
        _, result, extra = best
        # Validate each demodulation while its context is still small and local.
        try:
            if sb_replay(term, extra, hypothesis_left, hypothesis_right) != result:
                raise sb_ProofError("demodulation path ends at the wrong term")
        except sb_ProofError:
            break
        term = result
        path.extend(extra)
    return term, path

def sb_simplify_equation(equation, rules, hypothesis_left, hypothesis_right,
                      max_size):
    left, right, path = equation
    new_left, left_path = sb_normalise(
        left, rules, hypothesis_left, hypothesis_right, max_size)
    new_right, right_path = sb_normalise(
        right, rules, hypothesis_left, hypothesis_right, max_size)
    new_path = sb_invert(left_path) + path + right_path
    return new_left, new_right, new_path

def sb_equation_subsumes(general, specific):
    """Whether ``specific`` is an instance/variant of ``general``."""

    gl, gr, _ = general
    sl, sr, _ = specific
    for left, right in ((sl, sr), (sr, sl)):
        sigma = sb_match(gl, left, None)
        if sigma is not None and sb_match(gr, right, sigma) is not None:
            return True
    return False

def sb_canonical(left, right):
    """Equation key modulo side exchange and a consistent variable renaming."""

    def one(a, b):
        names = {}

        def visit(term):
            if term[0] == "v":
                if term[1] not in names:
                    names[term[1]] = "v%d" % len(names)
                return ("v", names[term[1]])
            if term[0] == "c":
                return term
            return ("o", visit(term[1]), visit(term[2]))

        return sb_show(visit(a)), sb_show(visit(b))

    direct = one(left, right)
    reverse = one(right, left)
    return min(direct, reverse)

class sb__Candidate:
    __slots__ = ("equation", "route")

    def __init__(self, equation, route):
        self.equation = equation
        self.route = route

def sb_invert_route(route):
    return [(record_id, position, not flipped, sigma)
            for record_id, position, flipped, sigma in reversed(route)]

def sb_replay_route(term, route, records):
    """Replay a compressed path whose steps apply previously proved lemmas."""

    for index, (record_id, position, flipped, sigma) in enumerate(route):
        if record_id < 0 or record_id >= len(records):
            raise sb_ProofError("route step %d refers to missing lemma %d" %
                             (index, record_id))
        left, right, _ = records[record_id].equation
        source, target = (right, left) if flipped else (left, right)
        actual = sb_subterm(term, position)
        expected = sb_subst(source, sigma)
        if actual != expected:
            raise sb_ProofError(
                "route step %d at %r: expected %s, found %s" %
                (index, position, sb_show(expected), sb_show(actual)))
        term = sb_replace(term, position, sb_subst(target, sigma))
    return term

def sb_unfold_route(route, records):
    """Expand a compressed lemma route to primitive hypothesis steps."""

    output = []
    for record_id, position, flipped, sigma in route:
        equation = records[record_id].equation
        path = sb_invert(equation[2]) if flipped else equation[2]
        output.extend(sb_under(sb_apply_match(path, sigma), position))
    return output

def sb__rewrite_candidates_records(term, records, max_size):
    """Demodulation candidates carrying both flat and compressed proofs."""

    for record in records:
        info = sb_oriented_rule_info(record.equation)
        if info is None:
            continue
        source, target, path, flipped = info
        for position, actual in sb_positions(term):
            sigma = sb_match(source, actual, None)
            if sigma is None or sb_variables(target) - set(sigma):
                continue
            result = sb_replace(term, position, sb_subst(target, sigma))
            if result == term or sb_size(result) > max_size:
                continue
            if not sb_kbo_gt(term, result):
                continue
            concrete = sb_under(sb_apply_match(path, sigma), position)
            use_sigma = {
                name: sigma.get(name, ("v", name))
                for name in sb_variables(record.equation[0]) |
                sb_variables(record.equation[1])
            }
            use = (record.identifier, position, flipped, use_sigma)
            priority = (sb_size(result), sb_term_weight(result), sb_show(result),
                        record.age, len(position), position)
            yield priority, result, concrete, use

def sb_normalise_records(term, records, hypothesis_left, hypothesis_right,
                      max_size, rounds=256):
    path, route = [], []
    records = list(records)
    for _ in range(rounds):
        best = None
        for candidate in sb__rewrite_candidates_records(term, records, max_size):
            if best is None or candidate[0] < best[0]:
                best = candidate
        if best is None:
            break
        _, result, extra, use = best
        if sb_replay(term, extra, hypothesis_left, hypothesis_right) != result:
            raise sb_ProofError("compressed demodulation has an invalid flat path")
        if sb_replay_route(term, [use], sb_records_by_id(records)) != result:
            raise sb_ProofError("compressed demodulation has an invalid lemma use")
        term = result
        path.extend(extra)
        route.append(use)
    return term, path, route

def sb_records_by_id(records):
    """Return an id-indexed record list, preserving holes when passed a subset."""

    records = list(records)
    if not records:
        return []
    highest = max(record.identifier for record in records)
    indexed = [None] * (highest + 1)
    for record in records:
        indexed[record.identifier] = record
    return indexed

class sb__Record:
    __slots__ = ("identifier", "equation", "route", "age", "live",
                 "active", "selected")

    def __init__(self, identifier, equation, route, age):
        self.identifier = identifier
        self.equation = equation
        self.route = route
        self.age = age
        self.live = True
        self.active = False
        self.selected = False

LAST_STATS = {}

LAST_SATURATION = None

class sb_Saturation:
    """A proof-checking given-clause loop with forward/backward simplification."""

    def __init__(self, hypothesis_left, hypothesis_right, max_size=15,
                 max_rules=1200, deadline=None, order="weight", dedup=None,
                 age_ratio=6):
        self.left = hypothesis_left
        self.right = hypothesis_right
        self.max_size = max_size
        self.max_rules = max_rules
        self.deadline = deadline
        self.order = order
        self.dedup = dedup or sb_canonical
        self.age_ratio = max(2, age_ratio)
        self.records = []
        self.admitted = 0
        self.selections = 0
        self.fresh = 0
        self.stats = {
            "paths_produced": 0,
            "paths_replayed": 0,
            "path_failures": 0,
            "dag_paths_replayed": 0,
            "dag_path_failures": 0,
            "raw_superpositions": 0,
            "forward_simplifications": 0,
            "backward_simplifications": 0,
            "forward_subsumed": 0,
            "backward_subsumed": 0,
            "tautologies": 0,
            "admitted": 0,
            "selected": 0,
            "live": 0,
        }

    def expired(self):
        return self.deadline is not None and time.monotonic() > self.deadline

    def live_records(self, exclude=None):
        for record in self.records:
            if record.live and record is not exclude:
                yield record

    def live_equations(self, exclude=None):
        return [record.equation for record in self.live_records(exclude)]

    def _track_path(self, equation):
        self.stats["paths_produced"] += 1
        if sb_proof_replays(equation, self.left, self.right):
            self.stats["paths_replayed"] += 1
            return True
        self.stats["path_failures"] += 1
        return False

    def _track_candidate(self, candidate):
        if not self._track_path(candidate.equation):
            return False
        if candidate.route is None:
            return True
        try:
            if sb_replay_route(candidate.equation[0], candidate.route,
                            self.records) != candidate.equation[1]:
                raise sb_ProofError("compressed path ends at the wrong term")
        except (sb_ProofError, IndexError, AttributeError):
            self.stats["dag_path_failures"] += 1
            return False
        self.stats["dag_paths_replayed"] += 1
        return True

    def _simplify_candidate(self, candidate, records):
        records = list(records)
        left, right, path = candidate.equation
        new_left, left_path, left_route = sb_normalise_records(
            left, records, self.left, self.right, self.max_size)
        new_right, right_path, right_route = sb_normalise_records(
            right, records, self.left, self.right, self.max_size)
        new_path = sb_invert(left_path) + path + right_path
        if candidate.route is None:
            new_route = None
        else:
            new_route = (sb_invert_route(left_route) + candidate.route +
                         right_route)
        return sb__Candidate((new_left, new_right, new_path), new_route)

    def deactivate(self, record):
        record.live = False
        record.active = False

    def admit(self, equation, origin="inference", route=None):
        """Simplify, proof-check, subsume, then enqueue an equation.

        Backward demodulation can create more equations, so admission drains a
        local work queue.  The return value is the first retained record, if
        any; callers do not rely on it for correctness.
        """

        initial = equation if isinstance(equation, sb__Candidate) else sb__Candidate(
            equation, route)
        work = _deque([(initial, origin)])
        first = None
        while work and self.admitted < self.max_rules and not self.expired():
            candidate, why = work.popleft()
            before = candidate.equation[:2]
            candidate = self._simplify_candidate(
                candidate, self.live_records())
            equation = candidate.equation
            if equation[:2] != before:
                self.stats["forward_simplifications"] += 1
            if not self._track_candidate(candidate):
                continue
            if equation[0] == equation[1]:
                self.stats["tautologies"] += 1
                continue
            if sb_size(equation[0]) > self.max_size or sb_size(equation[1]) > self.max_size:
                continue

            known = list(self.live_records())
            if any(sb_equation_subsumes(record.equation, equation)
                   for record in known):
                self.stats["forward_subsumed"] += 1
                continue

            # A newly found general equation makes its live instances redundant.
            for record in known:
                if record.live and sb_equation_subsumes(equation, record.equation):
                    self.deactivate(record)
                    self.stats["backward_subsumed"] += 1

            identifier = len(self.records)
            record = sb__Record(identifier, equation, candidate.route, identifier)
            self.records.append(record)
            self.admitted += 1
            self.stats["admitted"] = self.admitted
            if first is None:
                first = record

            # Backward demodulation uses only the genuinely new rule.  Any
            # simplified descendant is sent through full forward simplification
            # when its turn in this local queue arrives.
            if sb_oriented_rule(equation) is not None:
                for old in list(self.live_records(exclude=record)):
                    identity = {
                        name: ("v", name)
                        for name in sb_variables(old.equation[0]) |
                        sb_variables(old.equation[1])
                    }
                    seed = sb__Candidate(
                        old.equation,
                        [(old.identifier, (), False, identity)])
                    simplified = self._simplify_candidate(seed, [record])
                    if simplified.equation[:2] == old.equation[:2]:
                        continue
                    self.deactivate(old)
                    self.stats["backward_simplifications"] += 1
                    work.append((simplified, "backward"))
        return first

    def select(self):
        passive = [record for record in self.live_records()
                   if not record.selected]
        if not passive:
            return None
        choose_age = (self.order in ("fifo", "age") or
                      (self.order not in ("fifo", "age") and
                       self.selections % self.age_ratio == self.age_ratio - 1))
        if choose_age:
            chosen = min(passive, key=lambda record: record.age)
        else:
            chosen = min(
                passive,
                key=lambda record: (
                    sb_size(record.equation[0]) + sb_size(record.equation[1]),
                    sb_term_weight(record.equation[0]) + sb_term_weight(record.equation[1]),
                    len(record.equation[2]),
                    record.age,
                ),
            )
        chosen.selected = True
        chosen.active = True
        self.selections += 1
        self.stats["selected"] = self.selections
        return chosen

    def run(self):
        self.admit(sb_base_equation(self.left, self.right), origin="axiom",
                   route=None)
        while self.admitted < self.max_rules and not self.expired():
            given = self.select()
            if given is None:
                break
            active = [record for record in self.live_records()
                      if record.active]
            raw = []
            for other in active:
                if self.expired():
                    break
                self.fresh += 1
                suffix = "#%d" % self.fresh
                raw.extend(sb_ordered_superpositions(
                    given.equation, other.equation, self.max_size, suffix,
                    given.identifier, other.identifier))
                if other is not given:
                    self.fresh += 1
                    suffix = "#%d" % self.fresh
                    raw.extend(sb_ordered_superpositions(
                        other.equation, given.equation, self.max_size, suffix,
                        other.identifier, given.identifier))
            for candidate in raw:
                if self.admitted >= self.max_rules or self.expired():
                    break
                self.stats["raw_superpositions"] += 1
                # Raw paths are checked separately from their simplified form.
                if not self._track_candidate(candidate):
                    continue
                self.admit(candidate)
        result = [record.equation for record in self.live_records()]
        self.stats["live"] = len(result)
        return result

def sb_complete(left, right, max_size=15, max_rules=1200, deadline=None,
             order="weight", dedup=None):
    """Return proof-carrying consequences of ``left = right``.

    The signature intentionally follows the reference ``sb_complete.complete``.
    ``order='weight'`` uses a fair 5:1 weight/age ratio; ``fifo`` and ``age``
    select strictly by age.
    """

    global LAST_STATS, LAST_SATURATION
    saturation = sb_Saturation(left, right, max_size, max_rules, deadline,
                            order, dedup)
    result = saturation.run()
    LAST_STATS = dict(saturation.stats)
    LAST_SATURATION = saturation
    return result

def sb__path_variables(path):
    result = set()
    for _, _, sigma in path:
        for value in sigma.values():
            result |= sb_variables(value)
    return result

def sb_join_by_instance(rules, goal_left, goal_right, hypothesis_left,
                     hypothesis_right, default=None):
    if default is None:
        default = goal_left if goal_left[0] == "c" else ("c", "x")
    for equation in rules:
        for left, right, path in sb_orientations(equation):
            sigma = sb_match(left, goal_left, None)
            if sigma is None:
                continue
            sigma = sb_match(right, goal_right, sigma)
            if sigma is None:
                continue
            sigma = dict(sigma)
            free = (sb__path_variables(path) | sb_variables(left) | sb_variables(right))
            for name in free - set(sigma):
                sigma[name] = default
            concrete = [
                (position, direction,
                 {name: sb_subst(value, sigma) for name, value in step_sigma.items()})
                for position, direction, step_sigma in path
            ]
            if sb__path_variables(concrete):
                continue
            try:
                if sb_replay(goal_left, concrete, hypothesis_left,
                          hypothesis_right) == goal_right:
                    return concrete
            except sb_ProofError:
                continue
    return None

def sb_join_by_instance_records(saturation, goal_left, goal_right):
    for record in saturation.live_records():
        for reversed_side, (left, right, _) in enumerate(
                sb_orientations(record.equation)):
            sigma = sb_match(left, goal_left, None)
            if sigma is None:
                continue
            sigma = sb_match(right, goal_right, sigma)
            if sigma is None:
                continue
            use_sigma = {
                name: sigma.get(name, ("v", name))
                for name in sb_variables(record.equation[0]) |
                sb_variables(record.equation[1])
            }
            route = [(record.identifier, (), bool(reversed_side), use_sigma)]
            try:
                if sb_replay_route(goal_left, route, saturation.records) == goal_right:
                    return route
            except sb_ProofError:
                continue
    return None

def sb_join_by_normalising(rules, goal_left, goal_right, hypothesis_left,
                        hypothesis_right, max_size=25, rounds=64):
    left_nf, left_path = sb_normalise(
        goal_left, rules, hypothesis_left, hypothesis_right, max_size, rounds)
    right_nf, right_path = sb_normalise(
        goal_right, rules, hypothesis_left, hypothesis_right, max_size, rounds)
    if left_nf != right_nf:
        return None
    result = left_path + sb_invert(right_path)
    try:
        return result if sb_replay(goal_left, result, hypothesis_left,
                                hypothesis_right) == goal_right else None
    except sb_ProofError:
        return None

def sb_join_by_normalising_records(saturation, goal_left, goal_right,
                                max_size=25, rounds=64):
    records = list(saturation.live_records())
    left_nf, _, left_route = sb_normalise_records(
        goal_left, records, saturation.left, saturation.right,
        max_size, rounds)
    right_nf, _, right_route = sb_normalise_records(
        goal_right, records, saturation.left, saturation.right,
        max_size, rounds)
    if left_nf != right_nf:
        return None
    route = left_route + sb_invert_route(right_route)
    try:
        return route if sb_replay_route(goal_left, route,
                                     saturation.records) == goal_right else None
    except sb_ProofError:
        return None

def sb_rule_steps(term, rules, max_size, cap, default=None):
    count = 0
    for equation in rules:
        for left, right, path in sb_orientations(equation):
            for position, actual in sb_positions(term):
                sigma = sb_match(left, actual, None)
                if sigma is None or sb_variables(right) - set(sigma):
                    continue
                result = sb_replace(term, position, sb_subst(right, sigma))
                if sb_size(result) > max_size:
                    continue
                sigma = dict(sigma)
                for name in sb__path_variables(path) - set(sigma):
                    sigma[name] = default if default is not None else ("c", "x")
                concrete = [
                    (p, direction,
                     {name: sb_subst(value, sigma)
                      for name, value in step_sigma.items()})
                    for p, direction, step_sigma in path
                ]
                if sb__path_variables(concrete):
                    continue
                yield result, sb_under(concrete, position)
                count += 1
                if count >= cap:
                    return

def sb__splice(forward, backward, meeting):
    def chain(table, term):
        segments = []
        while table[term][0] is not None:
            parent, path = table[term]
            segments.append(path)
            term = parent
        segments.reverse()
        return [step for segment in segments for step in segment]

    head = chain(forward, meeting)
    tail = []
    term = meeting
    while backward[term][0] is not None:
        parent, path = backward[term]
        tail.extend(sb_invert(path))
        term = parent
    return head + tail

def sb_join_by_rewriting(rules, goal_left, goal_right, hypothesis_left,
                      hypothesis_right, max_size=17, cap=400, max_steps=4,
                      deadline=None):
    if goal_left == goal_right:
        return []
    forward = {goal_left: (None, None)}
    backward = {goal_right: (None, None)}
    front, back = [goal_left], [goal_right]
    for _ in range(max_steps):
        for table, other, frontier, is_forward in (
                (forward, backward, front, True),
                (backward, forward, back, False)):
            following = []
            for term in frontier:
                if deadline is not None and time.monotonic() > deadline:
                    return None
                for result, path in sb_rule_steps(
                        term, rules, max_size, cap,
                        default=goal_left if goal_left[0] == "c" else None):
                    if result in table:
                        continue
                    table[result] = (term, path)
                    if result in other:
                        candidate = sb__splice(forward, backward, result)
                        try:
                            if sb_replay(goal_left, candidate, hypothesis_left,
                                      hypothesis_right) == goal_right:
                                return candidate
                        except sb_ProofError:
                            pass
                    following.append(result)
            if is_forward:
                front = following
            else:
                back = following
        if not front and not back:
            break
    return None

def sb_join_by_pair(rules, goal_left, goal_right, hypothesis_left,
                 hypothesis_right, deadline=None):
    halves = []
    for equation in rules:
        for left, right, path in sb_orientations(equation):
            if left[0] == "v":
                halves.append((left[1], right, path))
    left_matches = [(name, term, path, sigma)
                    for name, term, path in halves
                    for sigma in [sb_match(term, goal_left, None)]
                    if sigma is not None]
    for name1, term1, path1, sigma1 in left_matches:
        if deadline is not None and time.monotonic() > deadline:
            return None
        for name2, term2, path2 in halves:
            suffix = "@pair"
            bridge = {name2 + suffix: ("v", name1)}
            term2 = sb_subst(sb_rename(term2, suffix), bridge)
            sigma = sb_match(term2, goal_right, dict(sigma1))
            if sigma is None:
                continue
            path2 = [
                (position, direction,
                 {name: sb_subst(sb_rename(value, suffix), bridge)
                  for name, value in step_sigma.items()})
                for position, direction, step_sigma in path2
            ]
            path = sb_invert(path1) + path2
            sigma = dict(sigma)
            for name in sb__path_variables(path) - set(sigma):
                sigma[name] = goal_left if goal_left[0] == "c" else ("c", "x")
            concrete = [
                (position, direction,
                 {name: sb_subst(value, sigma) for name, value in step_sigma.items()})
                for position, direction, step_sigma in path
            ]
            if sb__path_variables(concrete):
                continue
            try:
                if sb_replay(goal_left, concrete, hypothesis_left,
                          hypothesis_right) == goal_right:
                    return concrete
            except sb_ProofError:
                continue
    return None

def sb_shorten(path, start, hypothesis_left, hypothesis_right):
    steps = list(path)
    terms = [start]
    for step in steps:
        terms.append(sb_replay(terms[-1], [step], hypothesis_left,
                            hypothesis_right))
    index = 0
    while index < len(terms):
        last = index
        for other in range(len(terms) - 1, index, -1):
            if terms[other] == terms[index]:
                last = other
                break
        if last > index:
            del terms[index + 1:last + 1]
            del steps[index:last]
        index += 1
    return steps

def sb_interreduce(rules, hypothesis_left, hypothesis_right, max_size,
                window=None):
    """Compatibility helper; the main loop already interreduces eagerly."""

    selected = list(rules)
    if window is not None:
        selected = sorted(
            selected, key=lambda eq: sb_size(eq[0]) + sb_size(eq[1]))[:window]
    output = []
    for index, equation in enumerate(selected):
        others = selected[:index] + selected[index + 1:]
        simplified = sb_simplify_equation(
            equation, others, hypothesis_left, hypothesis_right, max_size)
        if simplified[0] != simplified[1] and sb_proof_replays(
                simplified, hypothesis_left, hypothesis_right):
            output.append(simplified)
    return output

def sb_lean_term(term):
    """Render without declaring notation or any parser/elaborator extension."""

    if term[0] in ("v", "c"):
        return term[1]
    return "op (%s) (%s)" % (sb_lean_term(term[1]), sb_lean_term(term[2]))

def sb__lean_arg(term):
    rendered = sb_lean_term(term)
    return rendered if term[0] != "o" else "(" + rendered + ")"

def sb__context_lambda(term, position):
    if not position:
        return None
    hole = ("c", "__hole")
    body = sb_lean_term(sb_replace(term, position, hole)).replace("__hole", "t")
    return "fun t => " + body

def sb_route_closure(route, records):
    needed = set()
    stack = [record_id for record_id, _, _, _ in route]
    while stack:
        record_id = stack.pop()
        if record_id in needed:
            continue
        needed.add(record_id)
        parent_route = records[record_id].route
        if parent_route is not None:
            stack.extend(parent for parent, _, _, _ in parent_route)
    return sorted(needed)

def sb__instantiate_route(route, own_variables, names):
    """Rename a lemma's sb_variables and pin proof-only sb_variables consistently."""

    mapping = {old: ("c", new) for old, new in zip(own_variables, names)}
    used = set()
    for _, _, _, sigma in route:
        for value in sigma.values():
            used |= sb_variables(value)
    pin = ("c", names[0])
    for loose in used - set(own_variables):
        mapping[loose] = pin
    return [
        (record_id, position, flipped,
         {name: sb_subst(value, mapping) for name, value in sigma.items()})
        for record_id, position, flipped, sigma in route
    ], mapping

def sb__lemma_proof(record_id, flipped, sigma, records, lemma_variables,
                 hypothesis_variables):
    if record_id == 0:
        name = "h"
        variables_order = hypothesis_variables
    else:
        name = "lem%d" % record_id
        variables_order = lemma_variables[record_id]
    args = []
    for variable in variables_order:
        value = sigma.get(variable)
        if value is None:
            raise sb_ProofError("lemma %d has unbound variable %s" %
                             (record_id, variable))
        args.append(sb__lean_arg(value))
    proof = name + ((" " + " ".join(args)) if args else "")
    if flipped:
        proof = "(" + proof + ").symm"
    return proof

def sb__calc_lines(start, route, records, lemma_variables,
                hypothesis_variables, indent=""):
    if not route:
        return [indent + "rfl"]
    lines = [indent + "calc"]
    current = start
    first = True
    for record_id, position, flipped, sigma in route:
        left, right, _ = records[record_id].equation
        source, target = (right, left) if flipped else (left, right)
        actual = sb_subterm(current, position)
        if actual != sb_subst(source, sigma):
            raise sb_ProofError("compressed Lean route does not sb_match its redex")
        following = sb_replace(current, position, sb_subst(target, sigma))
        proof = sb__lemma_proof(record_id, flipped, sigma, records,
                             lemma_variables, hypothesis_variables)
        context = sb__context_lambda(current, position)
        if context is not None:
            proof = "congrArg (%s) (%s)" % (context, proof)
        lhs = sb_lean_term(current) if first else "_"
        lines.append(indent + "  %s = %s := %s" %
                     (lhs, sb_lean_term(following), proof))
        current = following
        first = False
    return lines

def sb_emit_lean_dag(saturation, goal_route, goal_left, goal_right,
                  hypothesis_variables, goal_variables):
    """Emit a lemma-sharing proof body and validate every compressed route.

    Record zero is the original hypothesis and is emitted as applications of
    ``h``.  Every other record in the transitive dependency closure becomes one
    ``have``.  No macros, notation, tactics, or elaborator extensions are used.
    """

    records = saturation.records
    closure = sb_route_closure(goal_route, records)
    lemma_variables = {
        record_id: sorted(sb_variables(records[record_id].equation[0]) |
                          sb_variables(records[record_id].equation[1]))
        for record_id in closure
    }
    lines = []
    for record_id in closure:
        if record_id == 0:
            continue
        record = records[record_id]
        if record.route is None:
            raise sb_ProofError("derived lemma %d has no compressed proof" % record_id)
        own = lemma_variables[record_id]
        binder_names = ["a%d" % index for index in range(len(own))]
        if not binder_names:
            binder_names = ["_a"]
        route, mapping = sb__instantiate_route(record.route, own, binder_names)
        left = sb_subst(record.equation[0], mapping)
        right = sb_subst(record.equation[1], mapping)
        if sb_replay_route(left, route, records) != right:
            raise sb_ProofError("renamed compressed lemma %d does not sb_replay" %
                             record_id)
        binders = " ".join(binder_names)
        lines.append("have lem%d : ∀ %s : G, %s = %s := by" %
                     (record_id, binders, sb_lean_term(left), sb_lean_term(right)))
        lines.append("  intro " + binders)
        lines.extend(sb__calc_lines(left, route, records, lemma_variables,
                                 hypothesis_variables, indent="  "))

    if sb_replay_route(goal_left, goal_route, records) != goal_right:
        raise sb_ProofError("goal's compressed route does not sb_replay")
    if goal_variables:
        lines.append("intro " + " ".join(goal_variables))
    lines.extend(sb__calc_lines(goal_left, goal_route, records, lemma_variables,
                             hypothesis_variables))
    return "\n".join(lines), len(closure)

def sb_emit_lean_theorem(problem, proof_body, theorem_name="generated_proof"):
    """Wrap a compressed body in a standalone theorem for local checking."""

    left, right, goal_left, goal_right, hypothesis_variables, goal_variables = (
        sb_prepare(problem))
    hbinders = " ".join(hypothesis_variables)
    gbinders = " ".join(goal_variables)
    hypothesis = "∀ %s : G, %s = %s" % (
        hbinders, sb_lean_term(left), sb_lean_term(right))
    goal = "∀ %s : G, %s = %s" % (
        gbinders, sb_lean_term(goal_left), sb_lean_term(goal_right))
    indented = "\n".join("  " + line for line in proof_body.splitlines())
    return ("theorem %s {G : Type} (op : G → G → G) "
            "(h : %s) : %s := by\n%s\n" %
            (theorem_name, hypothesis, goal, indented))

def sb_parse_variables(text):
    seen, result = set(), []
    for name in re.findall(r"\b([a-z])\b", text):
        if name not in seen:
            seen.add(name)
            result.append(name)
    return result

def sb_prepare(problem):
    equation1 = problem["equation1"]
    equation2 = problem["equation2"]
    hypothesis_variables = sb_parse_variables(equation1)
    goal_variables = sb_parse_variables(equation2)
    left, right = [sb_parse(side.strip(), set(hypothesis_variables))
                   for side in equation1.split("=")]
    goal_left, goal_right = [sb_parse(side.strip(), set())
                             for side in equation2.split("=")]
    return (left, right, goal_left, goal_right,
            hypothesis_variables, goal_variables)

def sb_prove(problem, budget=12.0, max_size=17, max_rules=2000,
          order="weight"):
    """Attempt a proof and return a result dictionary.

    ``status`` is ``proved`` or ``unknown``.  No negative semantic claim is
    made: finite model search is deliberately outside this prover's scope.
    """

    (left, right, goal_left, goal_right,
     hypothesis_variables, goal_variables) = sb_prepare(problem)
    started = time.monotonic()
    deadline = started + budget if budget is not None else None
    # Preserve time for the goal phase.  sb_Saturation commonly has a nonempty
    # passive set even after it has already derived the one small lemma needed
    # by the goal; spending the entire clock there would hide that success.
    saturation_deadline = (started + budget * 0.8
                           if budget is not None else None)
    rules = sb_complete(left, right, max_size, max_rules,
                     saturation_deadline, order)
    saturation = LAST_SATURATION
    route_methods = (
        ("instance", lambda: sb_join_by_instance_records(
            saturation, goal_left, goal_right)),
        ("sb_normalise", lambda: sb_join_by_normalising_records(
            saturation, goal_left, goal_right,
            max_size=max(max_size, sb_size(goal_left), sb_size(goal_right)))),
    )
    flat_methods = (
        ("pair", lambda: sb_join_by_pair(
            rules, goal_left, goal_right, left, right, deadline)),
        ("rewrite", lambda: sb_join_by_rewriting(
            rules, goal_left, goal_right, left, right,
            max_size=max(max_size, sb_size(goal_left), sb_size(goal_right)),
            deadline=deadline)),
    )
    path = None
    route = None
    method = None
    for name, join in route_methods:
        route = join()
        if route is not None:
            method = name
            path = sb_unfold_route(route, saturation.records)
            break
    if path is None:
        for name, join in flat_methods:
            if deadline is not None and time.monotonic() > deadline:
                break
            path = join()
            if path is not None:
                method = name
                break
    elapsed = time.monotonic() - started
    stats = dict(LAST_STATS)
    stats["rules_returned"] = len(rules)
    if path is None:
        return {"status": "unknown", "seconds": elapsed,
                "method": None, "path": None, "route": None, "lean": None,
                "stats": stats}
    stats["paths_produced"] = stats.get("paths_produced", 0) + 1
    try:
        replayed = sb_replay(goal_left, path, left, right) == goal_right
    except sb_ProofError:
        replayed = False
    if replayed:
        stats["paths_replayed"] = stats.get("paths_replayed", 0) + 1
        path = sb_shorten(path, goal_left, left, right)
        lean = None
        if route is not None:
            try:
                lean, lemma_count = sb_emit_lean_dag(
                    saturation, route, goal_left, goal_right,
                    hypothesis_variables, goal_variables)
                stats["dag_lemmas"] = lemma_count
                stats["certificate_bytes"] = len(lean.encode("utf-8"))
            except sb_ProofError:
                stats["dag_path_failures"] = (
                    stats.get("dag_path_failures", 0) + 1)
        return {"status": "proved", "seconds": elapsed,
                "method": method, "path": path, "route": route,
                "lean": lean, "stats": stats}
    stats["path_failures"] = stats.get("path_failures", 0) + 1
    return {"status": "unknown", "seconds": elapsed,
            "method": None, "path": None, "route": None, "lean": None,
            "stats": stats}

def sb__jsonable_term(term):
    return [sb__jsonable_term(item) if isinstance(item, tuple) else item
            for item in term]

def sb__jsonable_path(path):
    if path is None:
        return None
    return [[list(position), direction,
             {name: sb__jsonable_term(value) for name, value in sigma.items()}]
            for position, direction, sigma in path]

# ── ordered superposition ────────────────────────────────────────────
#
# A given-clause loop with a Knuth-Bendix ordering, forward and backward
# demodulation inside the loop, and subsumption. It reaches lemmas the
# critical-pair search above cannot: the size test used there forbids rewrites
# that do not shrink the term, and some proofs live exactly there.
#
# Its certificates are emitted from the inference DAG rather than flattened --
# each derived lemma stated once as a `have` and cited. On the hardest problem
# in this set that is 13,728 bytes against 3,634,949 for the same proof written
# out flat, so the compression is what makes it submittable at all.


def prove_superposition(problem, eq1_text, eq2_text, budget):
    """A Lean proof body, or None. Body references `op`, so the wrapper binds it."""
    prob = {"id": problem.get("id", "p"),
            "eq1_id": problem.get("eq1_id"), "eq2_id": problem.get("eq2_id"),
            "equation1": eq1_text, "equation2": eq2_text}
    result = sb_prove(prob, budget, 20, 4000, "weight")
    if result.get("status") != "proved":
        return None
    body = result.get("lean")
    if not body or len(body.encode()) > MAX_CERT_BYTES:
        return None
    return body


def true_code_op(body):
    """`true_code`, plus the `op` binding the DAG emitter's bodies refer to."""
    return ("import JudgeProblem\n\n"
            "def submission : Goal := by\n"
            "  intro G _ h\n"
            "  let op : G \u2192 G \u2192 G := fun a b => a \u25c7 b\n"
            + "\n".join("  " + l for l in body.split("\n")) + "\n")


def solve_one(problem):
    """Run the ladder for one problem and return (verdict, code), or None."""
    MARATHON["answer"] = None
    main_body(problem)
    return MARATHON["answer"]


def run_marathon():
    """N problems, one shared wall-clock budget, answers appended as found.

    Scoring reads the answer file after the process is dead, last write wins
    per id, and a wrong answer scores the same as no answer. So the only thing
    that costs points is stopping early — never withholding a certificate we
    already have.
    """
    manifest_path = os.environ["JUDGE_MARATHON_MANIFEST"]
    output_path = os.environ["JUDGE_MARATHON_OUTPUT"]
    budget = float(os.environ.get("JUDGE_MARATHON_BUDGET_SECONDS", "3600"))
    deadline = time.monotonic() + budget

    problems = []
    with open(manifest_path, encoding="utf-8") as fh:
        for raw in fh:
            raw = raw.strip()
            if not raw:
                continue
            try:
                obj = json.loads(raw)
            except ValueError:
                continue
            if isinstance(obj, dict) and "id" in obj:
                problems.append(obj)
    if not problems:
        return

    # One pass. Measured on the reference 100-problem manifest, the ladder
    # needs 34.6 s at worst and 152 s in total, so a per-problem ceiling of
    # budget/N never binds and a second pass would re-run a deterministic
    # search for the same nothing. The floor is what the first stage of the
    # model search costs; without it a small smoke budget starves it.
    cap = max(6.0, budget / len(problems))
    # Leave room for the last write to land: the runner freezes the file at
    # SIGTERM, so a write that starts after the deadline may not count.
    margin = 30.0

    MARATHON["on"] = True
    for prob in problems:
        if time.monotonic() + margin >= deadline:
            break
        MARATHON["deadline"] = min(time.monotonic() + cap, deadline - margin)
        try:
            answer = solve_one(prob)
        except BaseException:      # SystemExit included, deliberately
            answer = None
        if answer is None:
            continue
        verdict, code = answer
        line = json.dumps({"id": prob["id"], "verdict": verdict, "code": code},
                          ensure_ascii=False) + "\n"
        with open(output_path, "a", encoding="utf-8") as fh:
            fh.write(line)
            fh.flush()

if __name__ == "__main__":
    if "JUDGE_MARATHON_MANIFEST" in os.environ:
        run_marathon()
    else:
        main()
