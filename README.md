# parsimagma

A magma signature and coverage engine over the 4694 equational laws of the
[Equational Theories Project](https://teorth.github.io/equational_theories/).

For a magma `M`, its **signature** is the 4694-bit vector recording which laws
`M` satisfies — 587 bytes. Every separation question is then a bit test: `M`
separates `E_i` from `E_j` exactly when `sig[i] && !sig[j]`, so one signature
answers all 22,028,942 ordered pairs and coverage over a corpus of magmas is
bitset algebra over packed words.

The question this is built to answer: **for the constructions the ETP used,
which of them cover which separations?**

## Two tiers

**Tier S (symbolic)** decides infinite constructions, which cannot be
table-checked. A linear model `x ◇ y = ax + by` over a ring satisfies a law
exactly when finitely many polynomials in the coefficients vanish, so the whole
family reduces to polynomial identities and needs no carrier at all. This is
where the hard separations live.

**Tier F (finite)** sweeps `n^v` variable assignments against one shared
subterm DAG covering all 4694 laws at once — 3777 distinct subterms rather than
18,311 independent applications.

Instances with finite carriers are decidable both ways, and the two routes are
required to agree bit for bit. That cross-check is what makes Tier S
trustworthy.

## Running it

```bash
cargo test --release          # 21 differential and validation tests
```

```bash
cargo run --release --bin pm -- stats
```

```bash
cargo run --release --bin pm -- coverage
```

```bash
cargo run --release --bin pm -- bruteforce 3
```

`bruteforce 4` sweeps 4.29 billion tables (179 million after isomorphism
filtering) and takes hours.

## Reading the results

- [`docs/phase0-findings.md`](docs/phase0-findings.md) — prior art, the four
  distinct "hard core" counts and why they are not the same set, and a
  correction to the published paper's section 5.1
- [`docs/phase-a-report.md`](docs/phase-a-report.md) — throughput, differential
  agreement, which construction families were implemented and which were not,
  and the coverage totals
- `out/` — raw outputs: the parameter grid, the coverage matrix, and the
  corpus signatures

## Data and reconstruction

`data/etp/` vendors the ETP files used as the differential-test oracle, plus a
few files derived from an 84 MB upstream dump that is deliberately not
vendored. `data/etp/PROVENANCE.txt` lists every file, its upstream path, the
pinned commit, and for the derived ones the exact recipe and the sha256 of the
input.

Everything else in the repository is either source or a committed result.
Nothing needs a network fetch to rebuild except the three files that depend on
that dump, and `PROVENANCE.txt` gives the pinned URL for it.

Generated ATP problem files are not tracked, since they are a pure function of
a sample list and the law set. To rebuild them:

```bash
python3 scripts/sample_control_pairs.py
```

```bash
./target/release/pm tptp out/atp/samples/witness11.txt out/tptp_w11
```

External tools used by the control experiment, neither of which is required to
build or test the crate: Vampire 5.1.0 and Prover9/Mace4, both available from
Homebrew.
