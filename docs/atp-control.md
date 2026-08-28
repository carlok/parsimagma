# Control experiment — why the residual resisted Vampire

Sprint 2. The question: is the 1062-pair residual hard mathematics, or an
artifact of how MACE-style finite model building searches?

The answer is neither of the two things I expected. It is not a time budget,
and it is not the incremental escalation of domain sizes. **Finite model
building on these problems falls off a cliff between carrier size 9 and 11,
and telling it exactly which size to try does not help.**

## Setup

Vampire 5.1.0 (arm64, CaDiCaL 2.1.3) against Janota's 5.0.0. Problems emitted
by `pm tptp` in the CNF shape his generator used: the hypothesis as an axiom
over TPTP variables, the conclusion negated and Skolemised.

```
% E1076 => E1455
cnf(lhs, axiom, X0 = m(X1,m(m(X0,m(X0,X1)),X1))).
cnf(rhs, negated_conjecture, sk0 != m(m(sk0,sk1),m(sk1,m(sk1,sk1)))).
```

Sanity check first: `E2 => E3` returns Unsatisfiable, i.e. proved, as it must.
Samples drawn with seed 20260828 from `data/etp/hard_core_partition.tsv`, whose
"witness size" is the smallest carrier the construction corpus can witness the
pair on.

## Result 1 — the S1 mechanism, confirmed directly

Eight pairs from the 814 saturation-refuted set whose smallest corpus witness
has seven elements, run with default `-sa fmb`:

```
8 / 8 solved, 0.4 to 2.1 seconds
```

These are pairs Janota's run recorded as refuted *without a witnessing model*,
which Phase 0 read as evidence they might be infinite-only. They are not. Given
a normal budget the model builder finds a model in under a second. The reason
his run did not is method ordering: `vsi500` (saturation) runs before the
longer fmb budgets, and a pair it refutes is marked solved and never retried.
S1 inferred this from the data; this confirms it on the prover.

## Result 2 — the cliff

Same configuration, samples stratified by witness size:

| smallest witness | solved in 60s | times |
|---|---|---|
| 7 elements | **8 / 8** | 0.4–2.1 s |
| 9 elements | **7 / 8** | 0.1–1.4 s |
| 11 elements | **0 / 8** | — |
| 13 elements (E1076, E2531) | **0 / 20** | — |

Below the cliff it is nearly instant. Above it, nothing. There is no middle
band of "slow but eventually".

Raising the budget does not move it. Four problems at 300 seconds with default
escalation: 0 / 4.

## Result 3 — it is not the escalation

This is what I got wrong going in. I predicted the bottleneck was fmb's
incremental walk up the domain sizes, and that pinning the size would fix it.

Twenty problems with a known 13-element model, run with `-fmbss 13`:

```
0 / 20 solved in 60s
```

Vampire's own trace shows it entering the right instance and stalling inside
the solver, not wandering through smaller sizes:

```
% TRYING [13]
% Time limit reached!
% Termination phase: Finite model building SAT solving
```

The same holds for `E1286 => E3` at `-fmbss 11`, whose 11-element witness is
the one the ETP paper itself gives in Example 5.2 as `(p,a,b) = (11,1,7)`.

So the wall is the SAT instance at that domain, not the search over domains.

## What this means for the residual

The 1062 is not a set of uniformly deep problems. It is, in large part, the set
of separations whose smallest counterexample sits just past where MACE-style
model building stops working — which on this evidence is around 10 elements for
magma laws of order 4.

A structured scan over linear magmas finds those models in about three seconds
for the whole 4694-law set, because it never searches: it solves polynomial
identities in two coefficients and reads the answer off. Two methods, the same
models, incomparable cost.

That is a benchmark-calibration datapoint of exactly the kind the ETP paper
says it wanted and could not produce: *"The objective of using the data from the
ETP to establish well-calibrated benchmarks to evaluate ATPs remains an
interesting open problem; the participants of this project did not have the
required expertise."* The residual is a good benchmark only if one states which
part of it is deep and which part is a model-finder's domain ceiling.

## Caveats, stated plainly

- One prover, one version, one machine. Vampire 5.1.0, not Janota's 5.0.0.
- fmb defaults except for start size. `--fmb_symmetry_ratio` and
  `--fmb_enumeration_strategy contour` were not tried, and either might move the
  cliff. Until they are, "MACE-style model building" should be read as "Vampire
  5.1.0's fmb with default options".
- Cells hold 8 to 20 problems, not hundreds.
- "Witness size" is the smallest carrier *in this corpus*. A smaller model may
  exist that neither the corpus nor fmb found — which would only sharpen the
  puzzle for the size-11 rows.
- The one size-9 failure, `907 => 843`, sits with the other E907 problems that
  fail at 11, so there is a per-law effect on top of the size effect that this
  sample is too small to separate.

## Raw output

`out/atp/by_witness_size.tsv`, `out/atp/d13_default_60s.tsv`,
`out/atp/d13_fmbss13_60s.tsv`, `out/atp_fixedsize300.tsv`. Problems in
`out/tptp_*/`. Regenerate with `pm tptp <pairs-file> <out-dir>`.
