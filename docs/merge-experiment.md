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
