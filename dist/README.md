# refute.py — a dependency-free disproof generator for ETP implications

Published as `parsimagma-greedy-cover` on the SAIR Contributor Network,
2026-08-29: <https://competition.sair.foundation/contributor-network/mathematics-distillation-challenge-equational-theories-stage2/EQT02-S00025>
(public ID `EQT02-S00025`). `dist/solo/solver.py` is the file published there;
`refute.py` below is the same construction corpus as a plain library.

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

## Stage 2 certificate format

`to_lean` emits a term of the judge's `Goal`, which is
`∃ (G : Type) (_ : Magma G), EquationLHS G ∧ ¬ EquationRHS G`:

```lean
import JudgeProblem
import JudgeDecide.DecideBang

-- x*y = 7x+7y over Z/13
def submission : Goal := by
  let m : Magma (Fin 13) := { op := fun x y => 7 * x + 7 * y }
  refine ⟨Fin 13, m, ?_⟩
  decideFin!
```

**205 bytes**, against a 20,000-byte cap on false certificates.

The operation is a function, not a `finOpTable` string. That helper parses its
argument one character at a time —

```lean
private def extractDigits (s : String) : List Nat :=
  s.toList.filterMap fun c => if c.isDigit then some (c.toNat - '0'.toNat) else none
```

— so any table entry of 10 or more is silently read as two entries, which
quietly breaks every carrier above 9. `to_lean_table` is the fallback for
witnesses with no closed-form rule and uses the judge's `magmaFin`, which takes
a `List Nat` and has no such bug.

Verified against a local replica of `JudgeProblem`: the certificates compile,
and `#print axioms submission` reports `[propext]` alone — inside the allowed
set of `propext`, `Quot.sound`, `Classical.choice`, and with no
`Lean.ofReduceBool`, which is what makes `native_decide` inadmissible.

## Order 5

The refuter is law-text driven and never sees an equation id, so the order-5
category needs no extra work. On 300 uniformly random order-5 pairs it refutes
**56.7%**; since that sample includes true implications, which cannot be
refuted at all, the effective rate on false ones is close to the 96.2% measured
on order 4. 147 of the 170 successes are two-element magmas.

## Checked

- The parser round-trips all 4,694 order-≤4 laws and all 62,576 order-≤5 laws.
- Emitted Lean compiles against Mathlib v4.32.0 with plain `decide`, no
  `native_decide`.
- Spot checks agree with the parent corpus: `E2700 ⊭ E2709` on `Z/9` with
  `2x + 8y`, `E1076 ⊭ E546` on `Z/13` with `7x + 7y`.
