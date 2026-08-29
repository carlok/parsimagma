# S8b — what the 39 actually want

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

[infinite-only-exact.md](infinite-only-exact.md) cut the hard core's residual
from 646 pairs to 39: the ones the ETP proves have a finite counterexample and
that no construction here reaches. This is what they need.

The short version: **not greedy constructions.** `phase-a-report.md` attributes
the uncovered remainder to greedy, and for the 646 in aggregate that was right,
since 605 of them admit no finite model at all and greedy is what the ETP used
on those. For the 39 it is wrong. They want §5.6 — magma extensions.

## The ETP has published witnesses for 30 of them

The Equation Explorer's `finite_graph.json` carries a live `full_entries` map,
1171 files and 11,897 entries, in the compact form
`f|line|name|[satisfied]|[refuted]` (schema in
`home_page/implications/show_proof.html`). Duality is applied automatically, so
a witness may be recorded against the dual pair.

Matching the 39 against its 1119 finite `facts` entries finds witnesses for
**30**, and every one of them comes from a single contiguous block:

| file | carrier | facts |
|---|---|---|
| `Generated/All4x4Tables/Refutation930` | `Fin 12` | E879 ⊭ E4065 |
| `…/Refutation932` | `Fin 21` | E854 ⊭ E3316, E3925 |
| `…/Refutation933` | `Fin 19` | E854 ⊭ E413 |
| `…/Refutation934` | `Fin 24` | E854 ⊭ E1045 |
| `…/Refutation935` | `Fin 36` | E503 ⊭ E3862, E4065 |
| `…/Refutation936` | `Fin 36` | E476 ⊭ E4065 |
| `…/Refutation937` | `Fin 35` | E1516 ⊭ E1489 |
| `…/Refutation938` | `Fin 65` | E1076 ⊭ E2294, E4435 |
| `…/Refutation939` | `Fin 15` | E1518 ⊭ E47, E614, E817, E3862 |

The directory name is a generated-file convention, not a size claim — these are
explicit tables at carriers 12 to 65, discharged by `decideFin!`.

The remaining 9 have no direct `facts` witness and are refuted by composition:
`(476,359)`, `(503,359)`, `(854,1055)`, `(854,1067)`, `(2712,2285)`,
`(2712,2452)`, `(2712,2488)`, `(3069,307)`, `(3076,307)`.

## Two of them are transparent, and they are the ones that matter

`Refutation938`, the `Fin 65` witness for the last two E1076 pairs, is exactly a
paper-§5.6 extension:

- **base**: the linear magma `x ◇ y = 4x + 2y` on `Z/5`, well-defined on all
  65 x 65 entries
- **fibre**: `Z/13`, and for all 25 base pairs the fibre operation is affine with
  **the same** coefficients, `α = 5, β = 9`
- **twist**: only the constant varies — a 5 x 5 matrix of elements of `Z/13`

So it is `x ◇ y = 5x + 9y` over `Z/13`, an instance this corpus already has,
twisted by a 2-cocycle over a 5-element base this corpus also already has. What
is missing is not either ingredient. It is the extension itself.

`Refutation937` (`Fin 35`) has the same shape: base of order 5, fibre `Z/7`,
fibres affine. Both are quasigroups, rows and columns permutations.

This is the family listed as §5.6 "submagma / projection / magma cohomology
extensions" in the Phase 0 construction table and never implemented here.

**None of the theory below is new.** `blueprint/src/chapter/cohomology.tex` is a
whole chapter on it: extensions with carrier `G x M`, the observation that an
extension satisfies a law exactly when a condition on the cocycle holds, and the
conjugation `(x,s) ↦ (x, s + g(x))` that quotients the search by coboundaries.
`blueprint/src/chapter/677.tex` uses the same shape in its Lemma "No
counterexamples via linear extension". What follows is a re-derivation of
documented theory, done to check the family is worth building here. It is new to
this corpus and to nothing else.

## The other eight are not transparent, and the test is weak

| file | carrier | rows perm | cols perm | block congruence |
|---|---|---|---|---|
| 930 | 12 | yes | no | none found |
| 931 | 25 | yes | yes | none found |
| 932 | 21 | no | no | none found |
| 933 | 19 | no | no | none found (19 is prime) |
| 934 | 24 | no | no | none found |
| 935 | 36 | yes | no | base 4 x fibre 9, fibres **not** affine |
| 936 | 36 | yes | no | base 4 x fibre 9, fibres **not** affine |
| 939 | 15 | yes | no | none found |

Caveat that matters: the congruence test only looks for **contiguous blocks in
the given labelling**. A congruence visible only after relabelling is invisible
to it, so "none found" is a limit of the test, not a proof of no structure.
Treat this table as a lead list, not a classification.

## The rediscovery test passes, and the search space is tiny

Before writing any Rust, the theory was checked against `Refutation938` directly.

**Both ingredients satisfy all three laws.** The base `4x + 2y` on `Z/5` satisfies
E1076, E2294 and E4435. The fibre `5x + 9y` on `Z/13` satisfies all three too.
The extension satisfies E1076 and violates E2294 and E4435. So the separation
lives entirely in the cocycle — which is exactly why no product, power or direct
family in this corpus can reach it, and why widening the linear grid never would
have.

**The law condition is linear in the cocycle.** For each of the three laws the
fibre residual turns out to be independent of the fibre coordinates `(s, t)`, so
it is a function of the base pair alone: 25 values. And it is linear in the 25
cocycle variables, verified by building the matrix from unit bumps and predicting
the residual at ETP's own cocycle.

That collapses the search. Over `F_13`:

```
E1076 cocycle system: rank 20  ->  solution space dimension 5
                                   13^5 = 371,293 cocycles, all satisfying E1076
ETP's cocycle lies in that space:  yes
of 4000 sampled from it, 3684 refute E2294 or E4435:  92%
```

**13^25 was never the search space. It is 13^5, and 92% of it works.** The
witness is not rare, it is generic once you are in the right family. This corpus
missed it for one reason: §5.6 is not implemented.

## What to build

The narrow version, which is cheap and has a known target: implement §5.6 for
the case both ingredients are already in the corpus — base magma `B` of small
order, fibre `Z/m` with coefficients `(α, β)` constant across base pairs, twist a
cocycle `c: B x B → Z/m`. `Refutation938` and `Refutation937` are then
reproducible checks: if the family is right, the engine rediscovers them, and if
it does not, the family is wrong and that is worth knowing before scaling it.

Two things to note before costing it. The naive cocycle space is `m^(|B|^2)` —
13^25 for the E1076 case — so this needs the cocycle condition, not enumeration.
And `atp-control.md` already showed generic model search dies past carrier 9,
so brute force over `Fin 65` tables is not an alternative route.

The 39 are worth what they are worth: closing them takes the corpus from 411 of
450 to 450 of 450 against the finitely refutable hard core. That is a complete
statement rather than a large number.

## Reproducing

```
curl -sLO https://teorth.github.io/equational_theories/implications/finite_graph.json
# the live witness index is the "full_entries" key, not data/full_entries.json,
# which is a stale snapshot carrying only 5 finite implications
```

then match `data/etp/finite_uncovered.txt` against its finite `f|…` entries,
under duality as well as directly.
