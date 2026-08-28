#!/usr/bin/env python3
"""Regenerate the control-experiment samples in out/atp/samples/.

The ATP control stratifies the hard core by the smallest carrier the
construction corpus can witness each pair on, then samples within each
stratum. Seed is fixed so the samples in out/atp/samples/ are reproducible;
if you change the seed you change which problems the numbers in
docs/atp-control.md refer to.
"""
import random
import pathlib

SEED = 20260828
ROOT = pathlib.Path(__file__).resolve().parent.parent
rows = [
    l.rstrip("\n").split("\t")
    for l in (ROOT / "data/etp/hard_core_partition.tsv").read_text().splitlines()
][1:]

out = ROOT / "out/atp/samples"
out.mkdir(parents=True, exist_ok=True)
random.seed(SEED)

# 20 pairs from the unresolved set whose smallest witness has 13 elements.
p13 = [(r[1], r[2]) for r in rows if r[0] == "unresolved" and r[3] == "13"]
(out / "unresolved_witness13.txt").write_text(
    "".join(f"{a} {b}\n" for a, b in random.sample(p13, 20))
)
# Companion set from the saturation-refuted side, 7-element witnesses.
p7 = [(r[1], r[2]) for r in rows if r[0] == "saturation_refuted" and r[3] == "7"]
(out / "saturation_witness7.txt").write_text(
    "".join(f"{a} {b}\n" for a, b in random.sample(p7, 20))
)
# 8 pairs at each witness size, across both sets, for the cliff measurement.
random.seed(SEED)
for size in ("7", "9", "11"):
    sel = [(r[1], r[2]) for r in rows if r[3] == size]
    k = min(8, len(sel))
    (out / f"witness{size}.txt").write_text(
        "".join(f"{a} {b}\n" for a, b in random.sample(sel, k))
    )
print(f"regenerated samples in {out} with seed {SEED}")
