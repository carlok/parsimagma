# Certificates

Every certificate the solver produced, with the problem it answers, so each can
be put back in front of the compiler without rerunning the solver.

    sprint12-certificates.json    all 200, with the Lean source of each
    order5-certificates.json      the 50 order-5 problems, plus a judge status
                                  and an axiom report per certificate

## Toolchain

Produced and re-elaborated against the judge's pinned toolchain: **Lean 4.33.1**
with the matching Mathlib release, commit
`0df444a360eaa60ab8c11dca51a86af692955474`. The organizer moved evaluation to
this toolchain on 2026-08-26, from Lean 4.32.2. Nothing here was produced
against the earlier one.

## The order-5 fifty

These lie outside the Equational Theories Project's 4694 laws of order at most
four, so no published status applies to them. Every one was re-elaborated
through the judge after the run, in a separate process, from the stored source:

    50 accepted, 0 rejected
    25 verdict true   — axioms []
    25 verdict false  — axioms [propext, Classical.choice, Quot.sound]

The split in the axiom column is a property of the emitter, not of the
mathematics. True implications are emitted as chains of `congrArg` and
`Eq.symm` over the hypothesis, which cannot reach anything classical.
Counterexamples are emitted as an explicit table decided by `decide`, and the
three axioms enter through the automation `decide` invokes. A finite
countermodel can in principle be certified without them; this solver does not
do so.

## Checking one

Each row carries `equation1`, `equation2`, `verdict` and `code`. The `code`
field is the complete Lean file as submitted. Against the judge:

```python
from judge.verify import verify_answer
from pipeline.proxy import DEFAULT_PROOF_POLICY
import json
row = json.load(open("order5-certificates.json"))[0]
spec = {k: row[k] for k in ("id","eq1_id","eq2_id","equation1","equation2")}
spec["proof_policy"] = DEFAULT_PROOF_POLICY
print(verify_answer(spec, json.dumps({"verdict": row["verdict"], "code": row["code"]})))
```
