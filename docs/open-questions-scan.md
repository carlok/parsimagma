# S6 — 143 open ETP questions, scanned

A complete negative, from a validated engine. Recording it because the shape
of the negative says more than the count.

## What was tested

| set | count | the open question | source |
|---|---|---|---|
| blueprint table 20.2 | 96 | no nontrivial finite model exists; **does an infinite one?** | ch. 20 |
| blueprint table 20.3 | 24 | **does a nontrivial finite model exist?** | ch. 20 |
| Higman-Neumann | 13 | **does a non-group model exist**, finite or infinite? | `data/Higman-Neumann.json` |
| blueprint table 20.1 | 10 | proved Austin — used as a control | ch. 20 |

The 96 are the interesting ones. The blueprint says plainly: *"Of the 96
remaining equations, Vampire did not establish any implications between
equations in this set. No effort was made to build infinite models for these
equations."* Building infinite models symbolically is what Tier S does, so this
was the obvious place to point it.

## What was thrown at them

- ~90,000 linear and affine instances: `Z/m` for `m = 2..96` (all coefficients),
  affine `Z/m` for `m = 2..32`, `Z`, `Z[t]`, `M_2(F_2)`, `M_2(F_3)`
- 46,538 twisted Cartesian powers over every 2-element base (`k = 2..8`) and
  every 3-element base up to isomorphism (`k = 2..3`), carriers 4 to 256
- the generic rings `Z[a,b]` and `Z<a,b>`
- `Z<a,b>/(ba+1)`, where `b` is a one-sided inverse of `a`
- the Weyl algebra `Z<a,b>/(ba - ab - 1)`, with generator and small
  integer-combination coefficients

## Result

**Nothing. All 143 stay open.**

The only hits anywhere were on the Higman-Neumann laws, and all 611 of them are
abelian group division: `x - y` over `Z/m`, `Z`, `Z[t]`, or `M_2(F_p)` with
`b = -a`, plus `x - y + c` affine variants and twisted powers of the same. Not
one non-group model.

## Why the negative is structured rather than blind

For table 20.2 the search space is far smaller than it looks. A law holds in a
linear magma exactly when its difference polynomials vanish in the coefficient
ring. If it vanishes over a ring `R`, it vanishes over every quotient of `R` —
so a linear magma over any ring with a nontrivial finite quotient hands you a
nontrivial **finite** model, which those 96 laws provably do not have.

Every ordinary ring is therefore excluded a priori. `Z`, `Z[t]`, `Z/m`,
`M_k(F_p)` cannot witness anything in table 20.2 no matter how wide the grid.
Only rings with no suitable finite quotient can, and that is a small class.
Two of its members are now implemented and both miss:

- `Z<a,b>/(ba+1)`: a one-sided inverse cannot survive into finite dimension,
  where one-sided inverses are two-sided.
- `Z<a,b>/(ba - ab - 1)`: in characteristic 0 the Weyl algebra has **no**
  finite-dimensional representation, since `tr(ab - ba) = 0` while `tr(1) = n`.

So the result is not "we searched and found nothing". It is "the only families
that could possibly work are exceptional rings, and the two natural ones do
not". The next members of that class — group rings of groups with no finite
quotients, such as Higman's group — are days of work each for a small chance,
which is why the queue stops here rather than continuing.

## Validation, because a negative from untested code is worthless

| check | result |
|---|---|
| `eq_size5.txt` extends `equations.txt` law for law over the first 4694 | exact |
| order-5 laws: max 5 operations, max 7 distinct variables | as expected |
| **positive control** — Z/m models found for >200 of 4000 order-5 laws | pass |
| **negative control** — the 10 proved Austin laws admit no `Z/m` model, `m ≤ 48` | pass |
| symbolic vs table route on order-5 laws, 29 instances | bit-identical |
| Weyl: `ba - ab = 1`, `ab ≠ 1`, associativity, `b a² = a² b + 2a` | pass |
| Weyl linear magma contains the generic `Z<a,b>` signature | pass |

Engine changes: `MAX_DEG` 4 → 6 and `MAX_VARS` 6 → 7, which cover order-5 laws
(depth 5, 7 variables) and the order-8 Higman-Neumann candidates (depth 6). All
24 order-4 tests still pass at the wider bounds, which is the regression that
matters.

## Artifacts

`data/etp/order5_open.txt`, `data/etp/hn_open.txt`, `data/etp/eq_size5.txt`.
Rerun with `pm openq`; `PM_SHOW=1000` prints every witnessing instance.
