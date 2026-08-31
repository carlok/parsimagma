# parsimagma

![ci](https://github.com/carlok/parsimagma/actions/workflows/ci.yml/badge.svg)

A signature and coverage engine over the 4694 equational laws of the
[Equational Theories Project](https://teorth.github.io/equational_theories/),
built to answer one question: **for the constructions the ETP used, which of
them cover which separations?**

**Produced by an AI agent (Claude) under human direction.** Everything below is
a command in this repository. The derivations and the code are machine-generated
and machine-checked, and are offered as data and reproduction steps rather than
as a proof.

**Published:** the solver built from this corpus is public on the SAIR
Contributor Network as
[`parsimagma-greedy-cover`](https://competition.sair.foundation/contributor-network/mathematics-distillation-challenge-equational-theories-stage2/EQT02-S00025)
(`EQT02-S00025`). Source in [dist/](dist/).

## What it measured

**The residual is not uniformly hard.** Of the 1062 implications Vampire left
unresolved, at least 411 have finite counterexamples on 9 to 32 elements. Three
independent solvers cannot find them — Vampire's `fmb` with CaDiCaL, Mace4, and
z3 on a fully ground encoding all fail from carrier 11 upward while solving
carrier 7 and 9 in under two seconds. A structured algebraic sweep finds them
in about three seconds for the whole law set, because it solves polynomial
identities in two coefficients instead of searching a space of `13^169` tables.
Details and caveats: [docs/atp-control.md](docs/atp-control.md).

**The 1062 is now split exactly.** 610 of them require an infinite model, 450
have a finite counterexample, and 2 are the only implications the ETP left open.
The corpus witnesses 411 of the 450, so its coverage of the finitely refutable
hard core is 91%, not the 39% a 1062 denominator suggests. Derived in
[docs/infinite-only-exact.md](docs/infinite-only-exact.md) from the ETP's own
finite implication graph, which also validates 790 of this corpus's finite
witnesses with zero disagreements. The earlier floor of 385 in
[docs/hard-core-anatomy.md](docs/hard-core-anatomy.md) held; the figure of 310 in
circulation is most likely a dual-class count, not an error.

**The ETP paper's section 5.1 does not reconcile with itself.** It reports
13,632,566 refutations from 524 magmas, of which 13,345,053 from size 3 and
"the remaining 415,293" from size 4. Those addends sum to 13,760,346. The
project's own `All4x4Tables/README.md` carries corrected figures, and this
engine reproduces them independently: **12,560,783 / 13,596,121 / 13,753,982**,
with **523** magmas sufficing.

**The binding constraint is the set of construction kinds, not compute.**
Thirteen instances suffice for the 416 hard-core pairs the corpus reaches, out
of 6005 distinct rows, and widening the grids thirtyfold adds nothing. See
[docs/phase-a-report.md](docs/phase-a-report.md) — whose 39.2% headline is
measured against the wrong denominator, corrected above.

**Magma extensions close 16 of the last 39, and provably cannot close the rest.**
The residual wants §5.6 of the paper — the blueprint's "Magma cohomology"
chapter — not the greedy constructions the report assumed. Implementing it takes
the hard core from 411 to **427 of 450**, and two of the sixteen land on exactly
the ETP's own parameters for `Refutation938`, found from the ingredients rather
than read off the table. For the remaining 23 the question is decidable rather
than searchable, and the answer is no across every viable setting: 661,142 fibre
candidates, zero further pairs. [docs/the-39.md](docs/the-39.md).

**A first coverage measurement on the order-5 laws.** The ETP maps order ≤ 4;
`pm order5` sweeps the 62,576 laws of order ≤ 5 and discharges **2,299,094,885
of 3,915,693,200 ordered pairs**, each witnessed by an explicit finite magma.
It has no oracle for the pair count, which the write-up says at length, but it
does have one for model existence: 19,522 laws of hard agreement, zero
disagreements. [docs/order5-first-map.md](docs/order5-first-map.md).

**143 open ETP questions were scanned and none fell.** The negative is
structured rather than blind, and it names the only ring class that could still
work: [docs/open-questions-scan.md](docs/open-questions-scan.md).

**Three further families were built and all three added nothing.**
Translation-invariant magmas over `Z/n`, 523 million candidates across
`n = 2..12`, reached 16 hard-core pairs and no new ones. Quadratic magmas over
`Z/N`, 11.5 million instances for `N = 2..13`, reached 93 and no new ones.
Linear magmas over `M_2(F_5)` and `M_3(F_2)`, carriers of 625 and 512 elements
and 652,769 coefficient pairs, reached 14 and no new ones. None is redundant as
a *family* — four of the six hitting permutations at `n = 9` are genuinely
nonlinear — but their coverage coincides with what linear models already reach.
The binding constraint on the coverage number is which *kinds* of construction
exist, not how wide any grid is swept. `pm transinv`, `pm quad`, `pm matring`.

## Checking it

```bash
cargo test --release
```

32 tests. Exact agreement with the ETP's own data on 824 magmas across all 4694
bits, on the implication graph, on the paper's published coefficient varieties,
on its 3068 full-spectrum count, and on OEIS A001329.

```bash
cargo run --release --bin pm -- mincover
```

```bash
cargo run --release --bin pm -- coverage
```

```bash
cargo run --release --bin pm -- bruteforce 4
```

The last sweeps 4,294,967,296 tables, 178,981,952 after isomorphism filtering,
in about 83 minutes on 11 cores at 75 MB resident.

## How it works

For a magma `M`, its **signature** is the 4694-bit vector recording which laws
`M` satisfies — 587 bytes. Every separation question becomes a bit test: `M`
separates `E_i` from `E_j` exactly when `sig[i] && !sig[j]`. One signature
answers all 22,028,942 ordered pairs, and coverage over a corpus is bitset
algebra over packed words.

**Tier S (symbolic)** decides constructions with infinite carriers, which no
table can check. In a linear magma `x ◇ y = ax + by` over a ring, every word is
`Σ_i P_{w,i}(a,b)·x_i` where `P_{w,i}` sums the root-to-leaf path word of each
occurrence of `x_i`. Since the carrier is the whole unital ring, the law holds
**iff** those coefficient polynomials agree — so the family reduces to
polynomial identities and never enumerates a carrier. This is what reaches
`E1117 ⊭ E2441`, a separation with no finite counterexample at all.

**Tier F (finite)** sweeps `n^v` assignments against one shared subterm DAG
covering all 4694 laws at once: 3777 distinct subterms against 18,311
independent applications. Laws are bucketed by arity so a `k`-variable law
sweeps `n^k` rather than `n^6`.

Instances with finite carriers are decidable both ways, and the two routes are
required to agree bit for bit. That cross-check is what makes Tier S
trustworthy, and it is why the load-bearing `Z/13` instance is tested against a
full table sweep.

## Data and reconstruction

`data/etp/` vendors the ETP files used as the differential-test oracle, plus a
few derived from an 84 MB upstream dump that is deliberately not vendored.
`data/etp/PROVENANCE.txt` lists every file, its upstream path, the pinned
commit, and for the derived ones the recipe and the sha256 of the input.

The two largest files are stored gzipped — the implication bit matrix
compresses 47x — and the loader accepts either form, so nothing in the build or
test path notices.

Generated ATP problem files are not tracked, being a pure function of a sample
list and the law set:

```bash
python3 scripts/sample_control_pairs.py
```

External tools used only by the control experiment, neither needed to build or
test: Vampire 5.1.0 and Prover9/Mace4, both in Homebrew.

## Documents

| | |
|---|---|
| [docs/atp-control.md](docs/atp-control.md) | the domain-size cliff, four methods, and what it means for benchmark use |
| [docs/infinite-only-exact.md](docs/infinite-only-exact.md) | the 1062 split exactly: 610 infinite-only, 450 finite, 2 open |
| [docs/order5-first-map.md](docs/order5-first-map.md) | first coverage measurement on the 62,576 order-5 laws, and why it has no oracle |
| [docs/merge-experiment.md](docs/merge-experiment.md) | merging the solver with the competition's reference: 55/200 to 172/200, none of it the corpus |
| [docs/the-39.md](docs/the-39.md) | what the 39 remaining pairs want, and the section 5.6 family that closes sixteen of them |
| [docs/hard-core-anatomy.md](docs/hard-core-anatomy.md) | what the 1062 actually contains, and the 385 floor (superseded) |
| [docs/phase-a-report.md](docs/phase-a-report.md) | engine, differential agreement, families implemented and not, coverage totals |
| [docs/open-questions-scan.md](docs/open-questions-scan.md) | 143 open questions, a structured negative |
| [docs/cluster-296.md](docs/cluster-296.md) | what the largest uncovered cluster wants, recovered from the ETP issue history |
| [docs/translation-invariant.md](docs/translation-invariant.md) | `x ◇ y = x + f(y - x)`, 523M candidates, no new coverage |
| [docs/quadratic.md](docs/quadratic.md) | `ax² + bxy + cy² + dx + ey + f`, 11.5M instances, no new coverage |
| [docs/phase0-findings.md](docs/phase0-findings.md) | prior art, and the four distinct "hard core" counts |
| [docs/where-a-wrong-sum-survives.md](docs/where-a-wrong-sum-survives.md) | a short note on which artifacts get checked, with the specifics |
| [notes/](notes/) | working state: sprint queue and checklist |

Apache-2.0, matching upstream. See `NOTICE`.
