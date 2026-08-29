# Phase A report — magma signature and coverage engine

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

Date: 2026-08-27/28. Toolchain pinned in `rust-toolchain.toml` (Rust 1.96.0).
Every number below is reproduced by `cargo test --release`,
`./target/release/pm stats`, `pm coverage`, and `pm bruteforce N`.
Raw outputs are in `out/`. The size-4 sweep referenced below has since
completed; see section 6.

---

## 1. The number that decides whether to continue

**416 of the 1062 hard-core separations (39.2%) are discharged by the
implemented corpus, and the redundancy in reaching them is extreme: 13
instances suffice for all 416, out of 6005 distinct rows.**

> Since this report was written, two further families were built and measured:
> translation-invariant magmas ([docs/translation-invariant.md](translation-invariant.md))
> and quadratic magmas ([docs/quadratic.md](quadratic.md)). Both add zero
> hard-core coverage, so the 416 stands. The family count below should be read
> as five implemented, three contributing.

> **The 39.2% is measured against the wrong denominator.** See
> [docs/infinite-only-exact.md](infinite-only-exact.md): the ETP's finite
> implication graph, decoded from the Equation Explorer's `finite_graph.json`,
> shows that **610 of the 1062 require an infinite model** and so cannot be
> discharged by any corpus of finite magmas. 450 have a finite counterexample and
> 2 remain unknown to the project. The corpus witnesses 411 of the 450 with a
> finite magma and reaches 5 of the 610 with its infinite families, which is
> where the 416 comes from. Against the part that is finitely refutable at all
> the figure is **411 of 450, 91%**, and the outstanding target is 39 pairs, not
> 646. The paragraphs below are left as written; read 39.2% as a statement about
> the whole hard core, not about what a finite corpus could reach.

The hard core is set (B) from the Phase 0 report, as pinned: the 1062
implications Vampire left undecided in Janota's exhaustive run
(arXiv:2508.15856). Vampire proved every implication that holds, so all 1062
are false and all of them want a counterexample.

By the brief's own decision rule this is the "coverage is poor, the family
library is incomplete, the fix is mathematics not compute" branch. Three
findings sharpen that.

**The 416 come from four instances.** The linear magma `x ◇ y = 7x + 7y` over
`Z/13` discharges 268 pairs on its own; two more `Z/13` instances add 89; the
paper's own twisted power — `F_2` under NAND raised to the fifth power and
twisted by the left and right shifts — adds 38. Everything else in a
165,969-instance grid contributes 21 pairs between them.

**Those pairs are not mathematically hard.** A 13-element multiplication
table is a finite counterexample. Vampire's finite model builder searches
domain sizes upward and did not reach 13 within 500 instructions, 60 seconds,
or 600 seconds. So a third of the "hard core" is a search-budget artifact of
MACE-style model finding at domain size 9 and above, not evidence of
mathematical depth. Anyone quoting 1062 as a measure of difficulty is
overstating it, and the coverage matrix is what makes that visible.

**Coverage saturates hard, twice over.** Widening the modulus grid from
`m ≤ 32` to `m ≤ 96` — 354,247 linear instances instead of 11,439 — discharges
exactly the same pairs. Adding 96,570 twisted powers over three-element bases
produces 3219 new distinct signatures and raises whole-graph coverage by three
percentage points, and discharges **zero** additional hard-core pairs. Compute
is not the binding constraint on this number; the set of construction *kinds*
is.

Against the whole graph rather than the hard core, the corpus discharges
**13,834,667 of the 13,855,357 false implications (99.85%)**. The contrast looked
like the finding when this was written: a corpus within 0.15% of complete against
the graph, at 39% against the part that resisted a good prover. It does not
survive the correct denominator. 610 of those 1062 admit no finite counterexample
at all, so the comparable figure is 411 of 450 — 91% — and the gap between the
two settings is far smaller than this section claims.

### What the remaining 646 want

