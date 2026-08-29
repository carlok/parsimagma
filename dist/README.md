# refute.py — a dependency-free disproof generator for ETP implications

Given two equational laws as text, find a finite magma satisfying the first and
violating the second. That magma disproves `E => E'`, and Lean checks it with
plain `decide`.

```python
from refute import refute, to_lean
r = refute("x = ((y ◇ x) ◇ (x ◇ y)) ◇ y", "x = ((y ◇ x) ◇ (y ◇ y)) ◇ x")
print(r["rule"], r["carrier"])          # x*y = 2x+8y over Z/9  9
print(to_lean(r))                        # a compiling Lean 4 file
```

Standard library only. No data files: the construction corpus is a generator,
ordered by a greedy set cover over the ETP's 13,855,357 false implications.

## Measured

On 400 uniformly sampled false implications from the ETP order-≤4 graph:

| | |
|---|---|
| refuted | **385 / 400 = 96.2%** |
| mean time at a 0.25s/problem budget | **11 ms** |
| carrier of the witness | 2 in 342 cases, 3 in 21, 4 in 8, 5 in 7, 7 in 5, 11 in 1, 13 in 1 |

The distribution is the whole story: 89% of the successes come from a
two-element magma found in the first four tries. Four instances of `Z/2` cover
85.7% of the ETP's false implications and twenty-five cover 97.2%, so the head
is tiny and the tail is generated rather than stored.

A miss costs the full sweep, about 26 seconds, so `budget=` matters under a
per-problem limit. A timeout means "no answer", not "no witness".

`refute` only ever disproves. It says nothing about true implications, and
`None` is not evidence that an implication holds.

## Checked

- The parser round-trips all 4,694 order-≤4 laws and all 62,576 order-≤5 laws.
- Emitted Lean compiles against Mathlib v4.32.0 with plain `decide`, no
  `native_decide`.
- Spot checks agree with the parent corpus: `E2700 ⊭ E2709` on `Z/9` with
  `2x + 8y`, `E1076 ⊭ E546` on `Z/13` with `7x + 7y`.
