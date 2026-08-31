# A solver with nothing borrowed and nothing looked up

`solver.py`, 986 lines, 35 KB. **196/200** on a local 200-problem sample set.
Three of the four categories are perfect; only order-5 is short, at 46/50. The
build it replaced — 4,323 lines carrying the competition's reference solver and
two embedded lookup tables — scored 190.

|  | self-contained | the build it matches |
|---|---|---|
| lines | 986 | 4,323 |
| bytes | 35,736 | 213,536 |
| reference-solver code | none | 2,268 lines |
| embedded tables | none | 80,746 chars base64 |
| LLM calls over 200 problems | 0 | 0 |
| order4_normal / hard / extra_hard / order5 | **50 / 50 / 50 / 46** | 48 / 47 / 50 / 45 |
| wall clock, 200 problems | 16.5 min | — |

## Two mechanisms, both search

**Finite model search** settles every `verdict: false` problem in the set. Fill
the Cayley table cell by cell; after each assignment, check the instances of the
hypothesis that have become determined, and prune the subtree the moment one
fails. That is the whole trick — the 4^16 tables on `Fin 4` are never
enumerated. Carriers used: 43 at `Fin 2`, 28 at 3, 27 at 4, 1 at 5.

**Completion** settles the true ones. Overlap the hypothesis with itself,
collect critical pairs — **smallest equation first**, which is the single
biggest lever here — and look for a derived equation that *is* the goal
under a substitution. Where the goal has compound terms on both sides, combine
two derived equations `v = T1` and `v = T2` into `T1 = T2`, a family a single
equation cannot express. There is no search over the goal at all: these laws read
`x = C[x,..]`, so every consequence keeps a variable on one side, and a goal
`x = <compound>` is discharged the moment some derived side matches the
compound. When that fails, a bounded meet-in-the-middle rewrites the goal with
the derived equations instead.

## Why there is no lookup table

An earlier build embedded the ETP implication graph as a 4694 × 4694 bit matrix
and consulted it to decide whether to search for a counterexample or a proof.
Removing it cost nothing but wall clock. Measured over 30 known-true
implications, the model finder never once wrongly reported a counterexample —
it spends its budget and gives up, which is exactly what the ladder needs. The
table was routing, not capability.

That also removes a rule obligation: a solver carrying a compressed payload
must disclose it in a submission note. This one has no payload.

## Axioms

The true certificates depend on **no axioms at all** — `#print axioms` returns
the empty list, not `[propext, Classical.choice, Quot.sound]`. They are chains
of `congrArg` and `Eq.symm` over the hypothesis, so nothing classical is
reachable. The false certificates use `decide` on a finite carrier and so bring
in the usual three, which are the permitted set.

## The invariant

Every derived equation is `(lhs, rhs, steps)`, where `steps` is a sequence of
hypothesis applications replaying `lhs` to `rhs`. A lemma is a recorded
derivation, never an assumption. Paths are replayed internally before any Lean
is written, and one that does not replay is discarded rather than emitted, so a
bug in the search surfaces as a missing proof and never as a wrong one.

## What it does not settle

Four of the 200, all order-5 laws outside the ETP's resolved graph. Measured, the best derived equation structurally matches 7 to 9 of a goal's 11
nodes and stalls there. Six thousand derived equations, term-size ceilings from
13 to 21, goal-biased selection, five-step goal rewriting and singleton
detection all return nothing on them.

The honest gap is that this is completion without simplification: derived
equations are never normalised against one another, so the set fills with
reducible consequences. A proper unfailing-completion loop with a Knuth-Bendix
ordering and forward and backward simplification is the standard answer and is
not implemented here. And the search is not saturating, so a negative means
"not found within the budget", never "no proof exists".

Readable, unprefixed sources are in [../prover](../prover).
