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

## Built, and it closes sixteen of them

`src/ext.rs` implements the family — the blueprint's, from
`chapter/cohomology.tex`, not one invented here — and `pm ext` runs it against
the 39. The fibre is `(Z/p)^k` with `α, β` as `k x k` matrices over `F_p`, which
is what the chapter-677 lemma allows — "M is an abelian group and α, β
endomorphisms". Scalars on `Z/p` are the `k = 1` case.

Grid: 3,533 bases (linear over `Z/nb` for `nb = 2..8` with every `(a,b)`, plus
every canonical 3-element magma) and 661,142 fibre candidates — 1,556 scalar for
`p` up to 23, the rest matrix endomorphisms on `(Z/p)^k` for `(p,k)` = (2,2),
(3,2), (2,3), (5,2). Carrier capped at 130. Candidates are filtered on whether
the fibre alone satisfies any still-open hypothesis law, which leaves 2,147; a
fibre failing every hypothesis can never be half of a separating extension. Each
surviving pair is decided rather than sampled. 8.5 minutes.

**16 of the 39 close.**

A note on how to quote the negative: 661,142 is the grid *before* filtering, and
2,147 fibres survive it. The 23 that stay open are blocked across the settings
actually decided — 55,516 of them — not across 661,142. The larger number sizes
the search; the smaller one is what the negative rests on.

```
E476  -> E359    base Z/5 0x+2y, fibre (Z/3)^2 a=[1,1,1,0] b=[0,2,2,1], carrier 45
E476  -> E4065   base Z/5 0x+2y, fibre (Z/3)^2 a=[1,1,1,0] b=[0,2,2,1], carrier 45
E503  -> E359    base Z/5 0x+2y, fibre (Z/3)^2 a=[0,2,1,0] b=[1,1,2,1], carrier 45
E503  -> E3862   base Z/5 0x+2y, fibre (Z/3)^2 a=[0,2,1,0] b=[1,1,2,1], carrier 45
E503  -> E4065   base Z/5 0x+2y, fibre (Z/3)^2 a=[0,2,1,0] b=[1,1,2,1], carrier 45
E1076 -> E2294   base Z/5 4x+2y, fibre Z/13 a=5 b=9,                    carrier 65
E1076 -> E4435   base Z/5 4x+2y, fibre Z/13 a=5 b=9,                    carrier 65
E1516 -> E1489   base Z/5 3x+3y, fibre Z/7  a=4 b=1,                    carrier 35
E2091 -> E2098   base Z/5 3x+3y, fibre Z/7  a=1 b=4,                    carrier 35
E2531 -> E1313   base Z/5 2x+4y, fibre Z/13 a=7 b=7,                    carrier 65
E2531 -> E4435   base Z/5 2x+4y, fibre Z/13 a=7 b=7,                    carrier 65
E3069 -> E307    base Z/5 2x+0y, fibre (Z/3)^2 a=[2,1,1,0] b=[2,2,2,1], carrier 45
E3069 -> E3253   base Z/5 2x+0y, fibre (Z/3)^2 a=[2,1,1,0] b=[2,2,2,1], carrier 45
E3069 -> E3456   base Z/5 2x+0y, fibre (Z/3)^2 a=[2,1,1,0] b=[2,2,2,1], carrier 45
E3076 -> E307    base Z/5 2x+0y, fibre (Z/3)^2 a=[1,1,1,0] b=[0,2,2,1], carrier 45
E3076 -> E3253   base Z/5 2x+0y, fibre (Z/3)^2 a=[1,1,1,0] b=[0,2,2,1], carrier 45
```

Six of those came from scalar fibres. **The other ten needed matrix
endomorphisms and nothing else** — every one is `(Z/3)^2` at carrier 45, and no
scalar fibre at any modulus up to 23 reaches them. Restricting `α, β` to scalars
was the binding constraint, not the grid width.

And the axis is now exhausted at this rank. Going from 8,373 fibres to 661,142 —
adding rank 3 over `F_2` and rank 2 over `F_5` — took the sweep from 25 seconds
to 8.5 minutes and closed **nothing further**. All sixteen are still `(Z/3)^2`,
`Z/7` or `Z/13`. What that buys is a much heavier negative rather than a
sixteenth-plus pair: E879 and E2650 are now blocked across 11,938 viable
settings each and E1518 and E2054 across 3,695, against 1,184 and 645 before.

The E1076 pair lands on **exactly** ETP's parameters for `Refutation938` — base
`Z/5` with `4x + 2y`, fibre `Z/13` at `α = 5, β = 9`, carrier 65 — found from the
ingredients rather than copied from the table. `E1516 -> E1489` at carrier 35
matches `Refutation937`'s shape.

So the hard-core figure moves from 411 to **427 of 450**, by construction:
nothing here reads ETP's tables.

