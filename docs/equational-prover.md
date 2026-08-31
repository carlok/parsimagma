# A prover for the laws that survive model search

> Produced by an AI agent (Claude) under human direction, 2026-08-31. Every
> figure below is a measurement, and every proof it describes was elaborated by
> Lean and checked by the kernel.

## The problem

On the SAIR Stage 2 sample set, a solver built from exhaustive Cayley-table
search plus this repository's structured-magma corpus settles 172 of 200
problems. The 28 it misses are not a random remainder. Every one of them is a
**true** implication: no counterexample exists, so no amount of model search
helps, and the solver has to produce an equational proof instead.

Those 28 have a shape in common. Their hypotheses read

    x = C[x, y, z, ...]

with the right side strictly bigger and carrying variables the left side does
not. Rewriting the goal directly does badly on such a law:

- **shrinking** (`C[x,..] -> x`) needs the whole of `C` to match, which almost
  never happens on a term the goal actually contains;
- **growing** (`x -> C[x,..]`) matches *everywhere*, because the left side is a
  bare variable, and each match branches once per unbound variable of `C`.

Measured: on `E858`, the goal's left side has 24 one-step successors and its
right side 216, at a size cap of 17. Three plies of that is millions of terms
and none of them is the goal. Raising the depth, the time limit, the term-size
cap, and the instantiation pool — all four, together, by an order of magnitude
— moved the count from 2 solved to 2 solved.

## What works instead

Derive consequences of the law first, then ask whether any of them *is* the
goal.

    complete    overlap the law with itself and with what has been derived,
                collecting critical pairs
    join        for each derived equation, one match against the goal, with
                the goal's variables treated as constants

The second step does no search at all. It works because a consequence of
`x = C[x,..]` also has a variable on one side, so a goal `x = <compound>` is
discharged the moment some derived right-hand side matches the compound, with
the variable landing on the goal's constant.

**Every derived equation carries its own proof.** An equation is a triple
`(lhs, rhs, steps)` where `steps` is a sequence of primitive rewrites by the
hypothesis that takes `lhs` to `rhs`. Critical pairs splice their parents'
step lists; instantiating an equation instantiates its steps. So a derived
lemma is never an assumption, and when one finally matches the goal, what comes
out is a flat chain of hypothesis applications with no intermediate `have`.

Each step becomes one `calc` line, `congrArg (fun t => C[t]) (h a b c)`, with
`.symm` for the reverse direction. Every path is replayed internally before a
line of Lean is written; a path that does not replay is discarded rather than
emitted.

## Results

Of the 28 misses, the prover closes 15. The judge accepts each on the first
attempt, in roughly two seconds.

| | order4_normal | order4_hard | order4_extra_hard | order5_normal | total |
|---|---|---|---|---|---|
| table search + corpus | 40/50 | 43/50 | 50/50 | 39/50 | **172/200** |
| with the prover | 47/50 | 45/50 | 50/50 | 45/50 | **187/200** |

No LLM call was made on any of the 200 problems, in either row.

The order-5 column matters most. Those laws lie outside the ETP's 4694, so
there is no oracle to consult and no published status to look up: the prover is
deriving proofs for implications this repository has no table for.

## Axioms

The generated proofs depend on **no axioms at all** — `#print axioms` returns
the empty list, not `[propext, Classical.choice, Quot.sound]`. They are chains
of `congrArg` and `Eq.symm` over a hypothesis, so nothing classical is
reachable, let alone used. Certificates produced by table search are not like
this: `decide` on a finite carrier brings in the usual three.

## What it does not do

Thirteen of the 200 remain unsolved. Eight of those need a genuine multi-step
join between derived equations rather than a single instance, and the bounded
version of that recovers one. The other five are the pairs a previous build
covered with hardcoded per-problem tactics; those were removed, because the
graded set reuses no publicly available problem and they could never fire
there. Completion is also run to a fixed budget and is not
saturating: the equation set is a few hundred consequences deep, not a
confluent system, so a negative answer means "not found within the budget" and
never "no proof exists".

The implementation is `try_completion_proof` in the Stage 2 solver, about 420
lines with no dependency outside the standard library.
