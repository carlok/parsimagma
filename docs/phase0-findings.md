# Phase 0 — ground truth, prior art, and exact counts

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

Date of survey: 2026-08-27. Sources fetched and archived under the session
scratchpad; every number below is traced to a named source.

---

## 1. Verdict on duplication

**A coverage matrix over infinite construction families does not exist. Stop is
not warranted.** But the finite tier is more crowded than the brief assumes,
and one live project changes the strategic picture.

Three findings, in order of how much they should change the plan.

### 1.1 The finite tier is already partly measured — and the published numbers disagree with each other

The ETP repo carries `equational_theories/Generated/All4x4Tables/`, which is a
brute-force corpus of every magma of size 2, 3, 4 that refutes something, stored
as `Table [...]` / `Proves [...]` pairs. 10 magmas of size 2, 299 of size 3, 515
of size 4 — 824 in total. `Proves` is the complete list of laws that magma
satisfies, i.e. a signature in list form.

Alongside it sits `src/check_redundant.py`, which computes exactly the
size-tier coverage deltas this project's Tier F was going to produce, and a
Haskell `min-cover` tool that solves the exact set-cover over that corpus with
an SMT solver. Its own README says min-cover "is very slow for large equation
sets or many magmas, and it's not intended to be used on full sets of either."

The numbers in the repo README (post-correction):

| corpus | implications refuted |
|---|---|
| magmas of size ≤2 | 12,560,783 |
| magmas of size ≤3 | 13,596,121 |
| magmas of size ≤4 | 13,753,982 |
| size 3 alone | 13,338,841 |
| size 4 alone | 13,726,214 |

Bruno Le Floch computed these in October 2025 after finding an off-by-one in
`check_redundant.py` (it allocated a 4695×4695 matrix and summed the whole
thing, counting phantom equations 0 and 4695). Douglas McNeil independently
confirmed all three deltas by brute force. Le Floch also recomputed the minimal
covering corpus as **523** magmas (515 of size 4, plus the 8 of size 3 not
covered by a size-4 model), not 524.

**The arXiv paper (v2, 16 Dec 2025) still carries the pre-correction numbers,
and they are internally inconsistent.** §5.1 says brute force over size ≤4
refutes 13,632,566 implications "with 524 distinct magmas", of which 13,345,053
came from size ≤3 and "the remaining 415,293" from size 4. Those two addends sum
to 13,760,346, not 13,632,566. The stated "96.3% of the false ones" matches
13,345,053/13,855,357, not the headline figure. Le Floch said a correcting PR
was incoming; it did not reach the arXiv version.

So the published Tier-F coverage record is wrong in print and correct only in
the repo, and nobody has re-derived it from an independently written engine.
That is a smaller contribution than "the coverage matrix", but it is real, and
it is a free by-product of building the differential harness.

### 1.2 A live benchmark harness over ETP exists, and its deadline is in four days

**SAIR Mathematics Distillation Challenge — Equational Theories, Stage 2**,
organised by Damek Davis and Terence Tao under the SAIR Foundation.
Repo: `SAIRcompetition/equational-theories-lean-stage2` (created 2026-05-01,
last push 2026-08-27, i.e. today). Stage 2 opened 1 May 2026;
**submission deadline 31 August 2026, 23:59 AoE.**

What it is: given a pair of laws, emit a machine-verifiable Lean 4 certificate —
a proof if the implication holds, **a magma witness (finite or infinite) if it
does not**. Deterministic Lean judge, no partial credit. Two tracks (Solo:
3600s/problem; Marathon: 100 problems on a shared budget, explicitly rewarding
triage and cross-problem caching).

Two things matter here.

- **It is not an RL environment.** It is an LLM-solver competition with a Lean
  judge. The brief's premise — no RL env or benchmark harness exists — is half
  right: no RL env, but a harness now exists.
- **Its problem set is order-5 laws** (`examples/problems/eq_size5.txt`, ~62K
  laws), where the implication graph is *not* settled. A construction corpus
  plus a fast signature engine is close to the ideal solver for the negative
  half of that task, and "cross-problem caching" in the Marathon track is
  literally what a precomputed signature corpus provides.

Four days is not enough to build Phase A and enter. Flagging it because it
changes what Phase A is *for*: the natural consumer of a construction corpus is
now a live, judged, order-5 problem set rather than the closed order-4 graph.

The judge's allowed axioms are `propext`, `Quot.sound`, `Classical.choice`.
`Lean.ofReduceBool` is out, and the repo carries a regression test named
`dbg_trace_hide_native_decide.answer.json` — they test for people smuggling it
in. Same constraint as Palomar. The brief's instruction to design export around
plain `decide` is correct and now has two independent enforcers.

### 1.3 Prior art that is adjacent but not overlapping

- **Janota, arXiv:2508.15856** (20 Aug 2025), "Experimental Results for Vampire
  on the Equational Theories Project". Vampire 5.0.0 over all 22,028,942 pairs,
  five configurations. This is the definitive hard-core measurement; numbers in
  §2 below. No coverage analysis.
