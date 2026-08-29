# Working checklist

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

Sorted. `[x]` done, `[>]` in progress, `[ ]` queued. Sprint definitions in
[roadmap.md](roadmap.md).

## S1 — Pin the target (partition the 1062)

- [x] 1.1  Re-fetch `2025-08-11-vampire.json.gz` (84 MB) pinned to upstream `e5a88a1`
- [x] 1.2  Stream-parse it — chunked scan, not `json.load`, which wants ~10 GB for 22M entries
- [x] 1.3  Extract the 814 saturation-refuted pairs (`vsi500`/`vss600` with value 0)
- [x] 1.4  Cross them against the 1062 unknowns and against my 416 covered
- [x] 1.5  Partition the 1062: finite witness found / infinite-only / still unknown
- [x] 1.6  Reconcile — 814 is NOT 820; floor of 385 contradicts the published 310
- [x] 1.7  Write `docs/hard-core-anatomy.md`, emit `data/etp/hard_core_partition.tsv`
- [x] 1.8  Delete the dump, report freed bytes
- [x] 1.9  Commit

## Control — is the residual an fmb escalation artifact?

- [x] 2.1  `brew install vampire` (5.1.0, bottled) — 16.4 MB
- [x] 2.2  TPTP emitter `pm tptp`, Janota CNF shape
- [x] 2.3  Sanity check — E2 => E3 returns Unsatisfiable
- [x] 2.4  Experiment A — fixed domain 13: 0/20, and it is NOT the escalation
- [x] 2.5  Experiment B — cliff located between witness size 9 and 11
- [x] 2.6  Write `docs/atp-control.md` with both verdicts
- [x] 2.7  Commit

## S6 — open questions, ranked by odds of a Palomar-admissible theorem

- [x] 6.1  Extract the three order-5 open sets from blueprint ch. 20 (10 / 96 / 24)
- [x] 6.2  Extract the 13 Higman-Neumann unknown candidates
- [x] 6.3  Widen the engine to order-5 and order-8 laws (MAX_DEG 6, MAX_VARS 7)
- [x] 6.4  Validate on the order-5 set: positive control, negative control, table cross-check
- [x] 6.5  Scan linear + affine grid: no hits on any of the 143
- [x] 6.6  Scan 46,538 twisted powers: no hits either
- [x] 6.7  Weyl algebra `Z<a,b>/(ba - ab - 1)` — implemented, validated, no hits
- [x] 6.8  Write up whatever the answer is (`docs/open-questions-scan.md`)

## S7 — the ATP calibration note (the thing people asked for)

- [x] 7.1  Third and fourth methods: Mace4, and z3 on a fully ground encoding
- [x] 7.2  Prove the encoding faithful by pinning the known model (sat in 0.01s)
- [x] 7.3  Close the `fmb` tuning caveat: contour and symmetry_ratio change nothing
- [x] 7.4  Write `docs/atp-control.md` as a standalone, reproducible note
- [x] 7.5  Venue: PR to teorth/equational_theories, filed as #1473 (paper numbers)
- [x] 7.6  Cliff finding filed as issue #1474 on 2026-08-28, without waiting for #1473
- [x] 7.7  Janota email sent 2026-08-28, two points, no PR cited (#1473 still open)

## S8 — pin the hard core exactly (the finite implication graph)

- [x] 8.1  Find the closed finite graph: `home_page/implications/.gitignore` names
           `finite_graph.json`, served by the site but not tracked in the repo
- [x] 8.2  Decode both graphs (RLE `(status, run)`, 4694x4694, codes in `script.js`);
           reproduce the dashboard exactly, recover the 2 unknowns unprompted
- [x] 8.3  820 confirmed as the closure count — closes the open question in #1474
- [x] 8.4  Split both Vampire sets: 1062 = 610 infinite-only / 450 finite / 2 unknown;
           814 = 210 / 604. 610 + 210 = 820 with nothing left over
- [x] 8.5  Differential test against the Lean-verified finite graph: 790/790 finite
           witnesses agree, 7/7 infinite-only labels agree, 0 disagreements
- [x] 8.6  Emit `data/etp/finite_uncovered.txt` — the 39 (of the 1062) and 225 (of
           the 814) finitely-refutable pairs the corpus does not reach
- [x] 8.7  Write `docs/infinite-only-exact.md`; correct the denominator in
           `phase-a-report.md` and the superseded claims in `hard-core-anatomy.md`
- [x] 8.8a Locate ETP's own witnesses for the 39: 30 of them, all in
           All4x4Tables/Refutation930-939, carriers 12 to 65 (docs/the-39.md)
- [x] 8.8b Rediscovery test on Refutation938: it is a section 5.6 extension, the
           law condition is linear in the cocycle, the space is 13^5 not 13^25,
           92% of it separates. Theory is blueprint/chapter/cohomology.tex, not new
- [x] 8.8c Implement section 5.6: src/ext.rs, `pm ext`, tests/ext.rs. Rediscovers
           Refutation938's exact parameters from the ingredients
- [x] 8.8d Closes 6 of the 39 by construction: 411 -> 417 of 450
- [ ] 8.8e The other 33. Widen the grid (nb > 5, m > 13, non-linear bases),
           allow alpha/beta to vary per base pair as the blueprint writes them,
           and handle the 3 laws with more than 3 variables
- [x] 8.9  Report the stale data/full_entries.json as a second comment on #1474
           rather than a third thread. Two threads open: #1473, #1474

## S9 — order 5

- [x] 9.1  `pm order5`: 66,151 linear and affine instances against the 62,576 laws
           of eq_size5, 52s, 2,299,094,885 distinct ordered pairs discharged
- [x] 9.2  Blueprint 20.1-20.3 control wired into every run: 130 laws, 0 satisfied
- [x] 9.3  Correct the framing — order 5 is not unexplored. The blueprint chapter
           'Order 5 Austin laws' classifies all 57,882 by model existence; what is
           missing is the implication graph (docs/order5-first-map.md)
- [ ] 9.4  Ask on Zulip for the per-law order-5 classification as data. The chapter
           enumerates only 130 ids; the 19,392 and 38,360 are counts, and the
           order5 branch data/ matches main. Thread:
           https://leanprover.zulipchat.com/#narrow/channel/458659-Equational/topic/Some.20results.20from.20order.205/near/482118667
- [ ] 9.5  With that data, check the containment law by law: every law this corpus
           satisfies must be one ETP records as having a nontrivial finite model.
           Counts already agree at 41,462 against 41,558

## Blocked / not started

- [x] S2   296-pair cluster is four dual pairs; one ad hoc infinite model on N (docs/cluster-296.md)
- [x] S3'  Translation-invariant models built: 523M candidates, 0 new coverage (docs/translation-invariant.md)
- [x] S3'' Quadratic magmas over Z/N built: 11.5M instances, 93 pairs, 0 new (docs/quadratic.md)
- [x] S3''' Larger matrix rings M_2(F_5), M_3(F_2): 652,769 pairs, 14 reached, 0 new
- [ ] S3   Greedy spike, single target, 3-day kill criterion
- [ ] S4   Greedy generalised — conditional on S3
- [ ] S5   Phase B set-cover bounds — needs an explicit go
- [ ] --   Simpler-witnesses comparison — deprioritised, ~25% odds