> Of these 646, **605 admit no finite counterexample** and are unreachable by any
> finite construction; 39 have one and are the real target; 2 are `E677 ⊭ E255`
> and its dual, still open. The per-law table below counts all 646 together, so
> its "uncovered" column overstates what is actually missing. The 39 are listed
> in `data/etp/finite_uncovered.txt`.

| hypothesis law | uncovered | covered | ETP's method (blueprint ch. 27) |
|---|---|---|---|
| E1133, E1167, E1659, E1661, E1979, E2000, E2473, E2481 | 37 each, 296 total | 0 | not in the curated list; greedy or ad hoc |
| E1076 / E2531 (duals) | 40 each | 134 each | greedy |
| E1924, E1648 | 15 each | 0 | greedy |
| E1895, E1692 | 11 each | 0 | greedy |
| E1485 / E2162 (duals) | **0** | 19 each | twisting semigroup, §5.4 — implemented, cluster closed |

Greedy constructions (paper §5.5) dominate what is missing, which is what the
Phase 0 report predicted: they carry most of the blueprint's hard list and
they are the one family with **no finite counterpart at all**, which is
exactly why `E677 ⊧_fin E255` remains the last open finite implication. A
family library without them cannot reach the hard core, and no amount of
grid-widening substitutes.

The E1485 row is the method working. That cluster was uncovered at the linear
stage; the paper names the twisting semigroup as its method and says the
separation "does not seem to be easily refuted by any of the other methods
discussed"; implementing §5.4 closed all 38 pairs with one instance. The same
loop — read which method the ETP used for an uncovered cluster, implement it,
watch the cluster fall — is what the remaining 646 pairs are waiting for. That
is mathematics, not machine time.

---

## 2. Engine

One shared subterm DAG across all 4694 laws, interpreted as a flat array of
`(op, lhs, rhs)` triples — not generated code, so the law list stays a runtime
input and pointing the engine at `eq_size5.txt` needs no recompile.

| | |
|---|---|
| laws | 4694 |
| applications across all laws | 18,311 |
| distinct subterms after hash-consing | **3777** |
| sharing factor | 4.8x |
| signature width | 4694 bits, 587 bytes |

Laws are bucketed by how many variables they use (31 / 779 / 2090 / 1447 / 325
/ 22 for one through six variables), and each bucket sweeps only `n^k`
assignments against the sub-DAG it needs. A single six-variable sweep would
waste a factor of `n^2` on the 4347 laws using four variables or fewer.
Assignments are visited on a golden-ratio stride rather than lexicographically,
so violations surface early; the sweep abandons a bucket as soon as every law
in it is refuted.

**Throughput** (single thread, M-series, no SIMD):

| corpus | signatures/s |
|---|---|
| ETP `All4x4Tables` magmas (`n = 2..4`, biased toward law-rich tables) | 2326 |
| exhaustive `n = 3`, all isomorphism classes, 5 workers | ~4250 |

No SIMD path was written. The brief's instruction was to write the scalar
version, profile it, and hand-vectorise only if the profile demands it: it does
not. The whole hard-core measurement runs in 3.2 seconds, and the one job big
enough to care about throughput — the exhaustive size-4 sweep — is bounded by
4.3 billion table constructions, not by DAG evaluation. NEON `vqtbl4q_u8`
remains available if a size-5 survey is ever attempted; nothing in Phase A
justified it.

---

## 3. Differential testing — agreement rate

**100%. No disagreement anywhere, on any corpus.**