- **arXiv:2605.21200** (20 May 2026), Kondylidou, Blanchette, Heule, "Tao's
  Equational Proof Challenge Accepted". Introduces **Krympa**, a proof
  minimizer; cuts a 62-step Vampire proof of 650⟹448 to 20 steps, and a
  151-step proof to 10. Evaluated on 1431 provable implications. Proof-length
  minimization, not construction economy. No overlap.
- **ML on ETP** (paper §11): a character-level CNN predicting implication truth
  (99.7% test accuracy, and still 92.2% trained on 0.1% of the data); a
  transformer; GNN autoencoders for directed link prediction. All predict the
  *graph*. None search for constructions. `scripts/predictor/` in the repo holds
  a hand/LLM-written syntactic implication-probability heuristic.
- **`scripts/find_powerful_theorems.py`** ranks *unknown* implications by how
  many pairs resolving them would settle. Scheduling, not coverage.
- **Zulip, "Brute-forcing with shared subterms"** (Breitner → Carlini,
  2 Oct 2024): the shared-subterm-DAG idea was proposed and Carlini replied that
  evaluation was not the bottleneck — table-saving and dedup scaffolding were.
  Worth internalising before optimising the evaluator.
- ETP's own brute forcer is `src/brute_force_4x4_tables.c` plus a **generated**
  `equations.c` of 1.2 MB. They took the codegen route the brief rules out.

No published coverage or construction-economy analysis for the infinite
families. No RL environment. Nothing to stop for.

---

## 2. Exact counts — four different sets, kept apart

Base: 4694 laws of order ≤4, in **1415** equivalence classes; the largest class
is the 1496 laws equivalent to E2 (`x ≃ y`). 4694² = 22,033,636 ordered pairs,
22,028,942 excluding reflexive.

| quantity | value | source |
|---|---|---|
| true implications, general magmas (incl. reflexive) | 8,178,279 (37.12%) | paper §1.3 |
| true, non-reflexive | 8,173,585 | derived; matches Janota's "proven" total exactly |
| false implications, general magmas | 13,855,357 (62.88%) | paper §1.3 |
| positive implications needing a direct Lean proof (generating set) | 10,657 | paper §1.3 |
| negative implications formalized in Lean (generating set) | 586,925 | paper §3 |

### The four "hard" sets

These are not the same set and the brief is right to insist on it.

**(A) Not refuted by any small finite magma — 101,375.**
13,855,357 − 13,753,982. False implications with no counterexample among magmas
of size ≤4. Computed from the corrected repo numbers, not the paper's.

**(B) Unresolved by Vampire — 1,062.**
Janota ran five configurations (fmb/saturation, 500 instructions / 60s / 600s)
and solved 22,027,880 of 22,028,942. Refuted 13,854,295, proved 8,173,585 —
that is *all* true implications, so the entire residue is negative. 1,062 left.
Janota notes 22 more follow by transitivity. This is Tao's "core of just under
a thousand". **This is the set I propose as the primary denominator for
"fraction of the hard core covered".**

**(C) Truth value changes under finiteness — 820, or 822 counting the open pair.**
Paper §8: 820 pairs with `E ⊭ E′` but `E ⊧_fin E′`. Exactly one implication (up
to duality) is unresolved for finite magmas: **E677 ⊧_fin E255**, dual
E2910 ⊧_fin E47, conjectured false. So false-even-for-finite is 13,854,535 or
13,854,537. Every one of the 820 necessarily has an infinite-only counterexample.

**(D) Curated as hard in the blueprint — 186 ordered pairs.**
Blueprint ch. 27, counted directly: slow-but-doable ATP positives, refutations
needing greedy methods, ad hoc refutations, and the final analysis set including
the "Hardy–Ramanujan cluster" (917/1323/1526/1729/2541/2744) and E1729 ⊭ E817,
which took months and ~4000 lines of Lean.

**One inconsistency I could not resolve, and it should be treated as a Phase A
target rather than papered over.** Janota writes that "only 310 of the undecided
implications require an infinite model according to the Equational project."
Set (C) says 820 pairs admit no finite counterexample, and every such pair is
unrefutable by a finite model builder, so it should sit inside the 1,062. 820 >
310. The likely explanation is that his 310 counts pairs *tagged* in ETP data as
having infinite-model proofs (a generating-set property) while 820 is a closure
count — and separately, his saturation runs "refuted" 778 + 36 = 814 pairs
without producing a witness, which is suspiciously close to 820 and is probably
where most of that set went. Reconciling 310 / 814 / 820 against the raw
`2025-08-11-vampire.json.gz` is a concrete, cheap, first-week task, and its
answer pins down which set the coverage denominator should be.

---

## 3. Oracle inventory — what the differential harness gets to test against

All confirmed present and fetched.

