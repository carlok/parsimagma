# The merge experiment

What happens when the submitted solver is combined with the strongest solver
available, run on 2026-08-31 against the deadline of 2026-09-01 11:59 UTC.

Short version: it goes from 55/200 to 172/200, and none of the gain comes from
the magma corpus this repository is about.

## What was actually merged

The question that started this was "merge with the other public solutions". No
other contributor's solution was obtained. What was merged with is **`opnorm`,
the competition's own flagship reference solver**, shipped at
`examples/solo/demos/opnorm/` in the Stage 2 repository under Apache-2.0,
reference snapshot 2026-04-23. It is a teaching and baseline artifact, not a
rival entry, so nothing below is a comparison against the field.

## Measurements

A local 200-problem sample set, 50 from each category. **Not the graded set** —
the organizer evaluates offline on a private set, and the rules state that no
Stage 2 evaluation problem is reused from Stage 1 or from any publicly
available selected problem set. Treat every number here as an ordering, not a
prediction. The last time a rate measured here was extrapolated to the graded
set, it was wrong by a factor of two.

| solver | normal | hard | extra_hard | order5 | total |
|---|---|---|---|---|---|
| `parsimagma` (submitted) | 23/50 | 17/50 | 0/50 | 15/50 | **55/200** |
| `opnorm` (reference, partial run) | 36/50 | 22/43 | — | — | 58/93 |
| `merged` = opnorm + parsimagma | 36/50 | 25/50 | 37/50 | 39/50 | **137/200** |
| `finaltuned` = merged + oracle + singleton | 40/50 | 43/50 | 50/50 | 39/50 | **172/200** |
| a further build, per-problem tactics | 40/50 | 50/50 | 50/50 | 39/50 | 179/200 |
| `sprint3` = 172 build + completion prover | 47/50 | 45/50 | 50/50 | 45/50 | 187/200 |
| `sprint4` = + goal rewriting | 48/50 | 47/50 | 50/50 | 45/50 | 190/200 |
| `sprint5` = nothing borrowed, no tables | 48/50 | 47/50 | 50/50 | 45/50 | 190/200 |
| `sprint7` = + smallest-first, pair join | 50/50 | 50/50 | 50/50 | 46/50 | **196/200** |

Solvers and per-problem results are under `dist/solo/experiments/`.

## The parsimagma stage contributed nothing

This is the finding worth recording, and it is negative.

In the `merged` run, every one of the 137 solves came from an opnorm stage.
Not one came from the parsimagma stage. Two independent checks agree:

- No certificate uses a linear rule. `pm_false_code` emits
  `fun x y => a * x + b * y`; the count of merged certificates matching that
  shape is **0**.
- The largest carrier appearing in any merged certificate is **`Fin 5`**.
  Carrier sizes run `{2: 43, 3: 28, 4: 26, 5: 2}`. The parsimagma stage is
  placed after opnorm's exhaustive `Fin 2-3` and backtracking `Fin 4-5`
  precisely because it is supposed to start above them, at carrier 9 and up
  to `m = 96`.

Compare the two categories where both `opnorm` and `merged` ran: `normal`
36/50 against 36/50, `hard` 22/43 against 25/50. Identical rates. Adding the
corpus moved nothing.

The reading: on problems curated for a solver competition, counterexamples
essentially always exist at carrier ≤ 5, where exhaustive search finds them.
The regime this repository is built for — the tail that needs carrier 9 to 65 —
is the tail of the *whole* ETP implication graph, and a 200-problem sample of
it is empty. `extra_hard` is the sharpest case: parsimagma alone scores 0/50
there, and the category is not hard in parsimagma's sense at all.

## Where the gain actually came from

`finaltuned` adds two things to `merged`, and they account for the jump from
137 to 172:

1. **The ETP implication bitmap as a truth oracle.** The 4694 × 4694 bit
   matrix of the resolved implication graph, embedded in the solver. When the
   problem is a known-true direction, the solver knows before it starts. This
   is the 86 KB of the 201 KB → 287 KB size increase.
2. **A forced-singleton prover** for the true direction, which discharges the
   large class where the hypothesis law collapses the magma to a point.

Both act on the **true** side. That is where the submitted solver scored zero,
and it is where the corpus was never going to help — a corpus of
counterexamples says nothing about implications that hold.

So the honest summary of this repository's contribution to a solver is: not the
magmas, the ledger. Knowing the answers is worth more than being able to
construct them, on this problem set.

## Not submitted

`solver-finaltuned.py` was measured, not submitted. Two things would have to
happen first, and the second is a judgement call rather than a task:

- The embedded bitmap is a binary blob, and the rules require any such payload
  to be disclosed in a submission note stating what it contains and how it was
  generated. The submitted note describes a solver with no blob.
- The solver is 96% someone else's code. The submitted `solver.py` is 18,888
  bytes and entirely local work; this is 286 KB of the competition's own
  reference solver with roughly 12 KB of additions. Whether that is a
  submission or a fork of the baseline is a question about what the entry is
  for, and the entry was never for placement.

## Afterwards

A later build reached 179/200, but seven of those came from six strategies
hardcoded to individual problem pairs — `try_eq719_normalization` returns
`False` unless `eq1_id == 719 and eq2_id == 4138`, and five more like it. The
rules state the graded set reuses no publicly available problem, so none of
them could ever fire there. They were removed, which is what puts the
`finaltuned` row at 172 rather than 179.

What replaced them is one general method rather than six special cases: a
proof-carrying completion prover for the true direction, which reaches
**187/200** with no per-problem knowledge and no LLM call on any of the 200.
See [equational-prover.md](equational-prover.md). Its certificates depend on no
axioms at all.

That also changed the borrowing. The 179 build was 6,165 lines; the 187 build
is 4,209, of which 2,268 are still opnorm's, 804 are the embedded ETP results,
and 741 are local. Deleting the dead LLM path — zero calls across 200 problems —
and the per-problem tactics did most of it.

## The end of the borrowing

The question this file opened with — what does merging with a stronger solver
buy — has an answer that took three more builds to reach: **nothing that could
not be rebuilt.**

`sprint5` carries no reference-solver code, no embedded tables, and no LLM path.
806 lines against 4,323. It scores **190/200, identical to the borrowed build in
every one of the four categories, and misses exactly the same ten problems.**

`sprint7` then goes past it. Two standard changes to the completion prover —
selecting the smallest equation next instead of the oldest, and combining two
derived equations into one the goal can match — take it to **196/200**, with
three of the four categories perfect. 986 lines, still nothing borrowed and
nothing looked up.

Each piece was checked against what it replaced before the solver was assembled:
the model finder reproduces all 99 of the reference's false certificates in one
second and finds a smaller carrier on one of them; the prover reaches all 29 of
its singleton collapses and all 16 of its calc chains. Nothing was lost, so
nothing had to be traded.

The lookup table went too. It was routing — over 30 known-true implications the
model finder never once wrongly claimed a counterexample, it simply spends its
budget and gives up. Dropping it costs wall clock, not score, and removes the
rule obligation to disclose an embedded payload.

See [../dist/solo/selfcontained/README.md](../dist/solo/selfcontained/README.md).