| test | corpus size | result |
|---|---|---|
| full signature vs ETP `All4x4Tables` `Proves` lists | **824 magmas x 4694 bits** | exact |
| published smallest model satisfies its law | 3198 laws | exact |
| smallest-model size histogram vs paper Table 1 | 3136/32/14/14/2 | exact |
| every claimed separation vs the published implication graph | 4022 magmas | no contradiction |
| symbolic vs table route, linear over `Z/m`, `m = 2..5` | 54 instances | bit-identical |
| symbolic vs table route, affine over `Z/m`, `m = 2..5` | 224 instances | bit-identical |
| symbolic vs table route, `Z/13` and `Z/11` load-bearing instances | 4 instances | bit-identical |
| symbolic vs table route, `M_2(F_2)`, noncommuting pairs | 2 instances, 16 elements | bit-identical |
| magmas up to isomorphism (OEIS A001329) | orders 1, 2, 3, 4 | 1, 10, 3330, 178,981,952 |
| `E1` holds always; `E2` holds exactly in singletons | 176 magmas | exact |
| twisted power vs table sweep, `F_2^k`, `k = 2..4` | 29 instances | bit-identical |
| one-coordinate twist reproduces its base magma | 3 instances | bit-identical |

The `All4x4Tables` corpus is the strongest oracle available and is used in
preference to the smallest-magma-per-equation set the brief names: each entry
pairs a table with the *complete* list of laws it satisfies, so agreement is
checked on all 4694 bits rather than on one.

The symbolic-vs-table cross-check is what makes Tier S trustworthy. A linear
model over `Z/m` is simultaneously a symbolic object and a finite magma, so the
two independent routes must produce identical signatures. They do, including
for `Z/13` with `a = b = 7`, the instance carrying 268 of the 416 covered
pairs. If that had disagreed the headline number would be worthless.

### Reproducing published counts

| claim | source | reproduced |
|---|---|---|
| 3068 laws have full spectrum via `a, b ∈ {-1,0,1}` | paper §9 | **3068** |
| variety of E1286 is `1 = ba³ + bab, 0 = a + ba²b + b²` | paper Ex. 5.2 | exact |
| variety of E3 is `a + b = 1` | paper Ex. 5.2 | exact |
| variety of E1117 is `1 = baba, 0 = a + ba², 0 = bab² + b²` | paper Ex. 5.3 | exact |
| variety of E2441 is `1 = a² + aba² + abab + ab² + b` | paper Ex. 5.3 | exact |
| refutations by magmas of size ≤ 2 | ETP repo README | **12,560,783** |
| refutations by magmas of size ≤ 3 | ETP repo README | **13,596,121** |
| refutations by magmas of size ≤ 4 | ETP repo README | **13,753,982** |
| Vampire: 13,854,295 refuted / 8,173,585 proven / 1062 unknown | arXiv:2508.15856 Table 2 | exact, per method |

### A correction to the published paper

The arXiv paper (v2, 16 Dec 2025) §5.1 reports that brute force over magmas of
size at most 4 refutes 13,632,566 implications with 524 distinct magmas, of
which 13,345,053 come from size ≤ 3 and "the remaining 415,293" from size 4.
Those two addends sum to 13,760,346, not 13,632,566, and the stated
"96.3% of the false ones" matches 13,345,053 rather than the headline figure.

The repo carries corrected numbers — 12,560,783 / 13,596,121 / 13,753,982 and
523 magmas — computed by Bruno Le Floch in October 2025 after he found an
off-by-one in the ETP's own `check_redundant.py` (it summed a 4695 x 4695
matrix, counting two equations that do not exist), and independently confirmed
by Douglas McNeil. A correcting PR was announced on Zulip and did not reach the
arXiv version.

This engine reproduces the corrected size-2 and size-3 figures exactly, from an
independently written implementation with no shared code. That makes three
independent confirmations of numbers the published paper still gets wrong.

---

## 4. Construction families

### Implemented

**Finite enumeration** (paper §5.1). Exhaustive over all magmas of a given
carrier size, with exact isomorphism canonicalisation: a table is kept only if
it is the lexicographically least member of its class. The filter costs
`n! · n²` byte comparisons, two orders of magnitude less than a signature, so
it turns the size-4 sweep from 4.29 billion tables into 178,981,952. Exact
canonicalisation is capped at carrier size 6 (`EXACT_ISO_CAP`), because
brute-force canonical form is `n!` and dies past roughly `n = 9`; above the cap
the corpus is deduplicated on the signature instead, which is coarser than
isomorphism but is exactly the equivalence coverage cares about.

