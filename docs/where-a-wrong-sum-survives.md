# Where a wrong sum survives

> Produced by an AI agent (Claude) under human direction. Every figure below
> is reproducible from this repository; the commands are given at the end.

A sentence in the Equational Theories Project paper says that brute force over
all magmas of size at most 4 refutes 13,632,566 implications, of which
13,345,053 came from magmas of size 3 and "the remaining 415,293" from size 4.

Add the two. You get 13,760,346.

Nothing about this is interesting as a mistake. It is interesting as a
question: given how much checking that sentence passed through, why is it
still there?

## What it passed through

The paper has 34 authors. It is on arXiv in a second version. The project it
reports on formalised twenty-two million implications in Lean, precisely so
that nothing would rest on human assertion.

More pointedly, the correct numbers were already in the repository. In October
2025 Bruno Le Floch found the cause: `check_redundant.py` allocated a
4695 x 4695 matrix and summed all of it, counting two equations that do not
exist. He recomputed 12,560,783 / 13,596,121 / 13,753,982, Douglas McNeil
confirmed them independently, and PR #1335 landed the corrections.

That PR touched seven files. It fixed the script. It fixed
`All4x4Tables/README.md`. It fixed the blueprint, and three other paper
chapters. It did not touch `paper/constructions.tex`.

So the repository has been right for ten months, and the prose has been wrong
for the same ten months, four directories away.

## Why that is the normal shape of things

The data was checked because checking it is what the project was for. The
script was checked because someone ran it and compared. The README was fixed
because it is generated from the script's output and the diff was obvious.

The sentence was not checked because nothing checks sentences.

There is no test that reads a paragraph and adds the numbers in it. The
figures in prose are unversioned, uncoupled from the computation that produced
them, and identical in appearance whether they are right or wrong.
"13,632,566" carries exactly as much authority as "13,753,982" if you do not
do the arithmetic, and doing the arithmetic feels like second-guessing a
computation rather than what it actually is, which is free.

The second symptom in that same sentence makes the point better. It says
13,632,566 is "96.3% of the false ones". It is 98.4%. The figure 96.3%
correctly describes 13,345,053, the other number in the sentence. So the
paragraph is not carrying one stale figure; it is carrying pieces of two
different versions of itself, stitched together. That is what an edit history
looks like when the artifact has no way to complain.

## What the machine actually contributed

Very little, and it is worth being precise about how little.

I did not find this by reading carefully. I found it because I was
reimplementing the brute-force sweep for unrelated reasons, got
12,560,783 / 13,596,121 / 13,753,982, and went looking for the published
figures to compare against. The paper disagreed with the repository, and while
working out which to trust, the two addends did not sum to their own total.

That is a differential test finding a discrepancy, not an insight. The same
mechanism that makes property-based tests catch bugs code review misses:
nobody was smarter, something was just checked mechanically that had
previously only been read.

The genuinely hard part was Le Floch's. Finding that a numpy slice was
`matrix[0:4696]` where it should have been `matrix[1:4695]` requires
understanding what the script means. Noticing that two integers do not add up
requires nothing at all.

## The counterweight, which matters

In the same few days of work that turned this up, working under an explicit
brief to be careful:

- Two commits went out unformatted and turned CI red, because I stopped
  re-running `cargo fmt` after switching to editing through generated patches.
- A draft claimed a solver found models "in under a second". The measured range
  was 0.4 to 2.1 seconds.
- The same draft said three model finders solve carrier 7 in seconds. Two do.
  Mace4 does not, and I had the run showing it.
- Compressing a data file left two `std::fs::read` calls on the old path.
  Both compiled. Both failed at runtime. The test suite did not notice, because
  no test drove the binary. A clean clone did.

Four prose-level errors of my own, in a fraction of the time and with far
fewer eyes on the output than a 34-author paper gets. The machine's error rate
on claims-about-its-own-work was worse, not better. What differed is only that
those errors met something that checks: a formatter, a fresh clone, a habit of
re-reading a draft against the raw run logs.

None of this is an argument that machines check better than people. It is an
argument that **the artifact determines whether anything gets checked at all**,
and prose is the artifact where nothing does.

## The small, boring, actionable version

Numbers that come from a computation should be emitted by that computation.
The project already does this in places: `All4x4Tables/README.md` carries the
literal stdout of `check_redundant.py`, which is exactly why it was easy to fix
and stayed fixed. The paper's `\num{13632566}` is a hand-typed transcription of
the same quantity, and it drifted.

A build step that regenerates a small `numbers.tex` from the data, with the
prose citing macros instead of literals, would have made this class of error
structurally impossible rather than merely unlikely. That is unglamorous, and
it is the whole lesson.

## And the actual point

Nobody's mathematics is wrong. No result changes. The implication graph is
still completely determined and still formally verified, which was the hard
thing and remains an extraordinary piece of work.

What slipped is one sentence describing that work, in the one place where
description is not checkable. It is worth saying plainly that this is not a
failure of care by anyone involved. Le Floch found the real bug, fixed it, and
fixed everything he could see. The sentence was four directories away in a file
he had no reason to open.

Everybody who has ever shipped anything has a version of this.

## Reproducing

```bash
cargo run --release --bin pm -- bruteforce 4
```

Sweeps all 4,294,967,296 tables of size at most 4, 178,981,952 after
isomorphism filtering, about 83 minutes on 11 cores.

```bash
cargo run --release --bin pm -- mincover
```

Derives the covering subset: the 515 magmas of size 4, plus the 8 of size 3
whose refutations they do not cover, all 10 of size 2 being covered. 523 in
total, against the 524 in the paper.

The correction is filed as
[teorth/equational_theories#1473](https://github.com/teorth/equational_theories/pull/1473).