| artifact | size | what it gives |
|---|---|---|
| `data/equations.txt` | 4694 lines | the law list, canonical numbering |
| `data/smallest_magma.txt` | 3198 lines | law → size of smallest nontrivial model |
| `data/smallest_magma_examples.txt` | 3198 lines | law → explicit multiplication table |
| `All4x4Tables/data/refutations{2,3,4}x{2,3,4}.txt` | 824 magmas | table + **complete** satisfied-law list |
| `full_entries.json` | 12,373 entries | every formalized Lean result |
| `data/2025-08-11-vampire.json.gz` | 84 MB | Vampire verdict + time + method for all 22M pairs |
| `data/eq_size5.txt` | 2.5 MB | ~62K laws of order ≤5, the SAIR problem space |

The 3198 smallest-magma entries split by carrier size as
**3136 / 32 / 14 / 14 / 2** for sizes 2 / 3 / 4 / 5 / 7 — I reproduced this from
the raw file and it matches paper Table 1 exactly. 4694 − 1496 = 3198 confirms
that every law not equivalent to E2 has a nontrivial finite model (Kisielewicz),
with E1286 and its dual E2301 the sole size-7 outliers.

The 824-magma corpus is the strongest oracle available: each entry pairs a table
with its *complete* satisfied-law set, so it tests a full 4694-bit signature, not
a handful of bits. The brief asks for agreement against smallest-magma-per-
equation up to N=5; the 824-magma corpus is a strictly stronger test and should
be the primary one. Combined corpus for differential testing: 3,198 +824 magmas,
of which the 824 give full-signature agreement and the 3,198 give
smallest-model-size agreement.

`full_entries.json` breaks down as 10,674 implication entries (10,669 general +
5 finite) and 1,698 `facts` entries — 1,106 finite, 338 general, 254 unproven
conjectures. Each proven `facts` entry is a magma with a partial signature
(mean 4.1 satisfied, 41 refuted labels), expanding to 1,318,771 negative pairs.
These are partial signatures, so they test containment, not equality.

---

## 4. Constructions, as the paper names them

Order for implementation, with paper sections to cite in comments.

| family | paper § | verification route |
|---|---|---|
| finite enumeration, small carriers | §5.1 | table sweep |
| linear models `x ⋄ y = ax + by` over commutative rings | §5.2 | Gröbner basis over coefficient variety |
| linear models over noncommutative rings | §5.2 | diamond lemma / Bergman |
| translation-invariant `x ⋄ y = x + f(y−x)` | §5.3 | functional equation |
| twisting semigroup `S_E` | §5.4 | semigroup comparison |
| greedy constructions | §5.5 | inherently infinite; no finite analogue |
| submagma / projection / magma cohomology extensions | §5.6 | cohomological |
| free magmas from confluent laws & complete rewriting systems | §6 | rewriting |

Two things the brief's ordering should absorb. Greedy methods (§5.5) carry a
large share of the blueprint's hard list — count the entries under "required
greedy methods" in ch. 27 — and they are the one family with **no finite
counterpart at all**, which is exactly why E677 ⊧_fin E255 stayed open. If the
goal is hard-core coverage, greedy is not an optional late family; skipping it
guarantees a poor coverage number for reasons that are architectural rather than
mathematical. Second, "free magmas from laws with complete rewriting systems" is
§6 (syntactic), not §5, and its verification is Knuth–Bendix completion, not
polynomial identity — it does not belong in the same tier as the linear models.

---

## 5. What I recommend changing before writing code

1. **Pin the hard-core denominator to set (B), 1,062**, and report against
   (C) 820 and (D) 186 as secondary lines. State all four in every coverage
   total. First task: reconcile 310/814/820 from the raw Vampire dump.
2. **Demote Tier F from "survey" to "verification".** The survey exists. Rerun
   it with an independently written engine, agree or disagree with 12,560,783 /
   13,596,121 / 13,753,982 and with 523 magmas, and publish the reconciliation
   against the paper's printed 13,632,566 / 524. That is a day of compute and a
   real correction to the record, and it doubles as the differential harness.
3. **Promote greedy constructions** out of "later" and into the core family set,
   on the grounds that they dominate the hard list and have no finite proxy.
4. **Treat the 824-magma corpus as the primary differential oracle**, above the
   smallest-magma-per-equation set.
5. Decide whether SAIR Stage 2 (deadline 31 Aug) matters. I assume not for
   Phase A, but the order-5 law set is the more interesting target for the
   corpus once it exists, and it is where the graph is still open.

---

## Sources

- Tao et al., *The Equational Theories Project*, arXiv:2512.07087v2, 16 Dec 2025.
- Janota, *Experimental Results for Vampire on the ETP*, arXiv:2508.15856, 20 Aug 2025.
- Kondylidou, Blanchette, Heule, *Tao's Equational Proof Challenge Accepted*, arXiv:2605.21200, 20 May 2026.
- `github.com/teorth/equational_theories` @ main, pushed 2026-08-23. Apache-2.0.
- ETP blueprint ch. 27, "Hard implications".
- Lean Zulip, #Equational: "Counting refutations by tiny magmas" (Oct 2025), "Brute-forcing with shared subterms" (Oct 2024).
- `github.com/SAIRcompetition/equational-theories-lean-stage2`, pushed 2026-08-27.
- Tao, *Palomar — a registry of Lean verified mathematics*, 18 Aug 2026.
