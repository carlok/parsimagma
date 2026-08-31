# What a 986-line solver gives up to a mature prover

> Produced by an AI agent (Claude) under human direction, 2026-08-31. Every
> figure is a measurement taken on this machine.

The self-contained Stage 2 solver settles 196 of 200 problems with no borrowed
code, no lookup tables and no LLM call. Four resist. This is what happened when
those four were handed to the tools that exist for exactly this.

## Vampire settles all four, in milliseconds

    7422  -> 18784   SZS status CounterSatisfiable
    19040 -> 12906   SZS status Theorem    0.024 s   46 proof lines
    4777  -> 33810   SZS status Theorem    0.001 s   15 proof lines
    6543  -> 29450   SZS status Theorem    0.002 s   25 proof lines

Three theorems and one refutation. Mace4 produced an explicit six-element
countermodel for the fourth in 1.4 seconds:

    [[1,2,0,0,0,0],[1,3,1,2,1,1],[1,3,2,2,0,2],
     [1,3,3,4,5,3],[2,4,4,4,0,4],[1,5,5,4,2,5]]

Checked independently here: E7422 holds on all 216 assignments and E18784 fails
on 161 of them. The judge accepts the certificate.

So the four are not hard. They are hard **for this solver**, which is a
different and more useful thing to know.

## Where the gap actually is

Vampire's shortest proof, for `4777 -> 33810`, is worth reading in full because
it says precisely what is missing. Stripped to its equations:

    f6   x = x ◇ (y ◇ (x ◇ (z ◇ (z ◇ z))))          the hypothesis
    f8   superposition of f6 into f6
    f12  superposition of f6 into f8                 y ◇ (x ◇ (x ◇ x)) = y
    f16  superposition of f6 into f12                x ◇ (y ◇ x) = x
    f15  superposition of f8 into f12
    f21  forward demodulation of f15 by f16          x ◇ y = x
    then three steps to close the goal

The whole proof turns on `f21`: **the law forces left projection.** After that
the goal is three rewrites.

The local prover derives 3,000 consequences of this law and **never reaches
`x ◇ y = x`** — verified by an exact structural test, not a loose match. Its
critical pairs plus matching are strictly weaker than superposition with
demodulation, and the missing step is the demodulation at `f21`.

That is a sharper statement than "needs a better prover". It is not term size:
the target lemma has three nodes. It is not budget: 6,000 equations and
ceilings from 13 to 21 change nothing. It is not simplification, which is
implemented and does not help. It is the inference rule.

## One thing that was worth fixing

`decideFin!` overruns Lean's default recursion limit past carrier 5, so the
judge answers "maximum recursion depth has been reached" and a perfectly good
counterexample is thrown away rather than rejected. `set_option maxRecDepth`
fixes it; it is not a banned token and adds no axiom, and the kernel checks the
same term. Certificates now carry it from carrier 6 up.

Finding the six-element model was out of reach too, until the search stopped
enumerating relabelings. The elements of a carrier are interchangeable, so most
of the n^(n*n) tables are the same magma written differently. The least-number
heuristic removes them: filling cells in order, a cell may introduce element
`k+1` only once `k` has appeared. Every isomorphism class keeps a
representative, so no model is lost.

Carrier 6 went from **not found in 280 seconds** to **found in 91**, and the
model the local search returns is not Mace4's — a different six-element magma,
verified the same way. The solver now settles this problem on its own, in 95
seconds, with no external tool.

## Honest summary

Vampire was worth an hour as an instrument, not as a dependency. It cannot be
part of the submission — the solver runs as a subprocess on the organizer's
machine with the Python standard library and nothing else — but it corrected a
wrong assumption (one of the four was never a theorem) and its proof named the
exact inference the local prover lacks.

Of the four, one is now settled locally. The three that remain are the ones
where a single non-obvious lemma unlocks everything, and the reason they hold
out is one missing inference rule rather than any amount of tuning.
