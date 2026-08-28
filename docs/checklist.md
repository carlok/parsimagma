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
- [>] 2.6  Write `docs/atp-control.md` with both verdicts
- [ ] 2.7  Commit

## Blocked / not started

- [ ] S2   Identify the method for the 296-pair cluster
- [ ] S3   Greedy spike, single target, 3-day kill criterion
- [ ] S4   Greedy generalised — conditional on S3
- [ ] S5   Phase B set-cover bounds — needs an explicit go
- [ ] --   Simpler-witnesses comparison — deprioritised, ~25% odds
