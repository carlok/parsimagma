# Marathon mode — design and decision log

## What the harness gives and expects

    JUDGE_MARATHON_MANIFEST         path to a JSONL file, one problem per line
    JUDGE_MARATHON_OUTPUT           path to append answers to, one JSON per line
    JUDGE_MARATHON_BUDGET_SECONDS   one global wall-clock budget for all of them

    answer line: {"id": ..., "verdict": "true"|"false", "code": "<lean>"}

Scored at the end of the run. **No judge feedback during it.** Reference N=100,
so the budget is 100 x 5 minutes = 30,000 seconds shared.

## Measured facts this design rests on

From the Solo run of the same solver over 200 problems:

    solved                          198/200
    accepted on the first try       197/198
    wall clock                      32.3 min for 200, so ~16 min for 100
    cost of a problem it solves     ~10 s
    cost of a problem it cannot     ~346 s, the full ladder

So on Marathon's own reference numbers the solver would use about 3% of the
budget. The design question is what to do with the other 97%.

## Decisions

### D1 — Commit blind, no judge loop
**Decision.** Emit the first certificate that verifies internally, and move on.
**Alternatives.** Keep a judge loop (impossible, no feedback in this track); or
emit several candidates per problem (the format allows one line per id).
**Rationale.** 197 of 198 were accepted first try, so the feedback loop was
already doing almost nothing. Internal verification is what earns that: every
proof path is replayed before emission, every counterexample is checked against
both laws on the whole carrier.
**Risk accepted.** At most one problem in 200.

### D2 — Cheap-first ordering, not manifest order
**Decision.** Two passes. First pass gives every problem a short slice in
manifest order; second pass returns to the unsolved with the remaining budget.
**Alternatives.** Manifest order with a uniform cap, as the demo does.
**Rationale.** Answers are appended as they are found, so a SIGTERM or an
overrun costs whatever has not been written yet. Banking the cheap 90% first
means an early kill loses the hard tail rather than an arbitrary suffix.
**Risk accepted.** Slightly more code than a single loop.

### D3 — Per-problem cap derived from the budget, not fixed
**Decision.** First pass caps at `budget * 0.25 / N`. Second pass divides what
is left among the still-unsolved, capped so no single problem can take more
than a fifth of the remainder.
**Alternatives.** The demo's `budget / N` uniform cap.
**Rationale.** The cost distribution is bimodal, not uniform: ~10 s or the
whole ladder. A uniform cap spends the same on both and wastes the headroom
that makes this track worth entering.

### D4 — Tail margin, and fsync every line
**Decision.** Stop starting new problems with 60 s left; fsync after each
appended line.
**Rationale.** The demo reserves 5 s. That is enough to finish one `write`,
not enough to finish a problem already in flight — and our per-problem work can
run minutes. 60 s covers the longest single judge-free step we measured.

### D5 — Every problem in its own try/except
**Decision.** Any exception on one problem is swallowed and the loop continues.
**Rationale.** In Solo a crash costs one problem. In Marathon it costs every
problem after it. This is the same fail-closed discipline the Solo ladder
already uses, and the reason it is there is that a name-prefixing bug once cost
17 problems in a single run.

### D6 — One file, dispatched on the env var
**Decision.** `if "JUDGE_MARATHON_MANIFEST" in os.environ: run_marathon()` else
the existing Solo `main()`. The Solo path is not touched.
**Verification.** The Solo behaviour must be byte-identical: re-run the full
200 and require exactly the same 198, or the change is reverted.

## Objections raised, and what changed

Found by reading `pipeline/marathon_score.py` and `pipeline/marathon_runner.py`,
which the design above was written without. All three contradict it.

### O1 — D1 is wrong: the output is last-write-wins, not write-once
`marathon_score.py:160` `_load_last_writes`: "the solver's append-only JSONL
output; last-write-wins per id", and malformed lines are skipped silently, so a
later well-formed line for the same id supersedes anything earlier.

D1 said "emit the first certificate that verifies internally, and move on",
which leaves free insurance on the table. **Revised:** append a provisional
answer the moment one exists, and allow it to be superseded later. If the
process is killed at any point, whatever was written still counts. There is no
penalty for a superseded line and no feedback that would let us prefer one
verified certificate over another, so the only thing this buys is crash
insurance — but it buys it for nothing.

### O2 — NEW: the answer file has a size cap, enforced by SIGTERM
`marathon_runner.py:70` `_MAX_OUTPUT_BYTES = 50 * 1024 * 1024`, and line 66:
the watchdog "polls and SIGTERMs with reason='output' if exceeded".

Overwriting freely, as O1 now permits, is bounded by this. The largest
certificate measured is 27,520 bytes, so 100 problems have headroom of roughly
18 revisions each — comfortable, but not unlimited, and a bug that rewrote in a
loop would end the run rather than waste a little disk. **Revised:** at most
one supersede per problem, and a running byte count that stops writing at 40 MB.

### O3 — D4's tail margin was reasoned from the wrong thing
`marathon_runner.py:22`: the runner "SIGTERMs the solver process group at the
deadline and SIGKILLs 5 s later; output JSONL is frozen at SIGTERM time (late
writes that hit disk after SIGTERM are" discarded).

So the margin does not protect a write in flight — it has to guarantee the
write happens *before* the deadline, and the 5 s grace is irrelevant to us
because our unit of work is minutes, not milliseconds. D4's 60 s was picked by
guess and happens to be defensible; it is now picked for a reason.

## Still unreviewed

An independent skeptic pass is running against the same sources. Anything it
finds that is not O1-O3 goes here before implementation is called done.