**Linear models** `x ◇ y = ax + by` (paper §5.2). The whole family reduces to
polynomial identities in the coefficients, with no carrier enumeration at all.
Every word `w(x_1..x_n)` in a linear magma is `Σ_i P_{w,i}(a,b) · x_i`, where
`P_{w,i}` sums one word in `{a,b}` per occurrence of `x_i` — the root-to-leaf
path to that occurrence, left = `a`, right = `b`, read root-to-leaf so the
product is taken in that order and the construction works verbatim in a
noncommutative ring. Because the carrier is the whole unital ring, setting one
variable to 1 and the rest to 0 shows the law holds **iff** `P_{w1,i} =
P_{w2,i}` for every `i`. A law of order 4 has depth at most 4, so there are
only 31 possible words and a difference polynomial is a fixed 31-slot array
with a handful of nonzero terms. One instance costs 31 ring multiplications
plus a sparse scan.

**Affine models** `x ◇ y = ax + by + c` (paper §5.2). The constant part of a
word is `S_w · c` where `S_w` sums the path word of every *internal* node, so
affine adds exactly one condition, `(S_lhs - S_rhs)·c = 0`, on top of the
linear ones.

**Twisted Cartesian powers** (paper §5.4). Given a magma `M` satisfying `E`
and two endomorphisms `T, U`, the twisted operation `x ◇' y := Tx ◇ Uy`
satisfies `E` again provided `T, U` obey the relations defining the *twisting
semigroup* `Twist_E`; a Cartesian power `M^k` always supplies such
endomorphisms as coordinate shifts. The twisted magma has `n^k` elements — 32
for the paper's worked case — and 22 laws use six variables, so a sweep would
need `32^6 ≈ 1.1e9` assignments. The check decomposes by coordinate instead:
writing `σ`, `τ` for the shifts,

```text
    eval(Var v, i)    = x_v[i]
    eval(Op(l, r), i) = eval(l, σ(i)) ◇_M eval(r, τ(i))
```

so at a fixed root coordinate the law reads at most six entries of the input
tuples, one per leaf, and the law holds exactly when the base identity holds at
every coordinate for every assignment to those entries. That is `k · n^leaves`
base operations per law rather than `n^(k·vars)`.

Rings implemented: `Z/m`; `Z`; `Z[t]`; `M_k(F_p)`; the free commutative
`Z[a,b]` and free noncommutative `Z<a,b>`, whose instances are the *generic*
linear magmas satisfying precisely the laws every linear magma satisfies; and
`Z<a,b>/(ba+1)`, in which `b` is a one-sided inverse of `a` and nothing more.

### Per-family validation against named ETP results

| construction | ETP claim | status |
|---|---|---|
| `Z/11`, `(a,b) = (1,7)` | paper Ex. 5.2: witnesses `E1286 ⊭ E3` | reproduced |
| `Z<a,b>/(ba+1)` | paper Ex. 5.3: witnesses `E1117 ⊭ E2441` | reproduced |
| the same, Remark 5.4 | that separation admits no finite counterexample | no `Z/m` witness in `m ≤ 13`, all `(a,b)` |
| `Z`, `(1,-1)` | abelian group subtraction, Tarski's `E543` | reproduced |
| `Z`, `(-1,1)` | backwards subtraction, `E1090` | reproduced |
| `Z`, `(-1,-1)` | semi-symmetric `E14`, totally symmetric `E492` | reproduced |
| `Z`, `(1,0)` / `(0,1)` / `(0,0)` | projections `E4`, `E5`; constant `E46` | reproduced |
| linear family as a whole | Remark 5.6: `E1485 ⊭ E151` is immune to it | no witness in a 900-instance grid |
| `F_2` NAND, `k = 5`, shifts `(+1, -1)` | paper Ex. 5.11: witnesses `E1485 ⊭ E151` | reproduced |
| the same | `Twist_{E1485}` cyclic of order 5, `Twist_{E151}` of order 2 | `k < 5` does not separate them |

