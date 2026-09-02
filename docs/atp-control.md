# A domain-size cliff in finite model finding, and what it means for the ETP benchmark

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

**Claim.** The 1062 implications left unresolved by Vampire in the Equational
Theories Project are not uniformly hard. At least 411 of them have finite
counterexamples on 9 to 32 elements. Those counterexamples are out of reach of
every search-based model finder tested — Vampire's `fmb` with CaDiCaL, Mace4,
and z3 on a fully ground encoding all fail at carrier 11 and above, while
solving carrier 7 and 9 in under two seconds — and are found in about three
seconds for the whole 4694-law set by solving polynomial identities in two
coefficients instead of searching.

Anyone using the ETP residual to calibrate a prover should partition it first.

The ETP paper asks for exactly this: *"The objective of using the data from the
ETP to establish well-calibrated benchmarks to evaluate ATPs remains an
interesting open problem; the participants of this project did not have the
required expertise to develop and test such benchmarks."*

Everything below is reproducible from this repository. Provenance and
regeneration steps are in `data/etp/PROVENANCE.txt`.

---

## 1. The cliff

Problems are ordered pairs `(E_i, E_j)` from the 1062, asking for a magma
satisfying `E_i` and refuting `E_j`. They are stratified by the smallest
carrier on which the construction corpus in this repository exhibits such a
magma, sampled with a fixed seed (`scripts/sample_control_pairs.py`).

| smallest witness | Vampire `fmb` | Mace4 | z3, ground | z3, quantified |
|---|---|---|---|---|
| 7 elements | **8 / 8**, 0.4–2.1 s | 0 / 4 in 60 s | **8 / 8** | — |
| 9 elements | **7 / 8**, 0.1–1.4 s | — | **6 / 8** | — |
| 11 elements | 0 / 8 | 0 / 8 | 0 / 8 | — |
| 13 elements | 0 / 20 | — | 0 / 20 | 0 / 1 at 100 s |

Three independent technologies, one cliff, in the same place.

Nothing moves it:

- **More time.** 300 s instead of 60 s with default escalation: 0 / 4. Janota's
  own run already shows the saturation: his `fmb` at 60 s added 16,302
  refutations over the instruction-capped run, and `fmb` at 600 s added
  **28** more.
- **Telling it the answer.** `-fmbss 13` on twenty problems with known
  13-element models: 0 / 20. Vampire's trace shows it entering the right
  instance and stalling inside the solver, not wandering through smaller sizes:
  `% TRYING [13]` … `% Termination phase: Finite model building SAT solving`.
- **Tuning `fmb`.** `--fmb_enumeration_strategy contour` and
  `--fmb_symmetry_ratio 4`, at fixed domain: 0 / 18.
- **Removing the model finder entirely.** A fully ground SMT encoding — the
  hypothesis instantiated at all `n^k` carrier tuples, 169 equations for a
  two-variable law at `n = 13`, 177 lines of SMT-LIB2 — also fails at 180 s.

So this is not an artifact of MACE-style search, of one prover's encoding, or
of quantifier instantiation.

### The encoding is not the problem, and here is the proof

`E1076 ⊭ E1455` is witnessed by `Z/13` under `x ◇ y = 7x + 7y`. Pinning that
table into the same ground SMT file as 169 extra equations:

```
sat
real 0.01s
```

The solver accepts the model instantly and cannot find it in 180 seconds. The
instance is satisfiable, the encoding is faithful, and the search still fails.

## 2. What finds them instead

In a linear magma `x ◇ y = ax + by` over a ring, every word is
`Σ_i P_{w,i}(a,b)·x_i`, where `P_{w,i}` sums one word in `{a,b}` per occurrence
of `x_i` — the root-to-leaf path to it. Since the carrier is the whole unital
ring, the law holds **iff** `P_{w1,i} = P_{w2,i}` for every `i`. A law of order
4 has depth at most 4, so there are 31 possible words and a difference
polynomial is a fixed 31-slot array with a handful of nonzero terms. Deciding
one instance costs 31 ring multiplications and a sparse scan; no carrier is
enumerated at all.

Sweeping `Z/m` for `m = 2..32` and all coefficient pairs — 11,439 instances —
takes about three seconds and discharges 357 of the 1062. The rest of the
corpus brings it to 416.

The two methods are not competing at the same task. One searches a space of
`13^169` multiplication tables. The other solves two polynomial equations.

## 3. Which part of the residual is which

Labelling every pair in the 1062 with the smallest carrier the corpus witnesses
it on:

```
finite  9        4
finite 11      101
finite 13      268
finite 32       38
infinite only    5
uncovered      646
```

