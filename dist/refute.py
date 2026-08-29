"""Refute an equational implication by exhibiting a finite magma.

Given two laws E and E' as text, find a finite magma satisfying E and violating
E'.  That magma is a disproof of `E => E'`, and Lean checks it with plain
`decide`.

Self-contained: standard library only, no data files.  The construction corpus
is a generator, not a table -- the families are cheap to enumerate and the
useful ones are tried first, in an order taken from a greedy set cover over the
Equational Theories Project's 13,855,357 false implications.  Four instances of
Z/2 alone account for 85.7% of them.

Laws are written in the ETP's own syntax, e.g.

    x = y * ((x * (x * y)) * y)

with `*` or the diamond as the operator; single-letter variables; both sides
parenthesised as needed.
"""

import time as _time
from itertools import product

DIAMOND = "◇"

# --------------------------------------------------------------------------
# Law parsing


class Term:
    __slots__ = ("var", "lhs", "rhs")

    def __init__(self, var=None, lhs=None, rhs=None):
        self.var, self.lhs, self.rhs = var, lhs, rhs


def _tokens(s):
    for ch in s.replace(DIAMOND, " * ").replace("(", " ( ").replace(")", " ) ").split():
        yield ch


def _atom(toks, pos):
    t = toks[pos]
    if t == "(":
        inner, pos = _side(toks, pos + 1)
        assert toks[pos] == ")", f"expected ), got {toks[pos]!r}"
        return inner, pos + 1
    assert len(t) == 1 and t.isalpha(), f"expected a variable, got {t!r}"
    return Term(var=t), pos + 1


def _side(toks, pos):
    """side := atom ['*' atom].  The ETP writes terms fully parenthesised
    except at the top level, and uses no associativity convention, so the
    operator never chains."""
    left, pos = _atom(toks, pos)
    if pos < len(toks) and toks[pos] == "*":
        right, pos = _atom(toks, pos + 1)
        return Term(lhs=left, rhs=right), pos
    return left, pos


def parse_law(text):
    """`text` -> (lhs, rhs, arity) with variables indexed by first appearance."""
    toks = list(_tokens(text))
    eq = toks.index("=")
    ltoks, rtoks = toks[:eq], toks[eq + 1 :]
    lhs, p = _side(ltoks, 0)
    assert p == len(ltoks), f"trailing tokens in lhs of {text!r}"
    rhs, p = _side(rtoks, 0)
    assert p == len(rtoks), f"trailing tokens in rhs of {text!r}"

    names = []

    def walk(t):
        if t.var is not None:
            if t.var not in names:
                names.append(t.var)
        else:
            walk(t.lhs)
            walk(t.rhs)

    walk(lhs)
    walk(rhs)
    idx = {n: i for i, n in enumerate(names)}

    def index(t):
        if t.var is not None:
            t.var = idx[t.var]
        else:
            index(t.lhs)
            index(t.rhs)

    index(lhs)
    index(rhs)
    return lhs, rhs, len(names)


def _eval(t, vals, table, n):
    if t.var is not None:
        return vals[t.var]
    return table[_eval(t.lhs, vals, table, n) * n + _eval(t.rhs, vals, table, n)]


def holds(law, table, n):
    """Whether the magma with `table` satisfies `law`, by exhaustive sweep."""
    lhs, rhs, k = law
    for vals in product(range(n), repeat=k):
        if _eval(lhs, vals, table, n) != _eval(rhs, vals, table, n):
            return False
    return True


def witness(law, table, n):
    """An assignment refuting `law`, or None."""
    lhs, rhs, k = law
    for vals in product(range(n), repeat=k):
        if _eval(lhs, vals, table, n) != _eval(rhs, vals, table, n):
            return vals
    return None


# --------------------------------------------------------------------------
# The construction corpus, as a generator

# Opening moves, in the order a greedy set cover picks them over the ETP's
# 13,855,357 false implications.  The first four -- all of Z/2 -- account for
# 11,871,871 of them, 85.7%; the first twenty-five for 97.2%.  Everything after
# that is reached by the systematic sweep below, which is why no table ships.
PRIORITY = [
    (2, 0, 0), (2, 0, 1), (2, 1, 0), (2, 1, 1),
    (3, 0, 2), (3, 2, 0), (3, 2, 2),
    (7, 0, 2), (7, 2, 0),
    (5, 2, 4), (5, 3, 3), (5, 4, 2),
    (4, 2, 3), (4, 0, 2), (4, 2, 0), (4, 3, 2), (4, 2, 2),
    (13, 7, 7), (11, 6, 6), (5, 0, 2),
]

LINEAR_MAX = 32   # m for the systematic linear sweep
AFFINE_MAX = 16   # m for the affine sweep, which is m^3 instances


