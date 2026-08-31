import json, sys, time, re
import eqprove as E


def parse_variables(text):
    """Same rule the solver uses: single lowercase letters, first appearance."""
    seen, out = set(), []
    for v in re.findall(r"\b([a-z])\b", text):
        if v not in seen:
            seen.add(v)
            out.append(v)
    return out

def prep(problem):
    """Hypothesis with variables, goal with its variables Skolemised."""
    e1, e2 = problem["equation1"], problem["equation2"]
    hv = parse_variables(e1)   # order of first appearance, as the judge binds them
    gv = parse_variables(e2)
    L, R = [E.parse(s.strip(), set(hv)) for s in e1.split("=")]
    GL, GR = [E.parse(s.strip(), set()) for s in e2.split("=")]   # all constants
    return L, R, GL, GR, hv, gv

def verify(path, lhs, rhs, start, end):
    """Replay every step. Returns None if sound, else a description of the break."""
    cur = start
    for i, (term, p, tag, s) in enumerate(path):
        src, dst = (lhs, rhs) if tag == "fwd" else (rhs, lhs)
        sub = cur
        for d in p:
            if sub[0] != "o":
                return f"step {i}: position {p} does not exist"
            sub = sub[d + 1]
        if E.subst(src, s) != sub:
            return (f"step {i}: rule {tag} instance {E.show(E.subst(src,s))} "
                    f"!= subterm {E.show(sub)}")
        if E.replace(cur, p, E.subst(dst, s)) != term:
            return f"step {i}: replacement does not yield the recorded term"
        cur = term
    return None if cur == end else f"path ends at {E.show(cur)}, wanted {E.show(end)}"

def make_pool(GL, GR, gv, max_pool_size=5):
    """Terms the expanding direction may substitute for its free variables.

    Goal constants, plus every subterm of the goal: those are the structures
    the goal actually needs, and guessing them from the goal beats enumerating
    arbitrary terms."""
    seen, pool = set(), []
    for t in [("c", v) for v in gv] + [s for g in (GL, GR) for _, s in E.positions(g)]:
        k = E.show(t)
        if k not in seen and E.size(t) <= max_pool_size:
            seen.add(k)
            pool.append(t)
    return sorted(pool, key=E.size)

def run(problem, budget=25.0, max_size=19, cap=40, max_expansions=4):
    L, R, GL, GR, hv, gv = prep(problem)
    pool = make_pool(GL, GR, gv)
    t0 = time.monotonic()
    path = E.search(L, R, GL, GR, pool, max_size=max_size,
                    expanding_cap=cap, max_expansions=max_expansions,
                    deadline=t0 + budget)
    dt = time.monotonic() - t0
    if path is None:
        return None, dt, None
    return path, dt, verify(path, L, R, GL, GR)

if __name__ == "__main__":
    probs = json.load(open(sys.argv[1]))
    only = sys.argv[2] if len(sys.argv) > 2 else ""
    budget = float(sys.argv[3]) if len(sys.argv) > 3 else 25.0
    found = bad = 0
    for p in probs:
        if only and not p["id"].startswith(only):
            continue
        path, dt, err = run(p, budget=budget)
        if path is None:
            print(f"  {p['id']:22} {p['eq1_id']:>6}->{p['eq2_id']:<6}  no derivation   {dt:5.1f}s")
        elif err:
            bad += 1
            print(f"  {p['id']:22} {p['eq1_id']:>6}->{p['eq2_id']:<6}  UNSOUND: {err}")
        else:
            found += 1
            print(f"  {p['id']:22} {p['eq1_id']:>6}->{p['eq2_id']:<6}  {len(path)} steps, verified  {dt:5.1f}s")
    print(f"\n  derivations found and replayed: {found}   unsound: {bad}")


# ── Lean emission ────────────────────────────────────────────────────

def context_lambda(term, p):
    """`fun t => C[t]` for the context of position p in term, or None at the root."""
    if not p:
        return None
    hole = ("c", "\x00")
    return "fun t => " + E.show(E.replace(term, p, hole)).replace("\x00", "t")


def step_proof(cur, p, tag, s, hv):
    """A Lean term proving `cur = next` for one rewrite step."""
    missing = [v for v in hv if v not in s]
    assert not missing, f"unbound hypothesis variables {missing} in step substitution"
    args = " ".join(
        E.show(s[v]) if s[v][0] != "o" else f"({E.show(s[v])})" for v in hv
    )
    base = f"h {args}" if args else "h"
    if tag == "bwd":
        base = f"({base}).symm"
    lam = context_lambda(cur, p)
    return base if lam is None else f"congrArg ({lam}) ({base})"


def emit(path, GL, GR, hv, gv):
    """A `calc` chain, one line per rewrite step."""
    if not path:
        return f"intro {' '.join(gv)}\nrfl"
    lines = [f"intro {' '.join(gv)}", "calc"]
    cur = GL
    first = True
    for (term, p, tag, s) in path:
        proof = step_proof(cur, p, tag, s, hv)
        lhs = E.show(cur) if first else "_"
        lines.append(f"  {lhs} = {E.show(term)} := {proof}")
        cur = term
        first = False
    return "\n".join(lines)


def run_complete(problem, budget=12.0, max_size=15, max_rules=1200):
    """Completion first, then the direct-instance join. Returns (path, err)."""
    import complete as C
    L, R, GL, GR, hv, gv = prep(problem)
    t0 = time.monotonic()
    rules = C.complete(L, R, max_size=max_size, max_rules=max_rules,
                       deadline=t0 + budget * 0.7)
    raw = C.join_by_instance(rules, GL, GR, L, R)
    if raw is None:
        raw = C.join_by_pair(rules, GL, GR, L, R, deadline=t0 + budget)
    if raw is None:
        return None, time.monotonic() - t0, None
    # completion emits (pos, tag, subst); the emitter wants each step's
    # resulting term too, so replay once and carry it.
    path, cur = [], GL
    for (pos, tag, sub) in raw:
        src, dst = (L, R) if tag == "fwd" else (R, L)
        cur = E.replace(cur, pos, E.subst(dst, sub))
        path.append((cur, pos, tag, sub))
    return path, time.monotonic() - t0, verify(path, L, R, GL, GR)
