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


def main():
    problem = read_message()["problem"]
    eq1 = problem["equation1"].replace("*", "◇")
    eq2 = problem["equation2"].replace("*", "◇")

    # A counterexample at carrier 2 or 3 is nearly free, so look before proving.
    code = attempt(find_model, eq1, eq2, (2, 3), 5.0)
    if code and call_judge("false", code).get("status") == "accepted":
        return

    # Proving is cheap; the wider carriers are not. Exhaust every proof route
    # before paying for a table search that a true implication can never end.
    for args in ((prove, eq1, eq2, 14.0),
                 (prove_collapse, eq1, eq2, 10.0),
                 (prove_by_normalising, eq1, eq2, 60.0),
                 (prove, eq1, eq2, 22.0, True)):
        body = attempt(*args)
        if body and call_judge("true", true_code(body)).get("status") == "accepted":
            return

    code = attempt(find_model, eq1, eq2, (4, 5, 6, 7), 240.0)
    if code:
        call_judge("false", code)



def emit_with_lemmas(uses_l, uses_r, rules, GL, GR, L, R, hv, gv):
    """A proof that states each derived lemma once and then applies it.

    `emit` inlines every hypothesis application, which is right for a short
    chain and ruinous when the same lemma is used three times over large terms.
    Here each lemma becomes a `have` proved by its own calc chain, and the goal
    is a calc chain over lemma applications.
    """
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

if __name__ == "__main__":
    main()