`E1117 ⊭ E2441` matters most: it is in the 1062-pair hard core, it has no
finite counterexample at all, and the instance that discharges it has an
infinite carrier. That is the case Tier S exists for, and the only one in the
implemented corpus that a table-based approach could never have reached.

### Measured, not assumed

On the order-≤4 law set the generic commutative and generic noncommutative
linear magmas have **identical** signatures. Separating them would need a law
placing the same variable at path `ab` on one side and `ba` on the other with
every other variable's paths matching, and no law of order 4 does that. The
noncommutative gap opens for *specialisations* — `M_2(F_3)` and
`Z<a,b>/(ba+1)` both contribute hard-core pairs no commutative instance
reaches — not for the generic models.

### Not implemented

Listed because the brief asks which families were left out, and because the
coverage number is a statement about this list as much as about the one above.

- **Translation-invariant models**, `x ◇ y = x + f(y-x)` (paper §5.3)
- **Greedy constructions** (paper §5.5) — the largest gap by far, and
  inherently infinitary, so it has no finite proxy
- **Submagma, projection, and magma cohomology extensions** (paper §5.6)
- **Free magmas from confluent laws and complete rewriting systems**
  (paper §6) — note this is a *syntactic* method verified by Knuth-Bendix
  completion, not a polynomial-identity method, so it does not belong in the
  same tier as the linear models even though the brief groups them
- **Abelian extensions**
- **Quadratic models** over `Z/N` (paper Remark 5.5)

---

## 5. Coverage matrix

One row per construction *instance*, not per family: a linear model with free
coefficients is an unbounded parametrised set, so "the coverage of the linear
family" has no value. The grid below is finite, is written to `out/grid.txt`,
and every total in this report is against it.

| family | enumerated | distinct signatures |
|---|---|---|
| `linear/Z_m`, `m = 2..32`, all `(a,b)` | 11,439 | 2352 |
| `affine/Z_m`, `m = 2..20`, all `(a,b,c)` | 41,230 | 203 |
| `linear/Z`, `a,b ∈ [-6,6]` | 169 | 3 |
| `affine/Z`, `a,b,c ∈ [-6,6]`, `c ≠ 0` | 2028 | 0 |
| `linear/Z[t]`, `a,b = c0 + c1·t`, `ci ∈ [-2,2]` | 625 | 0 |
| `linear/M2(F2)`, all `(a,b)` | 256 | 31 |
| `linear/M2(F3)`, all `(a,b)` | 6561 | 142 |
| `affine/M2(F2)`, all `(a,b,c)`, `c ≠ 0` | 3840 | 8 |
| `linear/generic`, `Z[a,b]` and `Z<a,b>` | 2 | 0 |
| `linear/one-sided`, `Z<a,b>/(ba+1)` | 1 | 1 |
| `twist/M2^k`, all 2-element bases, `k = 2..8`, all cyclic shift pairs | 3248 | 46 |
| `twist/M3^k`, all 3330 3-element bases up to isomorphism, `k = 2..4` | 96,570 | 3219 |
| **total** | **165,969** | **6005** |

Four instances have infinite carriers. 2024 have carriers past what a
six-variable table sweep can reach, and are decidable only symbolically or
coordinate-wise.

Rows are deduplicated on the signature. That subsumes deduplication up to
isomorphism — isomorphic magmas satisfy the same laws — and is the right
equivalence for coverage, since two magmas with the same signature discharge
exactly the same separations. The collapse is severe and informative: 2028
affine instances over `Z` produce **zero** signatures not already present,
because for coefficients with no algebraic relation the only laws satisfied are
those holding identically, which is the generic model. The same happens to all
625 `Z[t]` instances.

