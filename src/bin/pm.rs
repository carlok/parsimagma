//! `pm` — command line front end.

use parsimagma::corpus::{linear_corpus, Carrier};
use parsimagma::coverage::{CoverageMatrix, GraphCoverage};
use parsimagma::etpdata::parse_refutations;
use parsimagma::graph::{parse_pairs, ImplicationGraph};
use parsimagma::linear::LinearLaws;
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma, N_LAWS_ORDER4};
use std::time::Instant;

fn data(name: &str) -> String {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/data/etp/");
    std::fs::read_to_string(format!("{p}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "stats".into());
    match cmd.as_str() {
        "stats" => stats(),
        "coverage" => coverage(),
        "bruteforce" => bruteforce(),
        other => {
            eprintln!("unknown command {other:?}; try: stats, coverage, bruteforce");
            std::process::exit(2);
        }
    }
}

fn stats() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let dag = Dag::build(&laws);
    let independent: usize = laws.iter().map(|l| l.lhs.ops() + l.rhs.ops()).sum();
    let e = Engine::new(dag);

    println!("laws                     {}", e.n_laws());
    println!("applications in the laws {independent}");
    println!("distinct subterms (DAG)  {}", e.dag.n_op_nodes());
    println!(
        "sharing factor           {:.1}x",
        independent as f64 / e.dag.n_op_nodes() as f64
    );
    println!("\narity buckets");
    println!("  vars  laws  sub-DAG nodes");
    for b in &e.plan.buckets {
        println!(
            "  {:>4}  {:>4}  {:>13}",
            b.arity,
            b.laws.len(),
            b.order.len()
        );
    }
    for n in [2usize, 3, 4, 5] {
        println!("worst-case node evals at n={n}: {:>12}", e.cost(n));
    }

    // Throughput on the real oracle corpus.
    let mut corpus: Vec<FiniteMagma> = Vec::new();
    for f in [
        "refutations2x2.txt",
        "refutations3x3.txt",
        "refutations4x4.txt",
    ] {
        corpus.extend(
            parse_refutations(&data(f))
                .unwrap()
                .into_iter()
                .map(|r| r.magma),
        );
    }
    let reps = 20;
    let t = Instant::now();
    let mut acc = 0u64;
    for _ in 0..reps {
        for m in &corpus {
            acc += e.signature(m).count() as u64;
        }
    }
    let el = t.elapsed();
    let total = corpus.len() * reps;
    println!(
        "\nscalar throughput  {:.0} signatures/s  ({} magmas x{reps} in {:.2?}, checksum {acc})",
        total as f64 / el.as_secs_f64(),
        corpus.len(),
        el
    );
}

fn coverage() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let ll: &'static LinearLaws = Box::leak(Box::new(LinearLaws::build(&laws)));

    let t = Instant::now();
    let corpus = linear_corpus(ll);
    let build_time = t.elapsed();

    let hard = parse_pairs(&data("hard_core.txt")).unwrap();
    let graph = ImplicationGraph::from_bytes(
        N_LAWS_ORDER4,
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/etp/implications.bits"
        ))
        .unwrap(),
    );

    println!("# Coverage report");
    println!();
    println!("law set                4694 laws of order <= 4 (ETP equations.txt)");
    println!(
        "target                 {} pair hard core: implications Vampire left",
        hard.len()
    );
    println!("                       undecided (Janota, arXiv:2508.15856, run 2025-08-11)");
    println!();
    println!("## Parameter grid");
    println!();
    let mut enumerated = 0usize;
    for g in &corpus.grid {
        println!(
            "{:<18} {:>7} enumerated  {:>6} distinct signatures",
            g.family, g.enumerated, g.distinct
        );
        println!("    {}", g.description);
        enumerated += g.enumerated;
    }
    println!();
    println!(
        "total {enumerated} instances enumerated, {} distinct rows, built in {:.2?}",
        corpus.instances.len(),
        build_time
    );
    let infinite = corpus
        .instances
        .iter()
        .filter(|i| i.carrier == Carrier::Infinite)
        .count();
    let unsweepable = corpus
        .instances
        .iter()
        .filter(|i| !i.carrier.table_checkable())
        .count();
    println!("  {infinite} have an infinite carrier; {unsweepable} are past any table sweep");

    println!();
    println!("## Coverage of the hard core");
    println!();
    let cm = CoverageMatrix::build(&corpus, &hard);
    let covered = cm.covered();
    println!(
        "{:>6} / {} hard-core pairs discharged  ({:.2}%)",
        covered.len(),
        hard.len(),
        100.0 * covered.len() as f64 / hard.len() as f64
    );

    let mut by_family: std::collections::BTreeMap<&str, usize> = Default::default();
    for (i, row) in cm.rows.iter().enumerate() {
        if !row.is_empty() {
            *by_family.entry(corpus.instances[i].family).or_default() += 1;
        }
    }
    if by_family.is_empty() {
        println!("no instance in the grid discharges any hard-core pair");
    } else {
        println!();
        println!("contributing instances by family:");
        for (f, n) in &by_family {
            println!("  {f:<18} {n}");
        }
        let cover = cm.greedy_cover();
        println!();
        println!(
            "greedy cover: {} instances suffice for the {} pairs reached",
            cover.len(),
            covered.len()
        );
        println!("  (an upper bound within ln(n) of a smallest cover, not a minimum)");
        for (i, gain) in cover.iter().take(12) {
            let inst = &corpus.instances[*i];
            println!("    +{gain:<5} {} [{}]", inst.family, inst.params);
        }
        println!();
        let hit: std::collections::HashSet<u32> = covered.iter().copied().collect();
        let mut miss_by_src: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut hit_by_src: std::collections::BTreeMap<u32, usize> = Default::default();
        for (k, p) in hard.iter().enumerate() {
            if hit.contains(&(k as u32)) {
                *hit_by_src.entry(p.from).or_default() += 1;
            } else {
                *miss_by_src.entry(p.from).or_default() += 1;
            }
        }
        let mut miss: Vec<_> = miss_by_src.iter().map(|(a, b)| (*b, *a)).collect();
        miss.sort_unstable_by(|x, y| y.cmp(x));
        println!(
            "uncovered hard-core pairs by hypothesis law (top 20 of {} laws):",
            miss_by_src.len()
        );
        for (n, src) in miss.iter().take(20) {
            println!(
                "    E{src:<6} {n:>4} uncovered, {:>4} covered",
                hit_by_src.get(src).copied().unwrap_or(0)
            );
        }
        println!();
        println!("pairs reached, first 40:");
        for k in covered.iter().take(40) {
            let p = hard[*k as usize];
            print!(" {}!=>{}", p.from, p.to);
        }
        println!();
    }

    println!();
    println!("## Coverage of the whole graph");
    println!();
    let mut gc = GraphCoverage::new(N_LAWS_ORDER4);
    for inst in &corpus.instances {
        gc.add(&inst.sig);
    }
    if let Err((i, j)) = gc.check_against(&graph) {
        panic!("BLOCKING: corpus claims E{i} does not imply E{j}, but the ETP graph says it does");
    }
    let total_false = 13_855_357u64;
    println!(
        "{:>10} of {total_false} false implications discharged  ({:.2}%)",
        gc.count(),
        100.0 * gc.count() as f64 / total_false as f64
    );
    println!("every discharged pair checked against the published graph: no contradictions");

    write_raw_outputs(&corpus, &cm, &hard, gc.count());
}

