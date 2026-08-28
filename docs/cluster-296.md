# S2 — what the 296-pair cluster wants

The largest block of uncovered hard-core pairs is eight laws with 37 uncovered
pairs each. They are not in the blueprint's curated hard list, so the method
had to be recovered from the project's issue history.

## The eight are four dual pairs

```
E1133 ↔ E2481      E1167 ↔ E2473
E1659 ↔ E2000      E1661 ↔ E1979
```

So it is four problems seen twice, which is why the uncovered counts are
identical across all eight. That halves the target and explains the
suspiciously regular 37.

## Only one has a method on record

`teorth/equational_theories` issue **#571**, *"MODIFIED_BASE_MODEL: Resolve all
implications arising from 1659"*, points at the Zulip thread *Hard problems and
negative results* and says the "modified translation-invariant magma" approach
should work. It was claimed and closed by PR **#573**, which adds
`equational_theories/InfModel_1659.lean`, 430 lines.

The witness there is **infinite**, on the natural numbers:

```
op(x, y) = if x = 0      then (if y even then 1 else 0)
           if x = n + 1  then (if x ≡ y mod 2 then n + 2 else n)
```

Parity agreement moves you up or down by one, with a special case at zero. It
is not a linear model, not a Cartesian power, and not translation-invariant in
the clean `x + f(y - x)` sense — the zero case breaks the invariance. It is an
ad hoc construction with no finite parameter grid, which is exactly the kind of
thing this corpus cannot reach by sweeping.

The other three dual pairs have no method-naming issue at all. E1133 and E1167
appear only in *finite*-implication issues (#935, #985, about converting
Lean+Duper proofs), which is a different question.

## What that implies

Two readings, and they point different ways.

**Pessimistic.** The cluster was resolved by hand-built infinite models. No
family sweep reaches those, and reproducing them means reimplementing four ad
hoc constructions from their Lean sources. That is transcription, not
discovery, and it would add 296 pairs of coverage while proving nothing new.

**Optimistic, and cheap to test.** The absence of a method trail for three of
the four pairs suggests they fell to automation and left no issue. Combined
with the domain-size cliff — no search-based model finder gets past carrier 10
on these problems — a finite model may well exist at carrier 11 to 32 that
nobody looked for. That is precisely the situation that produced 357 covered
pairs from `Z/13` linear magmas.

The test is bounded: translation-invariant models `x ◇ y = x + f(y - x)` over
`Z/n` (paper §5.3), which is the nearest well-defined family to the
construction #573 used, swept exhaustively over `f` for small `n`.

## Decision

Build translation-invariant models next, ahead of the greedy spike. The grid is
finite and stated, the sweep is cheap because a law of this shape refutes
almost every candidate on the first assignment, and it aims at the carrier range
the cliff says is unsearched. If it comes back empty the cluster is ad hoc and
the honest answer is that this corpus does not reach it.
