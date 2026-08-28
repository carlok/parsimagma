# Working checklist

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
- [ ] 7.6  Cliff finding as a separate issue, only if #1473 lands
- [ ] 7.7  Janota email draft, after #1473 lands

## Blocked / not started

- [>] S2   Identify the method for the 296-pair cluster
- [ ] S3   Greedy spike, single target, 3-day kill criterion
- [ ] S4   Greedy generalised — conditional on S3
- [ ] S5   Phase B set-cover bounds — needs an explicit go
- [ ] --   Simpler-witnesses comparison — deprioritised, ~25% odds
