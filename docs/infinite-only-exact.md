# S8 — the hard core, pinned exactly

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

`hard-core-anatomy.md` bounded the infinite-only part of the 1062 from below and
left the exact figure open, because the closure of the finite implication graph
was not available. It is available. The Equation Explorer serves it.

The consequences: the published 310 is wrong by roughly a factor of two, and this
corpus's headline coverage number was understated by its own denominator.

## Where the data came from

`home_page/implications/.gitignore` lists three build artifacts the repo does not
track but the site serves:

```
https://teorth.github.io/equational_theories/implications/graph.json         (9.1 MB)
https://teorth.github.io/equational_theories/implications/finite_graph.json  (9.0 MB)
```

Each is `{"rle_encoded_array": [value, run, value, run, …]}`, decoding to a
4694 x 4694 row-major status matrix. The nine status codes are in
`home_page/implications/script.js`: `2 = explicit_proof_false`,
`3 = explicit_proof_true`, `6 = implicit_proof_false`, `7 = implicit_proof_true`,
`8 = unknown`. "Implicit" means obtained by closure, so these are the closed
graphs, not generating sets.

Decoded, they reproduce the dashboard exactly:

| | general | finite |
|---|---|---|
| explicit true | 10,657 | 10,750 |
| implicit true | 8,167,622 | 8,168,349 |
| explicit false | 586,925 | 586,220 |
| implicit false | 13,268,432 | 13,268,315 |
| unknown | 0 | **2** |

The two unknowns are `(677, 255)` and `(2910, 47)`, exactly as the final report
states, recovered here without being told.

## The 820 is the closed count

Pairs that are false in general and true for finite magmas: **820**, computed
pair by pair. That settles the question left open in
[#1474](https://github.com/teorth/equational_theories/issues/1474) — it is the
closure count, not a generating-set count. The floor argument in
`hard-core-anatomy.md` therefore held, and is now superseded by the exact split.

## The 1062, fully decomposed

```
hard core, 1062 pairs
  require an infinite model         610
  finite counterexample exists      450
      witnessed by this corpus        411
      reached by nobody here           39
  still unknown to the ETP            2     (677->255 and its dual)
```

Janota reports "only 310 of the undecided implications require an infinite model
according to the Equational project". **The ordered-pair figure is 610.** Whether
that makes 310 wrong depends on a convention the paper does not state, and the
honest answer is that it probably does not:

| reading of the same set | count |
|---|---|
| ordered pairs | **610** |
| up to duality | **316** |
| up to duality, both members required to lie in the 1062 | **294** |

310 falls between the two dual-class readings and matches neither. Nothing else
tried lands near it either — explicitly finite-true inside the 1062 is 51, the
generating-set hypothesis floated in `hard-core-anatomy.md` gives nothing close.
So 310 reads as a dual-class count against a slightly earlier state of the finite
graph, six off the current 316, rather than an error. The useful output here is
the ordered-pair split, not a correction.

Same computation on the 814 saturation-refuted pairs:

```
saturation-refuted, 814 pairs
  require an infinite model         210
  finite counterexample exists      604
      witnessed by this corpus        379
      reached by nobody here          225
```

610 + 210 = 820. Every infinite-only pair lands in one of the two Vampire sets,
with nothing left over — which is what `hard-core-anatomy.md` predicted from
first principles and could not then verify.

## The coverage number was understated by its denominator

`phase-a-report.md` leads with 411 of 1062, 39%, and reads it as a corpus that is
within 0.15% of complete against the whole graph but weak against the part a good
prover could not do. That comparison is not like for like. 610 of those 1062
**cannot** be refuted by any finite construction, so no corpus of finite magmas
can ever reach them.

Against the part that is finitely refutable at all:

| | corpus | finitely refutable | coverage |
|---|---|---|---|
| the 1062 | 411 | 450 | **91%** |
| the 814 | 379 | 604 | **63%** |

The honest statement is not "39% of the hard core". It is "91% of the finitely
refutable hard core, with 39 pairs outstanding".

## The 39

Emitted to `data/etp/finite_uncovered.txt` (264 rows: 39 from the 1062, 225 from
the 814). These are pairs the ETP proves have a finite counterexample and that no
construction in this corpus finds — the precise remaining target, replacing "646
uncovered", of which it turns out 605 were unreachable in principle.

Hypothesis laws over the 39, by count:

```
E2712 7   E854 6   E1518 4   E2054 4   E503 3   E3069 3
E476 2   E1076 2   E2531 2   E3076 2
E879 1   E1516 1   E2091 1   E2650 1
```

E854 and E1516 have their own blueprint chapters, so the method for those is
recorded upstream and can be read off rather than guessed. E1076 and E2531 are
the dual pair already 134-covered each by `Z/13` under `x ◇ y = 7x + 7y`; only
two pairs each remain.

## Differential test, which is why any of this is trustworthy

ETP's finite graph is fully Lean-verified, so it is an oracle for every finite
claim this corpus makes.

| check | result |
|---|---|
| 790 pairs this corpus witnesses with a finite magma, against ETP's finite graph | **790/790 agree**, 0 disagreements |
| 7 pairs this corpus labels infinite-only, against ETP's finite graph | **7/7 agree** |
| decoded general graph against the dashboard, and against Janota's "proved 8,173,585" | exact |
| decoded general graph false count against this repo's 13,855,357 | exact |

Zero disagreements across 797 independently derived finite claims is the
strongest validation this engine has had. The previous best oracle was 824
magmas of size at most 4.

## Reproducing

```
curl -sLO https://teorth.github.io/equational_theories/implications/graph.json
curl -sLO https://teorth.github.io/equational_theories/implications/finite_graph.json
# decode: pairs of (status, run_length), reshape row-major to 4694 x 4694
# false = {2, 6}, true = {3, 7}, unknown = 8
```

then intersect with `data/etp/hard_core.txt`, `data/etp/saturation_refuted.txt`
and `data/etp/hard_core_partition.tsv`.

Caveat: both graphs were fetched 2026-08-28. The two unknowns will move if
`E677 ⊧fin E255` is settled; nothing else in the finite graph can change.
