# Quadratic magmas: 93 pairs reached, none of them new

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

`x ◇ y = ax² + bxy + cy² + dx + ey + f` over `Z/N`, ETP paper Remark 5.5, all
six coefficients swept for `N = 2..13`. Linear instances are skipped, being
already covered exhaustively elsewhere.

| N | instances | hit something | pairs reached | new |
|---|---|---|---|---|
| 2–8 | 446,963 | 0 | 0 | 0 |
| 9 | 531,441 | 4 | 4 | 0 |
| 10 | 1,000,000 | 0 | 0 | 0 |
| 11 | 1,771,561 | 160 | 93 | 0 |
| 12 | 2,985,984 | 0 | 0 | 0 |
| 13 | 4,826,809 | 48 | 91 | 0 |
| **total** | **11,562,758** | **212** | **93** | **0** |

The family is more productive than translation-invariant models — 93 pairs
against 16 — and still adds nothing. Everything it reaches, the linear sweep
already reached.

The `N` pattern is worth noting. Nothing at all for `N ≤ 8`, and nothing at 10
or 12, with the hits concentrated at 9, 11 and 13. Composite moduli are dead
here and the odd prime-power moduli carry everything, which is consistent with
the linear family's own behaviour: `Z/13` supplies 268 of the 416 covered
pairs.

The paper predicted this family would thin out as `N` grows, because the
polynomial attached to a word has degree exponential in the word's order —
16 for a law of order 4. What the sweep adds is that it also thins out
*downward*: below carrier 9 there is nothing, because that range is already
exhausted by brute force over all magmas.

## Reproducing

```bash
cargo run --release --bin pm -- quad 13
```

Arguments are `max_n` then optional `min_n`.