A companion set matters here too. Janota's run records 814 pairs refuted by
**saturation** rather than by model building, so with no witnessing model
produced. That looks like evidence of infinite-only behaviour and is not:

```
saturation-refuted, 814 pairs
  finite  7      329
  finite  9       34
  finite 11       16
  infinite only    2
  uncovered      433
```

379 of them have finite counterexamples, 329 on seven elements, and Vampire's
`fmb` finds those in under a second when asked. The reason his run did not is
method ordering: `vsi500` runs before the longer `fmb` budgets, and a pair it
refutes is marked solved and never retried.

### A published count that cannot be right

Every pair with no finite counterexample is unrefutable by a model builder and
unprovable by saturation, so its only possible verdicts are *saturation-refuted*
or *unknown*:

```
    infinite-only  ⊆  (the 814)  ∪  (the 1062)
```

The finite witnesses above bound each side:

```
    |infinite-only ∩ 814|   ≤  814 − 379  =  435
    |infinite-only ∩ 1062|  ≤ 1062 − 411  =  651
```

The ETP paper (§8) puts the number of pairs with `E ⊭ E'` but `E ⊧_fin E'` at
820. Therefore

```
    |infinite-only ∩ 1062|  ≥  820 − 435  =  385
```

**At least 385 of the 1062 require an infinite model.**

> **Superseded.** This bound, and the reading of 310 that follows it, are both
> obsolete. The ETP's own finite implication graph is fetchable, and decoding it
> gives the split exactly: **610 infinite-only, 450 finite, 2 open** — see
> [infinite-only-exact.md](infinite-only-exact.md). The floor argument is kept
> here because it was how the question was approached before the graph was
> available, not because 385 is the answer.
>
> The 310 comparison was also unsound as stated, and the denominators are the
> reason. The 820 counts ordered pairs across the whole 4694² closed graph;
> 310 is quoted against the 1062 residue and appears to count dual classes.
> Recomputing dual orbits over the residue gives 316, or 294 if both members of
> a pair must lie inside it. So 310 is a dual-class count against an earlier
> snapshot rather than an error, and the claim that it sits below a provable
> floor is withdrawn. Pointed out by Wenlin Zhang.

## 4. Consequences for benchmark use

- Quoting 1062 as a difficulty measure overstates it. Roughly 40% of it is
  reachable, and a further slice of the 646 uncovered pairs may be too.
- A benchmark built from this residual should be split into *finite model at
  carrier ≥ 10*, *infinite model required*, and *unknown*. The first tests a
  capability current model finders do not have; the second tests something
  else entirely.
- The finite-model-search subset is a good hard-SAT benchmark precisely
  because it is satisfiable, small to state, and resists three solvers.
- For practitioners: on this problem family, a structured algebraic sweep
  dominates search by orders of magnitude, and it is twenty lines of
  arithmetic.

## 5. What is not established

- Four solvers, single versions: Vampire 5.1.0 with CaDiCaL 2.1.3 (Janota used
  5.0.0), Mace4 2009-11A, z3 as installed. No claim about others.
- Cells hold 8 to 20 problems. The cliff is sharp enough that the sample size
  is not the binding uncertainty, but it is not a large study.
- "Witness size" is the smallest carrier *in this corpus*. Smaller models may
  exist that neither the corpus nor any solver found, which would sharpen the
  puzzle rather than soften it.
- One size-9 problem, `907 ⊭ 843`, fails where its stratum-mates succeed, and
  the other `E907` problems fail at 11. There is a per-law effect on top of the
  size effect that this sample cannot separate.
- The 820 was taken from the paper, not recomputed. Recomputing it needs the
  full generating set of finite-only implications closed under transitivity and
  duality, and that set is scattered across the Lean development with no
  canonical index.

## 6. Reproducing

```bash
cargo test --release
```

```bash
./target/release/pm partition --order3-twists
```

```bash
python3 scripts/sample_control_pairs.py
```

```bash
./target/release/pm tptp out/atp/samples/witness11.txt out/tptp_w11
```

```bash
PM_GROUND=1 ./target/release/pm smt2 out/atp/samples/witness11.txt out/smt_g11 11
```

Results as run: `out/atp/by_witness_size.tsv`, `out/atp/d13_default_60s.tsv`,
`out/atp/d13_fmbss13_60s.tsv`, `out/atp/fmb_variants.tsv`,
`out/atp/z3_ground.tsv`, `out/atp_fixedsize300.tsv`.
Partition: `data/etp/hard_core_partition.tsv`.
