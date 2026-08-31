# eqprove — proof-carrying completion for one-law magma theories

Standalone, no dependency outside the standard library. This is the readable
copy; the Stage 2 solver inlines the same code with a `pv_` prefix on every
name. See [../../../docs/equational-prover.md](../../../docs/equational-prover.md)
for what it is for and what it measures.

    eqprove.py    terms, matching, positions, one-step rewriting
    complete.py   unification, critical pairs, completion, the two joins
    drive.py      problem parsing, replay verification, Lean `calc` emission

## Use

```python
import json, drive as D

problem = {"id": "p", "eq1_id": 1298, "eq2_id": 2,
           "equation1": "x = y * (((x * z) * x) * w)",
           "equation2": "x = y"}

path, seconds, error = D.run_complete(problem, budget=12.0)
assert error is None                     # None means the path replayed
L, R, GL, GR, hv, gv = D.prep(problem)
print(D.emit(path, GL, GR, hv, gv))
```

which prints a Lean `calc` chain proving the implication. Against the whole
problem set:

```bash
python3 drive.py miss21.json          # search only, with replay verification
python3 judgecheck.py miss21.json     # and elaborate each proof through the judge
```

## The one invariant

Every equation is `(lhs, rhs, steps)`, where `steps` replays `lhs` to `rhs`
using only the hypothesis. Nothing is ever assumed: a derived lemma is a
recorded derivation, and `verify()` replays a path step by step before any Lean
is written. A path that does not replay is discarded rather than emitted, so a
bug in the search shows up as a missing proof and never as a wrong one.