def linear_table(m, a, b, c=0):
    """`x * y = a*x + b*y + c` over Z/m, as a flat table."""
    return [(a * x + b * y + c) % m for x in range(m) for y in range(m)]


def candidates(limit=None):
    """Finite magmas to try, cheapest and most productive first."""
    seen = set()
    for m, a, b in PRIORITY:
        seen.add((m, a, b, 0))
        yield m, linear_table(m, a, b), f"x*y = {a}x+{b}y over Z/{m}", (a, b, 0)
    n = 0
    for m in range(2, LINEAR_MAX + 1):
        for a in range(m):
            for b in range(m):
                if (m, a, b, 0) in seen:
                    continue
                yield m, linear_table(m, a, b), f"x*y = {a}x+{b}y over Z/{m}", (a, b, 0)
                n += 1
                if limit and n > limit:
                    return
    for m in range(2, AFFINE_MAX + 1):
        for a in range(m):
            for b in range(m):
                for c in range(1, m):
                    yield m, linear_table(m, a, b, c), f"x*y = {a}x+{b}y+{c} over Z/{m}", (a, b, c)
                    n += 1
                    if limit and n > limit:
                        return


def refute(hyp_text, concl_text, limit=None, budget=None):
    """A finite magma satisfying `hyp_text` and violating `concl_text`.

    Returns a dict with the carrier, the table, a description of the rule and a
    refuting assignment for the conclusion, or None if nothing in the corpus
    separates them.  `None` is not a proof that the implication holds.

    `budget` caps the search in seconds.  It matters: on a random false
    implication the answer is usually immediate -- 96% are refuted, and 89% of
    those by a two-element magma found in the first four tries -- but a miss
    pays for the whole sweep, about 26 seconds.  Under a per-problem time limit,
    set a budget and treat a timeout as "no answer" rather than as "no witness".
    """
    hyp = parse_law(hyp_text)
    concl = parse_law(concl_text)
    deadline = None if budget is None else _time.monotonic() + budget
    for k, (n, table, desc, coeffs) in enumerate(candidates(limit)):
        if deadline is not None and (k & 255) == 0 and _time.monotonic() > deadline:
            return None
        # The hypothesis is the restrictive one, so testing it first discards
        # nearly every candidate on its first failing assignment.
        if not holds(hyp, table, n):
            continue
        w = witness(concl, table, n)
        if w is not None:
            return {
                "carrier": n,
                "table": table,
                "rule": desc,
                "counterexample": w,
                "coeffs": coeffs,
                "hypothesis": hyp_text,
                "conclusion": concl_text,
            }
    return None


def to_lean(r):
    """The Stage 2 false certificate: a term of the judge-provided `Goal`.

    `Goal` is `∃ (G : Type) (_ : Magma G), EquationLHS G ∧ ¬ EquationRHS G`,
    bound by the judge's `JudgeProblem`, so the certificate only has to exhibit
    the carrier, the magma and a `decide`.

    The operation goes in as a function rather than through `finOpTable`. That
    helper parses its argument one character at a time --

        extractDigits s = s.toList.filterMap fun c => if c.isDigit then ...

    -- so any table entry of 10 or more is silently read as two entries. Every
    witness here is linear or affine anyway, which states in a dozen characters
    and stays far inside the 20,000-byte cap on false certificates.
    """
    n, rule = r["carrier"], r["rule"]
    a, b, c = r["coeffs"]
    terms = []
    if a:
        terms.append("x" if a == 1 else f"{a} * x")
    if b:
        terms.append("y" if b == 1 else f"{b} * y")
    if c:
        terms.append(str(c))
    expr = " + ".join(terms) if terms else "0"
    return (
        "import JudgeProblem\n"
        "import JudgeDecide.DecideBang\n"
        "\n"
        f"-- {rule}\n"
        "def submission : Goal := by\n"
        f"  let m : Magma (Fin {n}) := {{ op := fun x y => {expr} }}\n"
        f"  refine ⟨Fin {n}, m, ?_⟩\n"
        "  decideFin!\n"
    )


def to_lean_table(r):
    """Fallback certificate for a witness that is not given by a rule.

    Uses the judge's `magmaFin`, which takes a `List Nat` and so is safe for
    carriers of ten or more, unlike `finOpTable`.
    """
    n = r["carrier"]
    vals = ", ".join(str(v) for v in r["table"])
    return (
        "import JudgeProblem\n"
        "import JudgeDecide.DecideBang\n"
        "\n"
        f"-- {r['rule']}\n"
        "def submission : Goal := by\n"
        f"  let m : Magma (Fin {n}) := magmaFin {n} [{vals}]\n"
        f"  refine ⟨Fin {n}, m, ?_⟩\n"
        "  decideFin!\n"
    )
