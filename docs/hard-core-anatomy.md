# S1 — anatomy of the hard core

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

What the 1062 actually contains, and why three published numbers describing it
do not line up. Everything here is recomputed from
`data/2025-08-11-vampire.json.gz` (upstream `e5a88a1`, sha256
`16e8efcd…a61b2`), streamed rather than loaded: 22,028,942 entries in 18
seconds at 184 MB peak.

## The three numbers

| number | source | what it was taken to mean |
|---|---|---|
| 1062 | Janota, arXiv:2508.15856 | implications Vampire left undecided |
| 820 | ETP paper §8 | pairs with `E ⊭ E'` but `E ⊧_fin E'` — no finite counterexample exists |
| 310 | Janota, quoting ETP data | "only 310 of the undecided implications require an infinite model" |
| 814 | derived here | pairs refuted by Vampire's *saturation* mode, so with no witnessing model |

The Phase 0 report guessed that the 814 and the 820 were close to the same set,
on the reasoning that a saturation refutation produces no model and might
therefore be the infinite-only cases. **That guess is wrong**, and the
correction is the main result of this sprint.

## The 814 are mostly finite, and small

Labelling every pair with the smallest carrier in the construction corpus that
witnesses it:

```
saturation-refuted, 814 pairs
  finite  7      329
  finite  9       34
  finite 11       16
  infinite only    2
  uncovered      433
```

**379 of the 814 have finite counterexamples, 329 of them on seven elements.**
A seven-element model is not out of reach for any finite model builder. The
explanation is Janota's method ordering, not difficulty: the run tries
`vmi500` (fmb, 500-instruction cap), then `vsi500` (saturation, same cap), then
`vms60` (fmb, 60s), then `vss600`, then `vms600`. A pair refuted by `vsi500`
is recorded as solved and **never sees the longer fmb budgets**. So "refuted
without a witnessing model" means "fmb's first and smallest budget missed it",
not "no finite model exists".

## The 1062 are at least a third finite

```
hard core, 1062 pairs
  finite  9        4
  finite 11      101
  finite 13      268
  finite 32       38
  infinite only    5
  uncovered      646
```

411 have finite witnesses at carrier size at most 32. Only 5 of the 416 the
corpus reaches need an infinite carrier.

## Where 310 fails

Every infinite-only pair has `E ⊭ E'` with no finite counterexample. Vampire's
finite model builder therefore cannot refute it, and saturation cannot prove it
because it is false. Its only possible verdicts are *refuted by saturation* or
*unknown*, so

```
    infinite-only  ⊆  (the 814)  ∪  (the 1062)
```

Both sides are now bounded from above by the finite witnesses exhibited:

```
    |infinite-only ∩ 814|   ≤  814 − 379  =  435
    |infinite-only ∩ 1062|  ≤ 1062 − 411  =  651
```

Taking the paper's 820 as the size of the infinite-only set,

```
    |infinite-only ∩ 1062|  ≥  820 − 435  =  385
```

**At least 385 of the 1062 undecided implications require an infinite model.**
Janota's figure of 310 is below that floor and cannot be a count of the closed
set. The likeliest reading is that it counts pairs *tagged* in the ETP data as
having an infinite-model proof, which is a property of the formalized
generating set rather than of its closure under transitivity and duality —
the same generating-versus-closure distinction that makes 10,657 formalized
positive implications stand for 8,173,585 real ones.

The bound is tight enough to be useful and loose enough to be honest: it does
not pin the number, it rules out the published one.

## What this changes downstream

The 646 pairs the corpus does not reach are the interesting remainder, and this
partition says they are not uniformly infinite: at most 651 of the 1062 can be
infinite-only, and 385 of those are forced, leaving a band of a few hundred
pairs that may well have finite models nobody has searched for at the right
carrier size. That is a cheaper target than greedy constructions, and it
reorders the sprint queue in favour of structured finite search before S3.

## Not settled

The exact 820 was not recomputed. It needs the full generating set of
finite-only implications closed under transitivity and duality, and that set is
scattered: `FiniteImplicationSearch/theorems/Inverses1.lean` carries 76 pairs
and `InversesManual.lean` three more, while `full_entries.json` at this commit
indexes only five finite implications, all from `InfModel.lean`. No single
file is the canonical source and completeness could not be verified, so the
closure was not attempted. The bounds above do not depend on it beyond taking
820 from the paper.

## Artifacts

- `data/etp/saturation_refuted.txt` — the 814, with the method that refuted each
- `data/etp/hard_core_partition.tsv` — every pair in both sets, labelled with
  the smallest witnessing carrier