/// Everything the report quotes, written to `out/` so the numbers can be
/// checked without rerunning anything.
fn write_raw_outputs(
    corpus: &parsimagma::corpus::Corpus,
    cm: &CoverageMatrix,
    hard: &[parsimagma::graph::Pair],
    graph_covered: u64,
) {
    use std::io::Write;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/out");
    std::fs::create_dir_all(dir).unwrap();

    let mut g = std::io::BufWriter::new(std::fs::File::create(format!("{dir}/grid.txt")).unwrap());
    writeln!(
        g,
        "# Parameter grid. Coverage totals are meaningless without it."
    )
    .unwrap();
    for spec in &corpus.grid {
        writeln!(
            g,
            "{}\t{}\t{}\t{}",
            spec.family, spec.enumerated, spec.distinct, spec.description
        )
        .unwrap();
    }
    drop(g);

    // One row per instance that discharges at least one hard-core pair.
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(format!("{dir}/coverage_hardcore.tsv")).unwrap(),
    );
    writeln!(
        f,
        "family\tparams\tcarrier\tlaws_satisfied\tpairs_covered\tpairs"
    )
    .unwrap();
    for (i, row) in cm.rows.iter().enumerate() {
        if row.is_empty() {
            continue;
        }
        let inst = &corpus.instances[i];
        let carrier = match inst.carrier {
            Carrier::Finite(n) => format!("finite:{n}"),
            Carrier::Infinite => "infinite".to_string(),
        };
        let pairs: Vec<String> = row
            .iter()
            .map(|&k| format!("{}!={}", hard[k as usize].from, hard[k as usize].to))
            .collect();
        writeln!(
            f,
            "{}\t{}\t{carrier}\t{}\t{}\t{}",
            inst.family,
            inst.params,
            inst.sig.count(),
            row.len(),
            pairs.join(",")
        )
        .unwrap();
    }
    drop(f);

    // Raw signatures: 587 bytes each, in corpus order, with a parallel index.
    let mut sig = std::io::BufWriter::new(
        std::fs::File::create(format!("{dir}/corpus_signatures.bin")).unwrap(),
    );
    let mut idx =
        std::io::BufWriter::new(std::fs::File::create(format!("{dir}/corpus_index.tsv")).unwrap());
    writeln!(idx, "row\tfamily\tparams\tcarrier\tlaws_satisfied").unwrap();
    for (i, inst) in corpus.instances.iter().enumerate() {
        sig.write_all(&inst.sig.to_bytes()).unwrap();
        let carrier = match inst.carrier {
            Carrier::Finite(n) => format!("finite:{n}"),
            Carrier::Infinite => "infinite".to_string(),
        };
        writeln!(
            idx,
            "{i}\t{}\t{}\t{carrier}\t{}",
            inst.family,
            inst.params,
            inst.sig.count()
        )
        .unwrap();
    }
    drop(sig);
    drop(idx);

    let covered = cm.covered().len();
    let mut sum =
        std::io::BufWriter::new(std::fs::File::create(format!("{dir}/summary.tsv")).unwrap());
    writeln!(sum, "metric\tvalue").unwrap();
    writeln!(sum, "laws\t{N_LAWS_ORDER4}").unwrap();
    writeln!(sum, "corpus_rows\t{}", corpus.instances.len()).unwrap();
    writeln!(sum, "hard_core_pairs\t{}", hard.len()).unwrap();
    writeln!(sum, "hard_core_covered\t{covered}").unwrap();
    writeln!(sum, "graph_false_implications\t13855357").unwrap();
    writeln!(sum, "graph_covered\t{graph_covered}").unwrap();
    writeln!(sum, "greedy_cover_size\t{}", cm.greedy_cover().len()).unwrap();
    drop(sum);

    println!();
    println!("raw outputs written to out/: grid.txt, coverage_hardcore.tsv,");
    println!(
        "  corpus_signatures.bin ({} x 587 bytes), corpus_index.tsv, summary.tsv",
        corpus.instances.len()
    );
}