Every closure is checked twice. The residual test is the decision procedure, and
wherever `carrier^arity` is affordable — which covers all 16, at carriers 45 and
65 — a full engine sweep is run with an assertion that fires on disagreement.
Independently, ETP's finite implication graph records all 16 as finite-false, as
it must.

`tests/ext.rs` pins the reproduction, including the claim the family rests on —
base and fibre each satisfy E1076, E2294 *and* E4435, so the separation cannot
come from either.

### The other 23 are a proof, not a search failure

Widening the grid alone did nothing: at 17x the bases and the same scalar
fibres, the count stayed at six. What moved it was changing the *kind* of fibre,
not the size of the grid. What is left after that is a proof, not a search that
came up short.

Sampling was the wrong tool, and replacing it with a decision procedure says so:

> The target's residual is affine in the cocycle and the hypothesis's solution
> set is an affine subspace, so the restriction is affine and vanishes
> identically exactly when it vanishes at the particular solution and at each
> `particular + basis_i`. That is `dim + 1` probes, and it *decides* the
> question, where sampling can only fail to find something.

**None of that is a finding here.** `blueprint/src/chapter/cohomology.tex` states
it as the method: the `E`-cocycles form a group `Z²_E(G,M)`, coboundaries
`B²(G,M)` sit inside it, and

> "to refute an implication `E ⟹ E'`, it suffices to locate a magma `G` and a
> linear magma `M` satisfying both `E` and `E'` such that
> `H²_E(G,M) ⊄ H²_{E'}(G,M)`. This leads to a computational approach to
> refutations, as these groups can be computed by linear algebra."

with a worked example — E1110 against E1629 on `F_5` with `x ◇ y = 3x - y`,
coboundaries four-dimensional, cocycles six, giving a 25-element counterexample.
What is here is an implementation of that method and a systematic run of it, not
the method.

One thing the chapter does that this implementation does **not**: quotient by
coboundaries. Working in `Z²` rather than `H² = Z²/B²` leaves
coboundary-equivalent cocycles in the search, which is redundancy, though it does
not affect the containment test — `B²` sits inside both spaces, so
`Z²_E ⊆ Z²_{E'}` and `H²_E ⊆ H²_{E'}` say the same thing. Quotienting would make
the sweep smaller, not the conclusions different.

Run that way the sweep decides every pair, and each of the 23 remaining comes
back the same way:

```
E854  -> E413    provably unreachable in all    160 viable settings
E879  -> E4065   provably unreachable in all 11,938 viable settings
E1518 -> E817    provably unreachable in all  3,695 viable settings
E2054 -> E255    provably unreachable in all  3,695 viable settings
E2650 -> E3253   provably unreachable in all 11,938 viable settings
E2712 -> E4128   provably unreachable in all    160 viable settings
...
```

So for every base and fibre in this grid whose ingredients satisfy the
hypothesis, *every* cocycle that keeps the hypothesis also keeps the target. The
family cannot separate those pairs with `α, β` held constant. **Nothing is left
untested**: the three laws using more than three variables were excluded at first
for a cost that turns out not to apply — a residual is `nb^arity` over base
tuples, not `carrier^arity`, because the fibre coordinate is pinned — and once
included they land the same way, unreachable across all 111 viable settings.

Two soundness points, since the conclusion is a negative and negatives are easy
to get wrong. The probes read the residual at fibre coordinate zero, which is
faithful only when the target's residual is flat in that coordinate; `separates`
checks that and returns `Undecided` rather than `Blocked` when it fails. Across
the whole sweep it never fails, but the check is there and the count is reported.
And where the carrier makes a full `carrier^arity` sweep affordable, every
claimed separation is cross-checked against the engine with an assertion that
fires on disagreement.

The dimension statistics show the same thing from another angle, and rule out the
obvious guess. The pairs that close have small cocycle spaces (max dimension 5
for E1076, 16 for E1516) and *no* dimension-zero settings. The pairs that never
close have spaces up to dimension **56** — far larger — across hundreds of solved
systems. More room in the cocycle does not help when the target's condition is
implied by the hypothesis's.

### What the remaining 33 need

The grid is deliberately small: `nb <= 5`, `m <= 13`, linear bases only, twelve
cocycles per space. Widening any axis is cheap and untried. Three laws (E1067,
E2285, E2488) use more than three variables and are skipped, since a sweep at
carrier 65 costs `65^4` and up.

Non-linear bases are now in the grid (every canonical 3-element magma) and change
nothing. What is left is the restriction that actually binds: **`α` and `β` held
constant**. The blueprint writes them as `α_{x,y}, β_{x,y}`, varying with the
base pair, and that is where the family still has room. It is a harder search —
the law condition stays linear in `c` only once `α, β` are fixed, so varying them
turns one linear solve into a search over `m^{2|B|²}` settings of them — but the
proof above says nothing about it, and the two ETP witnesses that motivated all
this happen to have constant coefficients. Iterated extensions are the other
untried direction.

## What to build

(Written before the above. Left as the record of what the plan was.)

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
