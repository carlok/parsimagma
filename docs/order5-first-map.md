# S9 — a first coverage measurement on the order-5 graph

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

The ETP mapped the 4694 laws of order at most 4 and its final report names order
5 as the frontier. `data/etp/eq_size5.txt` carries 62,576 laws of order at most
5; no implication graph for them exists. This is what the existing construction
corpus discharges against that law set, run with `pm order5`.

## The number

```
law set     62,576 laws of order <= 5
instances   66,151 linear and affine, swept in 54s
rows         5,899 distinct signatures, mean 94.6 laws satisfied, max 22,604
hypothesis laws reached  41,462

distinct ordered pairs discharged  2,299,094,885  of  3,915,693,200
```

Stated as a fact rather than a ratio: **at least 2,299,094,885 of the
3,915,693,200 ordered pairs of order-5 laws are non-implications**, each
exhibited by an explicit finite magma from the corpus. That is a lower bound on
the false part of a graph nobody has mapped, and it took under a minute.

| | order <= 4 | order <= 5 |
|---|---|---|
| laws | 4,694 | 62,576 |
| ordered pairs | 22,028,942 | 3,915,693,200 |
| pairs discharged by this corpus | 13,834,667 | 2,299,094,885 |
| known false | 13,855,357 | **unknown** |

## What this does not say

**No oracle for the implication graph — but order 5 is not unexplored.** The
blueprint chapter "Order 5 Austin laws" classifies all 57,882 equations of order
exactly 5 by *model existence*: 19,392 admit only trivial models, 38,360 have
known satisfying finite models, 106 admit only trivial finite models (10 of them
Austin laws, 96 with infinite models unknown), and 24 are unknown. What does not
exist is the implication graph between those laws — the chapter notes only that
"Vampire did not establish any implications between equations in this set" for
the 96. So the measurement above is not duplicated work, but the framing "order 5
is unmapped" is too broad and the chapter should be read before building on this.

That classification is a **partial oracle**, in one direction: every law this
corpus satisfies with a nontrivial finite magma must be one ETP records as having
a nontrivial finite model. The counts line up. This run reaches 41,462
hypothesis laws; ETP's 38,360 order-5 laws with known finite models plus the
3,198 order-<=4 laws with nontrivial finite models give 41,558. Being under that
is the correct direction, since a linear and affine corpus will not find every
model ETP knows about, and 0.23% under it is closer than expected.

The containment itself has **not** been checked law by law — the classification
lives in the blueprint text and a Zulip thread rather than in a machine-readable
file on `vlad902/equational_theories@order5`, whose `data/` matches main. Doing
that check is the obvious next step and would upgrade this from a measurement to
a validated one.

What holds it up instead is that the engine is the same one validated at order 4
against 824 ETP tables and 790 hard-core witnesses, that `open-questions-scan.md`
records positive and negative controls on order-5 laws specifically, and that
each discharged pair is witnessed by a concrete magma and so is checkable
one at a time by anyone who doubts it. That is weaker than an oracle. It should
be read as weaker.

**58.7% of all ordered pairs is not a coverage rate.** Most pairs in any such
graph are true implications and cannot be discharged at all. At order 4, 62.9% of
pairs are false and the same families reach 99.85% of them. The comparable
order-5 figure cannot be computed without knowing its true/false split, and that
is the thing nobody knows. Quoting 58.7% as coverage would repeat exactly the
denominator error that `infinite-only-exact.md` had to correct.

**Linear and affine families only.** `pm order5` runs `linear_corpus` and does
not add twisted powers, quadratic magmas, translation-invariant models or the
matrix rings beyond those already in the linear grid. Adding them can only raise
the count.

## Cost

The sweep is 54s for 66,151 instances against 62,576 laws, so roughly 13x the
laws of order 4 at comparable wall clock. The law DAG builds in 40ms. Counting
distinct discharged pairs takes 443ms: for each hypothesis law, the targets it
can never reach are the intersection of every row satisfying it, so the whole
accounting is one pass of bitwise ANDs rather than a 3.9-billion-entry matrix.

Order-5 laws use up to 7 variables (profile: 97 / 4,937 / 21,956 / 24,547 /
9,565 / 1,408 / 66 for 1 through 7), so the worst case per magma at carrier 13 is
about 1.5e10 evaluations. It does not cost that, because the sweep abandons a
variable bucket as soon as every law in it is refuted and visits assignments on a
golden-ratio stride so violations surface early.

## Reproducing

```
cargo build --release && ./target/release/pm order5
```
