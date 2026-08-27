# Post-Phase-A sprint queue

Five sprints, not four: greedy constructions do not fit in one. Each has an
exit criterion and a kill criterion, because two of these are research and can
fail for good reasons.

The running size-4 sweep is **not** a gate. It contributes zero to the pinned
denominator by construction, needs no attention, and finishes on its own into
`out/bruteforce4.log`. Nothing below waits on it.

---

## S1 — Pin the target

**Half a day. Low risk. Do this first.**

Re-fetch `data/2025-08-11-vampire.json.gz` (84 MB), read the per-method fields,
and settle the Phase 0 discrepancy: Janota's "only 310 of the undecided
implications require an infinite model" against the paper's 820 pairs whose
truth flips under finiteness, against the 814 pairs his saturation runs refuted
without producing a witness (`vsi500` 778 + `vss600` 36).

**Why first, not third.** It partitions the 1062 into *needs an infinite
construction* and *needs a finite search Vampire did not reach*. We already
know `Z/13` alone accounts for 268 of the second kind. If that partition is
lopsided, S2 and S3 are aimed differently — a large finite-reachable remainder
means widening structured finite families, not building greedy.

- **Deliverable:** `docs/hard-core-anatomy.md`, `data/etp/infinite_only.txt`,
  and a coverage report split by partition.
- **Exit:** every one of the 1062 labelled, and the three published numbers
  either reconciled or shown to measure different things.
- **Kill:** none. This is bounded.
- **Housekeeping:** delete the dump once the derived files are committed, as
  with the last one.

## S2 — Identify the method for the 296-pair cluster

**One day. Low risk, research not build.**

E1133, E1167, E1659, E1661, E1979, E2000, E2473, E2481 — 37 uncovered pairs
each, 46% of what is left. They are **not** in blueprint ch. 27's curated list,
so the method is unrecorded there. Read the per-equation blueprint chapters and
the Zulip archive; check whether the eight are four dual pairs of one thing.

**Why before greedy.** If the method turns out to be translation-invariant
models (§5.3) or something already half-built, S3 and S4 shrink or disappear.
Building greedy first and discovering that afterwards would be the expensive
ordering.

- **Deliverable:** `docs/cluster-296.md` naming a method per law, or stating
  that they are ad hoc.
- **Exit:** a named construction for at least four of the eight.
- **Kill:** if two days pass with no method identified, treat the cluster as
  ad hoc and go to S3 without it.

## S3 — Greedy spike

**Two to three days. High risk. One target only.**

Do **not** attempt general greedy construction. Pick one target from ch. 27's
greedy list with a small ruleset and implement only that: `E1648 ⊭ E3253` is
the smallest single-target entry; `E1076 ⊭ 47, 99, 151, ...` (24 pairs) is the
highest-value one and is already half-covered by `Z/13`, which makes the
comparison informative.

The construction is the seed-and-extend scheme of paper §5.5: a finitely
supported partial operation, extended greedily under a ruleset, on carrier `N`.
Verification reuses what exists — the produced magma is checked against the
hypothesis and the target by the Tier F engine, and any claimed separation is
checked against `implications.bits`, so a wrong construction fails loudly.

- **Deliverable:** one reproduced separation the ETP attributes to greedy, plus
  a written judgement on whether the general form is tractable here.
- **Exit:** the target separation reproduced and cross-checked.
- **Kill:** three days without a verified separation. Greedy is inherently
  infinitary and the ETP spent months on the hard ones; failing here is
  information, not defeat, and it goes in the report as such.

## S4 — Greedy generalised

**Conditional on S3. Size unknown until S3 lands.**

Parameterise the ruleset, define a finite grid over rulesets and seeds, state
it in the output as every other family does, and measure the coverage delta on
the S1 partition.

- **Exit:** coverage number recomputed with greedy in the corpus.
- **Kill:** if the grid saturates the way the linear and twist grids did —
  large instance counts, no new hard-core pairs — stop and report the
  saturation. That result is worth as much as coverage.

## S5 — Phase B: set cover bounds

**BLOCKED. Needs your explicit go, not a queue slot.**

The brief says stop at the end of Phase A and do not start Phase B. I am not
starting this on my own initiative. When authorised it is small: the greedy
upper bound already exists (13 instances for the 416 reached), so the work is
an LP relaxation for the lower bound and reporting the **gap**, which is the
quantity of interest. The word "minimal" stays out of the output either way —
set cover is NP-hard here and the ETP already attaches "minimal" to its
generating set in a different, graph-theoretic sense.

- **Deliverable:** upper bound, lower bound, gap, against a stated grid.
- **Precondition:** a coverage number worth basing a basis on. Running set
  cover over 39% coverage measures the corpus's gaps more than its economy.

---

## Ordering rationale in one line each

| | why here |
|---|---|
| S1 | cheap, and it changes what S2/S3 aim at |
| S2 | cheap, and it may delete S3/S4 |
| S3 | expensive and risky, so it goes after the two things that could shrink it |
| S4 | only exists if S3 works |
| S5 | out of Phase A scope until you say otherwise |
