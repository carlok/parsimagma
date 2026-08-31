"""Drop-in replacement for prover/models.py with incremental constraint checking.

The shipped version re-checks all n^k instances of the hypothesis after every
single cell assignment.  Almost all of that is wasted: assigning cell (p,q) can
only change the status of an instance whose evaluation actually reads (p,q).

Four changes, in descending order of how much they matter here:

  watched cells   Each still-undetermined instance is parked on the ONE cell
                  its evaluation currently blocks on.  Assigning a cell wakes
                  only the instances parked there; everything else is skipped
                  without being touched.  When an instance becomes fully
                  determined and true it is retired for the rest of the subtree.

  unit propagation  When an instance blocks on the root lookup of one side and
                  the other side is already determined, the blocked cell has
                  only one possible value.  Write it instead of guessing it,
                  and cascade.  For laws of the shape `x = C[x,...]` this fires
                  constantly, because the left side is a bare variable.

  isomorphism cut Elements the partial table mentions nowhere are mutually
                  interchangeable, so when choosing a cell's value only one
                  representative of the unmentioned ones need be tried.  Cells
                  are visited in `square_order` to keep the mentioned set a
                  short prefix for as long as possible.  Set iso=False to
                  disable; verdicts are unchanged either way.

  per-size budget `find_counterexample(per_size=...)` gives every carrier its
                  own clock.  The shipped version hands each size whatever is
                  left of one shared pool, so a carrier that is hard but has no
                  model (here carrier 5) eats the entire budget and the search
                  never reaches the carrier that does.

Public API matches models.find_counterexample; `iso` and `per_size` are new
keyword arguments and both default to the shipped behaviour's semantics
(`iso=True` is a pure speedup, `per_size=None` keeps the shared pool).
"""

import time
from itertools import product

UNKNOWN = -1

STATS = {"instances_woken": 0, "node_evals": 0, "cell_trials": 0,
         "nodes": 0, "propagations": 0, "conflicts": 0}


def flatten(lhs, rhs, varnames):
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


def square_order(n):
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


def search_size(n, prog, lroot, rroot, nvars, gprog, glroot, grroot, gnvars,
                deadline, stats=False, iso=True):
    ncells = n * n
    order = square_order(n) if iso else list(range(ncells))
    nprog = len(prog)
    tbl = [UNKNOWN] * ncells
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
            STATS["node_evals"] += nprog
            return _ev(i)

    def process(c, queue):
        lst = watch[c]
        j = 0
        while j < len(lst):
            i = lst[j]; j += 1
            if state[i] or wcell[i] != c:
                continue
            if stats: STATS["instances_woken"] += 1
            blk, cc = evaluate(i)
            if blk < 0:
                if val[lroot] != val[rroot]:
                    if stats: STATS["conflicts"] += 1
                    return False
                state[i] = 1; wcell[i] = -1
                trail.append((1, i, c)); pending[0] -= 1
            elif blk == rroot and lroot < rroot:
                # evaluation reached the last node, so every earlier node -- the
                # other side's root included -- already has a value: this cell
                # has exactly one admissible value rather than n.
                setcell(cc, val[lroot]); queue.append(cc)
                if stats: STATS["propagations"] += 1
                state[i] = 1; wcell[i] = -1
                trail.append((1, i, c)); pending[0] -= 1
            elif blk == lroot and rroot < lroot:
                setcell(cc, val[rroot]); queue.append(cc)
                if stats: STATS["propagations"] += 1
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
                tbl[e[1]] = UNKNOWN; mx[0] = e[2]

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
        if stats: STATS["nodes"] += 1
        if deadline is not None and time.monotonic() > deadline:
            raise TimeoutError
        p = start
        while p < ncells and tbl[order[p]] != UNKNOWN:
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
            if stats: STATS["cell_trials"] += 1
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


def find_counterexample(eq1_text, eq2_text, parse_vars, parse_term,
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
    prog, lroot, rroot = flatten(l1, r1, allv)
    gprog, glroot, grroot = flatten(l2, r2, allv)
    for n in sizes:
        left = budget - (time.monotonic() - t0)
        if left <= 0.5:
            break
        # per_size caps what one carrier may spend; the total budget still
        # binds, otherwise a long `sizes` list overruns the caller's deadline.
        dl = time.monotonic() + (min(left, per_size) if per_size is not None else left)
        tbl = search_size(n, prog, lroot, rroot, len(allv),
                          gprog, glroot, grroot, len(allv), dl, stats, iso)
        if tbl is not None:
            return n, [tbl[i * n:(i + 1) * n] for i in range(n)]
    return None, None
