# S3 — translation-invariant models add constructions but no coverage

`x ◇ y = x + f(y - x)` over `Z/n`, ETP paper section 5.3. Built because S2
identified it as the nearest well-defined family to the construction the ETP
used for the 296-pair cluster, and because the domain-size cliff says carrier
11 and above is unsearched territory.

## Result

**Zero new hard-core pairs, at any carrier size reachable.**

| grid | candidates | reach something | distinct pairs | new |
|---|---|---|---|---|
| all `f`, `n = 2..7` | 873,611 | 0 | 0 | 0 |
| permutations `f`, `n = 8..11` | 43,948,800 | 19 | 16 | **0** |
| permutations `f`, `n = 12` | 479,001,600 | 0 | 0 | **0** |

523 million candidates. All 16 pairs reached were already covered by the linear
family, and `n = 12` reached nothing at all — 35 minutes for an empty result,
which is itself informative: the family thins out rather than improving as the
carrier grows into the range the cliff leaves unsearched.

## Why permutations, and why that was not enough

Sweeping every `f` is `n^n`, which is 16.7 million at `n = 8` and 285 *billion*
at `n = 11`. The family is exhaustively searchable only *below* the cliff —
exactly where finite model finders already succeed, so exactly where there is
nothing to find. That is the structural problem with this family: its grid
grows far faster than the linear family's `n^2`.

Restricting `f` to permutations brings `n = 11` down to `11! = 39,916,800`,
three minutes. The restriction is principled rather than convenient: a law
`x = w(x, y, ...)` with `x` occurring once on the right forces the relevant
translations to be surjective, and the left translation `y ↦ x + f(y - x)` of a
translation-invariant magma is bijective exactly when `f` is.

It reached the range and still found nothing new.

## The family is not redundant, its coverage is

Worth separating. At `n = 9`, six permutations discharge hard-core pairs and
**four of them are genuinely nonlinear** — `f = [0,2,1,6,8,7,3,5,4]` is not
`d ↦ bd` for any `b`. So the family does produce magmas the linear sweep cannot.
They simply land on pairs the linear sweep already covers.

That is a sharper negative than "the family is a subset of linear". It is a
distinct family whose *coverage* happens to coincide.

## What it says about the cluster

S2 left two readings of the 296-pair cluster. This closes one of them. The
ETP's witness for `E1659` is an ad hoc infinite model on the naturals built
from parity and successor; the nearest systematic family does not reproduce
even its finite shadow. On the evidence, the cluster is ad hoc, and reaching it
means transcribing four hand-built constructions rather than sweeping a grid —
which would add coverage while proving nothing.

## Reproducing

```bash
cargo run --release --bin pm -- transinv 7
```

```bash
cargo run --release --bin pm -- transinv 12 8 perm
```

Arguments are `max_n`, then optional `min_n`, then optional `perm`.
