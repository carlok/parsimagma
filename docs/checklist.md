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
- [>] 1.9  Commit

## Control — is the residual an fmb escalation artifact?

- [ ] 2.1  `brew install vampire` (5.1.0, bottled)
- [ ] 2.2  TPTP emitter from the parsed law set (`pm tptp`), matching Janota's CNF shape
- [ ] 2.3  Sanity check: reproduce a known verdict on a handful of settled pairs
- [ ] 2.4  Experiment A — fmb at *fixed* domain size 13 on pairs `Z/13 (7,7)` covers
- [ ] 2.5  Experiment B — fmb with escalation at a large budget, same sample
- [ ] 2.6  Write `docs/atp-control.md` with both verdicts
- [ ] 2.7  Commit

## Blocked / not started

- [ ] S2   Identify the method for the 296-pair cluster
- [ ] S3   Greedy spike, single target, 3-day kill criterion
- [ ] S4   Greedy generalised — conditional on S3
- [ ] S5   Phase B set-cover bounds — needs an explicit go
- [ ] --   Simpler-witnesses comparison — deprioritised, ~25% odds