/// Exhaustive enumeration of every magma of size at most `n`, accumulating
/// which implications they refute.
///
/// This reproduces the counts in `All4x4Tables/README.md`, which were
/// recomputed by Bruno Le Floch in October 2025 after an off-by-one was found
/// in the ETP's own `check_redundant.py` (it summed a 4695 x 4695 matrix,
/// counting two equations that do not exist). The arXiv paper still carries
/// the pre-correction figures, and they do not add up: section 5.1 reports
/// 13,632,566 refutations of which 13,345,053 come from size <= 3 and
/// "the remaining 415,293" from size 4, but those two addends sum to
/// 13,760,346.
fn bruteforce() {
    use parsimagma::finite::{is_canonical, permutations, Scratch, EXACT_ISO_CAP};
    use rayon::prelude::*;

    let max: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let e = Engine::new(Dag::build(&laws));
    let graph = ImplicationGraph::from_bytes(
        N_LAWS_ORDER4,
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/etp/implications.bits"
        ))
        .unwrap(),
    );

    println!("# Exhaustive small-magma enumeration");
    println!();
    println!("reference: equational_theories/Generated/All4x4Tables/README.md");
    println!("  size <= 2   12560783 refutations");
    println!("  size <= 3   13596121");
    println!("  size <= 4   13753982");
    println!();

    let mut gc = GraphCoverage::new(N_LAWS_ORDER4);
    let mut separating_total = 0u64;
    for n in 2..=max {
        assert!(
            n <= EXACT_ISO_CAP,
            "exhaustive sweep capped at size {EXACT_ISO_CAP}"
        );
        let perms = permutations(n);
        let total: u64 = (n as u64).pow((n * n) as u32);
        let t = Instant::now();
        // Fold per worker rather than collecting per chunk: one accumulator
        // is 2.8 MB, and the size-4 sweep has hundreds of thousands of
        // chunks. Chunks stay small because the efficiency cores on Apple
        // silicon are much slower than the performance cores and straggle at
        // the end of a long chunk.
        let chunk = 1u64 << 14;
        let nchunks = total.div_ceil(chunk);
        // Progress, so a multi-hour sweep is observable rather than silent.
        let done = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watcher = {
            let done = done.clone();
            let stop = stop.clone();
            let start = Instant::now();
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    let d = done.load(std::sync::atomic::Ordering::Relaxed);
                    if d == 0 || stop.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    let frac = d as f64 / nchunks as f64;
                    let el = start.elapsed().as_secs_f64();
                    eprintln!(
                        "  n={n}: {:.2}% ({d}/{nchunks} chunks) elapsed {:.0}s, eta {:.0}s",
                        100.0 * frac,
                        el,
                        el / frac - el
                    );
                }
            })
        };
        struct Acc {
            gc: GraphCoverage,
            sep: u64,
            canon: u64,
        }
        let (bits, sep, canon) = (0..nchunks)
            .into_par_iter()
            .fold(
                || {
                    (
                        Acc {
                            gc: GraphCoverage::new(N_LAWS_ORDER4),
                            sep: 0,
                            canon: 0,
                        },
                        Scratch::new(&e.dag),
                        vec![0u8; n * n],
                        vec![0u8; n * n],
                    )
                },
                |(mut acc, mut scratch, mut table, mut buf), c| {
                    let lo = c * chunk;
                    let hi = (lo + chunk).min(total);
                    for code in lo..hi {
                        let mut v = code;
                        for cell in table.iter_mut() {
                            *cell = (v % n as u64) as u8;
                            v /= n as u64;
                        }
                        // Keep only the lexicographically least member of
                        // each isomorphism class: relabelled magmas have
                        // identical signatures and add nothing but time.
                        if !is_canonical(&table, n, &perms, &mut buf) {
                            continue;
                        }
                        acc.canon += 1;
                        let m = FiniteMagma::new(n, table.clone()).unwrap();
                        let sig = e.signature_with(&m, &mut scratch);
                        if sig.is_separating() {
                            acc.sep += 1;
                            acc.gc.add(&sig);
                        }
                    }
                    done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    (acc, scratch, table, buf)
                },
            )
            .map(|(acc, _, _, _)| (acc.gc, acc.sep, acc.canon))
            .reduce(
                || (GraphCoverage::new(N_LAWS_ORDER4), 0u64, 0u64),
                |mut a, b| {
                    a.0.or_bits(b.0.bits());
                    (a.0, a.1 + b.1, a.2 + b.2)
                },
            );
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        watcher.thread().unpark();
        drop(watcher);
        gc.or_bits(bits.bits());
        separating_total += sep;
        let el = t.elapsed();
        println!(
            "size <= {n}:  {:>10} refutations   ({total} tables, {canon} canonical, {:.2?}, {:.2e} tables/s)",
            gc.count(),
            el,
            total as f64 / el.as_secs_f64()
        );
    }

    println!();
    println!("{separating_total} of the swept tables separate at least one pair");
    match gc.check_against(&graph) {
        Ok(()) => println!("every refuted pair agrees with the published implication graph"),
        Err((i, j)) => panic!("BLOCKING: claimed E{i} does not imply E{j}, graph says it does"),
    }
}