### Redundancy

| | |
|---|---|
| distinct rows in the corpus | 6005 |
| rows discharging at least one hard-core pair | 138 |
| greedy cover of the 416 reached pairs | **13 instances** |

The word "minimal" is not used. Set cover is NP-hard, greedy gives an upper
bound within `ln(n)`, and the LP relaxation that would give a lower bound —
and therefore the gap, which is the interesting quantity — is Phase B. What
can be said now: 13 is an upper bound on the size of a smallest cover of what
this corpus reaches, and 5992 of 6005 rows are redundant for the hard core.

---

## 6. The exhaustive size-4 sweep

Completed. 4,294,967,296 tables, **178,981,952** after isomorphism filtering,
83 minutes wall clock at 856,000 tables/s, 75 MB resident.

```
size <= 2:    12560783 refutations
size <= 3:    13596121 refutations
size <= 4:    13753982 refutations
```

All three match the ETP repo's corrected figures exactly, from an
independently written implementation sharing no code with theirs. Against the
published paper's 13,632,566 — a figure that does not reconcile with its own
stated parts — this is the third independent confirmation that §5.1 is wrong
and the repo is right. The canonical count is also exactly OEIS A001329(4),
which validates the isomorphism filter at the only scale where it matters.

Every refuted pair was checked against the implication graph. No contradictions.

Two notes on reading this.

It contributes **nothing** to the hard-core number by construction: every
counterexample of carrier size at most 4 is precisely what Vampire's finite
model builder already found, which is why set (B) exists in the first place.

The run also reports that 178,985,292 of the swept tables "separate at least
one pair", i.e. all of them. That is not a finding. Every magma of size at
least 2 satisfies E1 and refutes E2, so the predicate is trivially true above
the singleton and the number carries no information.

Worker memory was the one thing that needed care: a per-chunk accumulator is
2.8 MB and there are 262,144 chunks, so collecting them would have wanted
728 GB. The sweep folds one accumulator per worker instead, which is what the
75 MB reflects.

Still outstanding from Phase 0, and now sprint S1: reconciling Janota's "only
310 of the undecided implications require an infinite model" against the
paper's 820 pairs whose truth flips under finiteness, and against the 814 pairs
his saturation runs refuted without producing a witness. That needs the
per-method fields of the 84 MB Vampire dump, which was deleted once
`implications.bits` and `hard_core.txt` were extracted; redoing it means
re-fetching that file.

## 7. Reproducibility

- Toolchain pinned to Rust 1.96.0 in `rust-toolchain.toml`.
- No randomness anywhere. The assignment-order stride is derived from the
  golden ratio and the carrier size, so runs are reproducible without a seed;
  the one test that wants pseudo-random magmas uses a fixed LCG constant.
- Vendored ETP data in `data/etp/` with the upstream commit in
  `PROVENANCE.txt` (`e5a88a1`, Apache-2.0).
- `implications.bits` (2.7 MB) and `hard_core.txt` are derived from the ETP's
  84 MB Vampire dump by the recipe recorded in the Phase 0 report; the derived
  files are committed, the dump is not.
- Raw outputs in `out/`: `grid.txt`, `coverage_hardcore.tsv`,
  `corpus_signatures.bin` (6005 x 587 bytes), `corpus_index.tsv`,
  `summary.tsv`, `bruteforce4.log`.
- 24 tests, `cargo test --release`, about 50 seconds.

## 8. Not done, deliberately

No learning, no reward function, no GPU. No claim of minimality. No Lean
export — when it comes it must avoid `native_decide`, which introduces
`Lean.ofReduceBool`; both Palomar and the SAIR judge allow only `propext`,
`Quot.sound`, and `Classical.choice`. Nothing has been submitted anywhere.
