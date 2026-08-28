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

## What it measured

**The residual is not uniformly hard.** Of the 1062 implications Vampire left
unresolved, at least 411 have finite counterexamples on 9 to 32 elements. Three
independent solvers cannot find them — Vampire's `fmb` with CaDiCaL, Mace4, and
z3 on a fully ground encoding all fail from carrier 11 upward while solving
carrier 7 and 9 in under two seconds. A structured algebraic sweep finds them
in about three seconds for the whole law set, because it solves polynomial
identities in two coefficients instead of searching a space of `13^169` tables.
Details and caveats: [docs/atp-control.md](docs/atp-control.md).

**A published count sits below a provable floor.** At least 385 of those 1062
require an infinite model; the figure in circulation is 310. The two count
different things — generating set against its closure — and the bound is
derived in [docs/hard-core-anatomy.md](docs/hard-core-anatomy.md).

**The ETP paper's section 5.1 does not reconcile with itself.** It reports
13,632,566 refutations from 524 magmas, of which 13,345,053 from size 3 and
"the remaining 415,293" from size 4. Those addends sum to 13,760,346. The
project's own `All4x4Tables/README.md` carries corrected figures, and this
engine reproduces them independently: **12,560,783 / 13,596,121 / 13,753,982**,
with **523** magmas sufficing.

**Coverage of the hard core is 416 of 1062**, from three construction families,
and 13 instances suffice for all 416 out of 6005 distinct rows. Widening the
grids thirtyfold adds nothing, so the binding constraint is the set of
construction *kinds*, not compute. See
[docs/phase-a-report.md](docs/phase-a-report.md).

**143 open ETP questions were scanned and none fell.** The negative is
structured rather than blind, and it names the only ring class that could still
work: [docs/open-questions-scan.md](docs/open-questions-scan.md).

**Two further families were built and both added nothing.** Translation-invariant
magmas over `Z/n`, 523 million candidates across `n = 2..12`, reached 16
hard-core pairs and no new ones. Quadratic magmas over `Z/N`, 11.5 million
instances for `N = 2..13`, reached 93 and no new ones. Neither is redundant as a
*family* — four of the six hitting permutations at `n = 9` are genuinely
nonlinear — but their coverage coincides with what linear models already reach.
The binding constraint on the coverage number is which *kinds* of construction
exist, not how wide any grid is swept.

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
| [docs/hard-core-anatomy.md](docs/hard-core-anatomy.md) | what the 1062 actually contains, and the 385 floor |
| [docs/phase-a-report.md](docs/phase-a-report.md) | engine, differential agreement, families implemented and not, coverage totals |
| [docs/open-questions-scan.md](docs/open-questions-scan.md) | 143 open questions, a structured negative |
| [docs/cluster-296.md](docs/cluster-296.md) | what the largest uncovered cluster wants, recovered from the ETP issue history |
| [docs/translation-invariant.md](docs/translation-invariant.md) | `x ◇ y = x + f(y - x)`, 523M candidates, no new coverage |
| [docs/quadratic.md](docs/quadratic.md) | `ax² + bxy + cy² + dx + ey + f`, 11.5M instances, no new coverage |
| [docs/phase0-findings.md](docs/phase0-findings.md) | prior art, and the four distinct "hard core" counts |
| [notes/](notes/) | working state: sprint queue and checklist |

Apache-2.0, matching upstream. See `NOTICE`.
