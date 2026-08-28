//! `pm` — command line front end.

use parsimagma::corpus::{add_twist_family, linear_corpus, order3_canonical, Carrier};
use parsimagma::coverage::{CoverageMatrix, GraphCoverage};
use parsimagma::etpdata::parse_refutations;
use parsimagma::graph::{parse_pairs, ImplicationGraph};
use parsimagma::linear::LinearLaws;
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma, N_LAWS_ORDER4};
use rayon::prelude::*;
use std::time::Instant;

use parsimagma::etpdata::read_text as data_text;

fn data(name: &str) -> String {
    data_text(name)
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "stats".into());
    match cmd.as_str() {
        "stats" => stats(),
        "coverage" => coverage(),
        "bruteforce" => bruteforce(),
        "partition" => partition(),
        "tptp" => tptp(),
        "ladr" => ladr(),
        "smt2" => smt2(),
        "mincover" => mincover(),
        "transinv" => transinv(),
        "quad" => quad(),
        "openq" => openq(),
        other => {
            eprintln!("unknown command {other:?}; try: stats, coverage, bruteforce, partition, tptp, ladr, smt2, openq, mincover, transinv, quad");
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
    let mut corpus = linear_corpus(ll);
    let want3 = std::env::args().any(|a| a == "--order3-twists");
    let bases3 = if want3 {
        order3_canonical()
    } else {
        Vec::new()
    };
    add_twist_family(&mut corpus, &laws, &bases3);
    let corpus = corpus;
    let build_time = t.elapsed();

    let hard = parse_pairs(&data("hard_core.txt")).unwrap();
    let graph = ImplicationGraph::from_bytes(
        N_LAWS_ORDER4,
        parsimagma::etpdata::read_bytes("implications.bits"),
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
    // Compressed: the corpus signatures shrink 27x, and the file is a pure
    // output of this command.
    let mut sig = flate2::write::GzEncoder::new(
        std::io::BufWriter::new(
            std::fs::File::create(format!("{dir}/corpus_signatures.bin.gz")).unwrap(),
        ),
        flate2::Compression::best(),
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
    sig.finish().unwrap();
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
        parsimagma::etpdata::read_bytes("implications.bits"),
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

/// Partition the hard core by the smallest carrier that witnesses each pair.
///
/// Sprint S1. Three published numbers describe overlapping but distinct sets
/// and have never been reconciled: Janota's "only 310 of the undecided
/// implications require an infinite model", the ETP paper's 820 pairs whose
/// truth flips under finiteness, and the 814 pairs Janota's saturation runs
/// refuted without producing a witnessing model. This labels every pair in
/// the 1062-pair hard core and in the 814 with the smallest carrier the
/// corpus can witness it on, which bounds how much of each set can possibly
/// be infinite-only.
fn partition() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let ll: &'static LinearLaws = Box::leak(Box::new(LinearLaws::build(&laws)));
    let mut corpus = linear_corpus(ll);
    let bases3 = if std::env::args().any(|a| a == "--order3-twists") {
        order3_canonical()
    } else {
        Vec::new()
    };
    add_twist_family(&mut corpus, &laws, &bases3);

    let hard = parse_pairs(&data("hard_core.txt")).unwrap();
    let sat = parse_pairs(&data("saturation_refuted.txt")).unwrap();

    // Smallest witnessing carrier per pair: None = uncovered, Some(None) =
    // only an infinite carrier witnesses it, Some(Some(n)) = a carrier of n.
    let label = |targets: &[parsimagma::graph::Pair]| -> Vec<Option<Option<usize>>> {
        targets
            .iter()
            .map(|&p| {
                let mut best: Option<Option<usize>> = None;
                for inst in &corpus.instances {
                    if !parsimagma::graph::separates(&inst.sig, p) {
                        continue;
                    }
                    let here = match inst.carrier {
                        Carrier::Finite(n) => Some(n),
                        Carrier::Infinite => None,
                    };
                    best = Some(match (best, here) {
                        (None, h) => h,
                        (Some(None), h) => h,
                        (Some(Some(a)), Some(b)) => Some(a.min(b)),
                        (Some(Some(a)), None) => Some(a),
                    });
                }
                best
            })
            .collect()
    };

    let hl = label(&hard);
    let sl = label(&sat);

    let report =
        |name: &str, targets: &[parsimagma::graph::Pair], lab: &[Option<Option<usize>>]| {
            let mut hist: std::collections::BTreeMap<String, usize> = Default::default();
            for l in lab {
                let k = match l {
                    None => "uncovered".to_string(),
                    Some(None) => "infinite only".to_string(),
                    Some(Some(n)) => format!("finite {n}"),
                };
                *hist.entry(k).or_default() += 1;
            }
            let covered = lab.iter().filter(|l| l.is_some()).count();
            let finite = lab.iter().filter(|l| matches!(l, Some(Some(_)))).count();
            println!();
            println!("## {name}: {} pairs", targets.len());
            println!("  {covered} covered, of which {finite} by a finite carrier");
            for (k, v) in &hist {
                println!("    {k:<16} {v}");
            }
        };

    println!("# Hard-core partition (sprint S1)");
    report("hard core, Vampire-unresolved", &hard, &hl);
    report("saturation-refuted, no witnessing model", &sat, &sl);

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data/etp");
    use std::io::Write;
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(format!("{dir}/hard_core_partition.tsv")).unwrap(),
    );
    writeln!(f, "set\tfrom\tto\tsmallest_witness").unwrap();
    for (set, targets, lab) in [
        ("unresolved", &hard, &hl),
        ("saturation_refuted", &sat, &sl),
    ] {
        for (p, l) in targets.iter().zip(lab) {
            let w = match l {
                None => "none".to_string(),
                Some(None) => "infinite".to_string(),
                Some(Some(n)) => n.to_string(),
            };
            writeln!(f, "{set}\t{}\t{}\t{w}", p.from, p.to).unwrap();
        }
    }
    println!();
    println!("wrote data/etp/hard_core_partition.tsv");
}

/// Emit TPTP problems, one per ordered pair, in the CNF shape Janota used:
/// the hypothesis as an axiom over TPTP variables, the conclusion negated and
/// Skolemised to fresh constants.
///
/// Usage: `pm tptp <pairs-file> <out-dir>`
fn tptp() {
    use parsimagma::law::Term;
    use std::io::Write;

    let args: Vec<String> = std::env::args().collect();
    let pairs_file = args.get(2).expect("usage: pm tptp <pairs-file> <out-dir>");
    let out_dir = args.get(3).expect("usage: pm tptp <pairs-file> <out-dir>");
    std::fs::create_dir_all(out_dir).unwrap();

    let laws = parse_laws(&data("equations.txt")).unwrap();
    let text = std::fs::read_to_string(pairs_file).unwrap();
    let pairs = parse_pairs(&text).unwrap();

    // `m` is the magma operation, matching Janota's generated problems.
    fn render(t: &Term, skolem: bool) -> String {
        match t {
            Term::Var(v) => {
                if skolem {
                    format!("sk{v}")
                } else {
                    format!("X{v}")
                }
            }
            Term::Op(l, r) => format!("m({},{})", render(l, skolem), render(r, skolem)),
        }
    }

    let mut n = 0usize;
    for p in &pairs {
        let hyp = &laws[p.from as usize - 1];
        let concl = &laws[p.to as usize - 1];
        let path = format!("{out_dir}/{}_{}.p", p.from, p.to);
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "% E{} => E{}", p.from, p.to).unwrap();
        writeln!(
            f,
            "cnf(lhs, axiom, {} = {}).",
            render(&hyp.lhs, false),
            render(&hyp.rhs, false)
        )
        .unwrap();
        writeln!(
            f,
            "cnf(rhs, negated_conjecture, {} != {}).",
            render(&concl.lhs, true),
            render(&concl.rhs, true)
        )
        .unwrap();
        n += 1;
    }
    eprintln!("wrote {n} TPTP problems to {out_dir}");
}

/// Emit LADR problems for Mace4, the original MACE-style model finder, so the
/// domain-size cliff can be checked against an implementation independent of
/// Vampire's `fmb`.
///
/// LADR treats `x y z u v w` (with optional digits) as variables and every
/// other symbol as a constant, so Skolem constants are named `c0`, `c1`, ...
/// and law variables are `x0`, `x1`, ...
///
/// Usage: `pm ladr <pairs-file> <out-dir>`
fn ladr() {
    use parsimagma::law::Term;
    use std::io::Write;

    let args: Vec<String> = std::env::args().collect();
    let pairs_file = args.get(2).expect("usage: pm ladr <pairs-file> <out-dir>");
    let out_dir = args.get(3).expect("usage: pm ladr <pairs-file> <out-dir>");
    std::fs::create_dir_all(out_dir).unwrap();

    let laws = parse_laws(&data("equations.txt")).unwrap();
    let pairs = parse_pairs(&std::fs::read_to_string(pairs_file).unwrap()).unwrap();

    fn render(t: &Term, skolem: bool) -> String {
        match t {
            Term::Var(v) => {
                if skolem {
                    format!("c{v}")
                } else {
                    format!("x{v}")
                }
            }
            Term::Op(l, r) => format!("f({},{})", render(l, skolem), render(r, skolem)),
        }
    }

    let mut n = 0usize;
    for p in &pairs {
        let hyp = &laws[p.from as usize - 1];
        let concl = &laws[p.to as usize - 1];
        let mut f = std::fs::File::create(format!("{out_dir}/{}_{}.in", p.from, p.to)).unwrap();
        writeln!(
            f,
            "% E{} => E{}: a model here refutes the implication",
            p.from, p.to
        )
        .unwrap();
        writeln!(f, "formulas(assumptions).").unwrap();
        writeln!(
            f,
            "  {} = {}.",
            render(&hyp.lhs, false),
            render(&hyp.rhs, false)
        )
        .unwrap();
        writeln!(
            f,
            "  {} != {}.",
            render(&concl.lhs, true),
            render(&concl.rhs, true)
        )
        .unwrap();
        writeln!(f, "end_of_list.").unwrap();
        n += 1;
    }
    eprintln!("wrote {n} LADR problems to {out_dir}");
}

/// Scan the construction grid against the open order-5 questions.
///
/// Blueprint chapter 20 leaves three sets. Ten laws are proved Austin, which
/// serve as a control. Ninety-six have no nontrivial finite model and it is
/// open whether they admit an infinite one — the blueprint says plainly that
/// "no effort was made to build infinite models for these equations". Another
/// twenty-four are open even for finite models.
///
/// The linear tier answers both cheaply, and the two sets need opposite
/// instances. A linear magma over `Z` satisfying a law satisfies it over every
/// `Z/m` too, since the law is an identity in the coefficients and reduction
/// is a ring map — so any ordinary ring gives a nontrivial *finite* model.
/// That rules every ordinary ring out for the 96 and leaves only rings with no
/// suitable finite quotient, such as `Z<a,b>/(ba+1)` where `b` is a one-sided
/// inverse of `a`: in finite dimension a one-sided inverse is two-sided, so
/// that relation cannot survive. For the 24 the same fact runs the other way,
/// and any `Z/m` model settles the question outright.
///
/// The 10 Austin laws and the 96 must therefore have **no** `Z/m` hit. A hit
/// there would contradict an established ETP result and should be read as a
/// bug in this engine before anything else.
fn openq() {
    use parsimagma::linear::{AffineModel, LinearModel, RingOps};
    use parsimagma::rings::{
        FreeComm, FreeNc, Integers, MatFp, OneSidedInverse, PolyZ, WeylAlgebra, Zmod,
    };

    let all_laws = parse_laws(&data("eq_size5.txt")).unwrap();
    let mut targets: Vec<(String, u32)> = Vec::new();
    let mut laws: Vec<parsimagma::Law> = Vec::new();
    // Order-5 open questions are referenced by line number into eq_size5.txt.
    for line in data("order5_open.txt").lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut it = line.split('\t');
        let set = it.next().unwrap().to_string();
        let id: u32 = it.next().unwrap().parse().unwrap();
        targets.push((set, id));
        laws.push(all_laws[id as usize - 1].clone());
    }
    // The Higman-Neumann candidates are order-8 and carry their law text
    // inline, since eq_size5.txt only reaches order 5.
    for line in data("hn_open.txt").lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut it = line.split('\t');
        let set = it.next().unwrap().to_string();
        let id: u32 = it.next().unwrap().parse().unwrap();
        let text = it.next().unwrap();
        let parsed = parse_laws(text).unwrap();
        targets.push((set, id));
        laws.push(parsed.into_iter().next().unwrap());
    }
    let ll = LinearLaws::build(&laws);

    println!("# Open order-5 questions vs the construction grid");
    println!();
    for set in ["austin", "infinite", "finite", "hn"] {
        println!(
            "  {set:<10} {} laws",
            targets.iter().filter(|(s, _)| s == set).count()
        );
    }

    let mut hits: Vec<(usize, String)> = Vec::new();
    let record = |sig: &parsimagma::Signature, label: String, hits: &mut Vec<(usize, String)>| {
        for i in sig.iter_set() {
            hits.push((i, label.clone()));
        }
    };

    // Finite rings: these can only ever settle the 24.
    for m in 2u64..=96 {
        for a in 0..m {
            for b in 0..m {
                let s = LinearModel::new(Zmod { m }, a, b).signature(&ll);
                if s.count() > 0 {
                    record(&s, format!("Z/{m} linear a={a} b={b}"), &mut hits);
                }
            }
        }
    }
    for m in 2u64..=32 {
        for a in 0..m {
            for b in 0..m {
                for c in 1..m {
                    let s = AffineModel::new(Zmod { m }, a, b, c).signature(&ll);
                    if s.count() > 0 {
                        record(&s, format!("Z/{m} affine a={a} b={b} c={c}"), &mut hits);
                    }
                }
            }
        }
    }
    for p in [2u64, 3] {
        let r = MatFp { p, k: 2 };
        let n = (p as usize).pow(4);
        let mats: Vec<Vec<u64>> = (0..n)
            .map(|mut code| {
                (0..4)
                    .map(|_| {
                        let d = (code % p as usize) as u64;
                        code /= p as usize;
                        d
                    })
                    .collect()
            })
            .collect();
        for a in &mats {
            for b in &mats {
                let s = LinearModel::new(r.clone(), a.clone(), b.clone()).signature(&ll);
                if s.count() > 0 {
                    record(&s, format!("M_2(F_{p}) a={a:?} b={b:?}"), &mut hits);
                }
            }
        }
    }

    // Infinite rings: the only ones that can settle the 96.
    for a in -6i128..=6 {
        for b in -6i128..=6 {
            let s = LinearModel::new(Integers, a, b).signature(&ll);
            if s.count() > 0 {
                record(&s, format!("Z linear a={a} b={b}"), &mut hits);
            }
        }
    }
    for a0 in -2i128..=2 {
        for a1 in -2i128..=2 {
            for b0 in -2i128..=2 {
                for b1 in -2i128..=2 {
                    let s = LinearModel::new(PolyZ, PolyZ::lin(a0, a1), PolyZ::lin(b0, b1))
                        .signature(&ll);
                    if s.count() > 0 {
                        record(&s, format!("Z[t] a={a0}+{a1}t b={b0}+{b1}t"), &mut hits);
                    }
                }
            }
        }
    }
    let s = LinearModel::new(FreeComm, FreeComm.gen_a(), FreeComm.gen_b()).signature(&ll);
    record(&s, "Z[a,b] generic".to_string(), &mut hits);
    let s = LinearModel::new(FreeNc, FreeNc.gen_a(), FreeNc.gen_b()).signature(&ll);
    record(&s, "Z<a,b> generic".to_string(), &mut hits);
    let s = LinearModel::new(
        OneSidedInverse,
        OneSidedInverse.gen_a(),
        OneSidedInverse.gen_b(),
    )
    .signature(&ll);
    record(&s, "Z<a,b>/(ba+1) one-sided inverse".to_string(), &mut hits);
    let s = LinearModel::new(WeylAlgebra, WeylAlgebra.gen_a(), WeylAlgebra.gen_b()).signature(&ll);
    record(&s, "Weyl Z<a,b>/(ba-ab-1)".to_string(), &mut hits);
    // Also sweep small integer combinations of the generators, since the law
    // may need coefficients other than the generators themselves.
    for ca in -2i32..=2 {
        for cb in -2i32..=2 {
            for c1 in -2i32..=2 {
                if ca == 0 && cb == 0 && c1 == 0 {
                    continue;
                }
                let r = WeylAlgebra;
                let mut a = r.zero();
                r.scale_add_assign(&mut a, ca, &r.gen_a());
                r.scale_add_assign(&mut a, c1, &r.one());
                let mut b = r.zero();
                r.scale_add_assign(&mut b, cb, &r.gen_b());
                let s = LinearModel::new(WeylAlgebra, a, b).signature(&ll);
                if s.count() > 0 {
                    record(&s, format!("Weyl a={ca}x+{c1} b={cb}d"), &mut hits);
                }
            }
        }
    }

    // Twisted Cartesian powers. Their carriers are n^k — 16, 27, 32, 81 —
    // which is exactly the range the blueprint reports for order-5 models
    // ("a few had a minimum satisfying model size of order 17 ... one was
    // found with a satisfying model of order 26"), and exactly where the
    // control experiment shows finite model builders stop working.
    {
        use parsimagma::twist::TwistedPower;
        let bases2: Vec<FiniteMagma> = (0..16u32)
            .map(|bits| {
                FiniteMagma::new(2, (0..4).map(|i| ((bits >> i) & 1) as u8).collect()).unwrap()
            })
            .collect();
        let mut jobs: Vec<(FiniteMagma, usize, usize, usize)> = Vec::new();
        for b in &bases2 {
            for k in 2..=8usize {
                for sh in 0..k {
                    for t in 0..k {
                        jobs.push((b.clone(), k, sh, t));
                    }
                }
            }
        }
        for b in order3_canonical() {
            for k in 2..=3usize {
                for sh in 0..k {
                    for t in 0..k {
                        jobs.push((b.clone(), k, sh, t));
                    }
                }
            }
        }
        let found: Vec<(usize, String)> = jobs
            .par_iter()
            .flat_map(|(b, k, sh, t)| {
                let tw = TwistedPower::cyclic(b.clone(), *k, *sh, *t);
                let sig = tw.signature(&laws);
                sig.iter_set()
                    .map(|i| {
                        (
                            i,
                            format!(
                                "twist base{}#{:?} k={k} shifts=({sh},{t}) carrier={}",
                                b.n,
                                b.table,
                                tw.carrier_size()
                            ),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        eprintln!("twisted powers: {} instances scanned", jobs.len());
        hits.extend(found);
    }

    println!();
    if hits.is_empty() {
        println!(
            "no instance in the grid satisfies any of the {} target laws",
            laws.len()
        );
        return;
    }
    let mut by_law: std::collections::BTreeMap<usize, Vec<String>> = Default::default();
    for (i, lab) in hits {
        by_law.entry(i).or_default().push(lab);
    }
    println!("## Hits");
    for (i, labels) in &by_law {
        let (set, id) = &targets[*i];
        println!();
        println!("E{id}  [{set}]  {} instances", labels.len());
        println!("  law: {:?} = {:?}", laws[*i].lhs, laws[*i].rhs);
        let cap: usize = std::env::var("PM_SHOW")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6);
        for l in labels.iter().take(cap) {
            println!("    {l}");
        }
        if *set != "finite" {
            println!("  !! a hit outside the `finite` set contradicts an established ETP");
            println!("     result unless the witnessing ring has no nontrivial finite quotient");
        }
    }
}

/// Emit SMT-LIB2 problems that fix the carrier to a finite enumerated sort.
///
/// This is the apples-to-apples control the ATP experiment needs. A
/// MACE-style model finder builds its own propositional encoding with
/// symmetry breaking and its own search order; handing the *same* question to
/// an SMT solver over an explicit `n`-element datatype separates "this
/// problem is hard" from "that encoding and search are hard on this problem".
///
/// Usage: `pm smt2 <pairs-file> <out-dir> <carrier-size>`
fn smt2() {
    use parsimagma::law::Term;
    use std::io::Write;

    let args: Vec<String> = std::env::args().collect();
    let pairs_file = args
        .get(2)
        .expect("usage: pm smt2 <pairs-file> <out-dir> <n>");
    let out_dir = args
        .get(3)
        .expect("usage: pm smt2 <pairs-file> <out-dir> <n>");
    let n: usize = args
        .get(4)
        .expect("usage: pm smt2 <pairs-file> <out-dir> <n>")
        .parse()
        .unwrap();
    std::fs::create_dir_all(out_dir).unwrap();

    let laws = parse_laws(&data("equations.txt")).unwrap();
    let pairs = parse_pairs(&std::fs::read_to_string(pairs_file).unwrap()).unwrap();

    fn render(t: &Term, skolem: bool) -> String {
        match t {
            Term::Var(v) => {
                if skolem {
                    format!("sk{v}")
                } else {
                    format!("x{v}")
                }
            }
            Term::Op(l, r) => format!("(op {} {})", render(l, skolem), render(r, skolem)),
        }
    }

    for p in &pairs {
        let hyp = &laws[p.from as usize - 1];
        let concl = &laws[p.to as usize - 1];
        let mut f =
            std::fs::File::create(format!("{out_dir}/{}_{}_{n}.smt2", p.from, p.to)).unwrap();
        writeln!(
            f,
            "; E{} => E{} over a carrier of exactly {n}",
            p.from, p.to
        )
        .unwrap();
        writeln!(f, "(set-logic ALL)").unwrap();
        let elems: Vec<String> = (0..n).map(|i| format!("e{i}")).collect();
        writeln!(f, "(declare-datatypes ((M 0)) ((({}))))", elems.join(") (")).unwrap();
        writeln!(f, "(declare-fun op (M M) M)").unwrap();
        // Ground expansion rather than a quantifier: instantiate the
        // hypothesis at every tuple of carrier elements. This hands the
        // solver the constraint system directly, so a failure here cannot be
        // blamed on quantifier instantiation.
        let ground = std::env::var("PM_GROUND").is_ok();
        if ground {
            let k = hyp.arity as usize;
            let total = n.pow(k as u32);
            for code in 0..total {
                let mut c = code;
                let mut sub: Vec<String> = Vec::with_capacity(k);
                for _ in 0..k {
                    sub.push(format!("e{}", c % n));
                    c /= n;
                }
                let subst = |t: &Term| -> String {
                    fn go(t: &Term, sub: &[String]) -> String {
                        match t {
                            Term::Var(v) => sub[*v as usize].clone(),
                            Term::Op(l, r) => format!("(op {} {})", go(l, sub), go(r, sub)),
                        }
                    }
                    go(t, &sub)
                };
                writeln!(f, "(assert (= {} {}))", subst(&hyp.lhs), subst(&hyp.rhs)).unwrap();
            }
        } else {
            let vars: Vec<String> = (0..hyp.arity).map(|v| format!("(x{v} M)")).collect();
            writeln!(
                f,
                "(assert (forall ({}) (= {} {})))",
                vars.join(" "),
                render(&hyp.lhs, false),
                render(&hyp.rhs, false)
            )
            .unwrap();
        }
        for v in 0..concl.arity {
            writeln!(f, "(declare-const sk{v} M)").unwrap();
        }
        writeln!(
            f,
            "(assert (not (= {} {})))",
            render(&concl.lhs, true),
            render(&concl.rhs, true)
        )
        .unwrap();
        writeln!(f, "(check-sat)").unwrap();
    }
    eprintln!("wrote {} SMT-LIB2 problems at carrier {n}", pairs.len());
}

/// Reproduce the claim that a small subset of the brute-forced magmas refutes
/// everything all of them do.
///
/// The ETP paper says 524 distinct magmas suffice; Bruno Le Floch recomputed
/// 523 on Zulip in October 2025 and the repository README carries the working:
/// all 10 magmas of size 2 are covered by the size-4 models, and 291 of the
/// 299 magmas of size 3 are too, leaving 515 + 8. That figure has not been
/// independently checked, so it is checked here before being quoted.
///
/// Note this is a *sufficiency* claim, not minimality. Set cover is NP-hard;
/// nothing here computes a minimum.
fn mincover() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let e = Engine::new(Dag::build(&laws));

    let mut by_size: std::collections::BTreeMap<usize, Vec<parsimagma::Signature>> =
        Default::default();
    for f in [
        "refutations2x2.txt",
        "refutations3x3.txt",
        "refutations4x4.txt",
    ] {
        for entry in parse_refutations(&data(f)).unwrap() {
            let n = entry.magma.n;
            by_size
                .entry(n)
                .or_default()
                .push(e.signature(&entry.magma));
        }
    }
    println!("# Minimal covering subset of the brute-forced magmas");
    println!();
    for (n, v) in &by_size {
        println!("  size {n}: {} magmas", v.len());
    }

    let fold = |sigs: &[&parsimagma::Signature]| {
        let mut gc = GraphCoverage::new(N_LAWS_ORDER4);
        for s in sigs {
            gc.add(s);
        }
        gc
    };

    let all: Vec<&parsimagma::Signature> = by_size.values().flatten().collect();
    let full = fold(&all);
    println!();
    println!(
        "all {} magmas refute {} implications",
        all.len(),
        full.count()
    );

    // Everything the size-4 models alone reach.
    let four: Vec<&parsimagma::Signature> = by_size[&4].iter().collect();
    let base = fold(&four);
    println!(
        "the {} size-4 magmas alone refute {}",
        four.len(),
        base.count()
    );

    // Which smaller magmas add anything on top of them?
    let mut keep: Vec<(usize, usize)> = Vec::new();
    for (n, v) in &by_size {
        if *n == 4 {
            continue;
        }
        let mut redundant = 0usize;
        for (i, s) in v.iter().enumerate() {
            if base.adds_anything(s) {
                keep.push((*n, i));
            } else {
                redundant += 1;
            }
        }
        println!(
            "  {redundant}/{} magmas of size {n} are already covered by the size-4 models",
            v.len()
        );
    }

    let mut subset: Vec<&parsimagma::Signature> = four.clone();
    for (n, i) in &keep {
        subset.push(&by_size[n][*i]);
    }
    let covered = fold(&subset);
    println!();
    println!(
        "subset of {} magmas ({} of size 4 plus {} smaller) refutes {}",
        subset.len(),
        four.len(),
        keep.len(),
        covered.count()
    );
    if covered.count() == full.count() {
        println!(
            "=> {} distinct magmas suffice for every refutation the full set makes",
            subset.len()
        );
    } else {
        println!(
            "=> INSUFFICIENT: {} pairs short of the full set",
            full.count() - covered.count()
        );
    }
}

/// Sweep translation-invariant magmas `x ◇ y = x + f(y - x)` over `Z/n`,
/// exhaustively in `f`, against the hard core.
///
/// Aimed at the 296-pair cluster of `docs/cluster-296.md`, and at the carrier
/// range the domain-size cliff leaves unsearched. The grid is every function
/// `f: Z/n -> Z/n`, which is `n^n` and therefore stated exactly rather than
/// sampled.
///
/// Only the laws appearing in hard-core pairs are evaluated, not all 4694: a
/// sweep of hundreds of millions of candidates cannot afford a full signature
/// each, and every question here is about those laws alone.
///
/// Usage: `pm transinv [max_n]`
fn transinv() {
    use parsimagma::finite::Scratch;
    use parsimagma::transinv::TranslationInvariant;
    use rayon::prelude::*;

    let max_n: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    // `perm` restricts f to permutations, which is the only way this family
    // reaches carrier 11 and above.
    let perms_only = std::env::args().any(|a| a == "perm");
    let min_n: usize = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    let all_laws = parse_laws(&data("equations.txt")).unwrap();
    let hard = parse_pairs(&data("hard_core.txt")).unwrap();

    // The sub-problem: only laws that appear in a hard-core pair.
    let mut ids: Vec<u32> = hard.iter().flat_map(|p| [p.from, p.to]).collect();
    ids.sort_unstable();
    ids.dedup();
    let index: std::collections::HashMap<u32, usize> =
        ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let laws: Vec<parsimagma::Law> = ids
        .iter()
        .map(|id| all_laws[*id as usize - 1].clone())
        .collect();
    let e = Engine::new(Dag::build(&laws));
    let pairs: Vec<(usize, usize)> = hard
        .iter()
        .map(|p| (index[&p.from], index[&p.to]))
        .collect();

    println!("# Translation-invariant magmas vs the hard core");
    println!();
    if perms_only {
        println!(
            "grid       x ◇ y = x + f(y - x) over Z/n, every *permutation* f, n = {min_n}..{max_n}"
        );
    } else {
        println!("grid       x ◇ y = x + f(y - x) over Z/n, every f, n = {min_n}..{max_n}");
    }
    println!(
        "evaluating {} laws (those appearing in the {} hard-core pairs)",
        laws.len(),
        hard.len()
    );
    println!();

    let mut hits: Vec<(usize, Vec<u8>, Vec<usize>)> = Vec::new();
    for n in min_n..=max_n {
        let total = if perms_only {
            parsimagma::transinv::factorial(n)
        } else {
            TranslationInvariant::grid_size(n)
        };
        let t = Instant::now();
        let chunk = 1u64 << 14;
        let nchunks = total.div_ceil(chunk);
        let found: Vec<(usize, Vec<u8>, Vec<usize>)> = (0..nchunks)
            .into_par_iter()
            .fold(
                || {
                    (
                        Vec::new(),
                        Scratch::new(&e.dag),
                        Vec::with_capacity(n),
                        vec![0u8; n * n],
                    )
                },
                |(mut acc, mut scratch, mut f, mut table), c| {
                    let lo = c * chunk;
                    let hi = (lo + chunk).min(total);
                    for code in lo..hi {
                        if perms_only {
                            parsimagma::transinv::permutation(n, code, &mut f);
                        } else {
                            let mut v = code;
                            f.clear();
                            for _ in 0..n {
                                f.push((v % n as u64) as u8);
                                v /= n as u64;
                            }
                        }
                        let ti = TranslationInvariant::new(n, f.clone());
                        ti.fill(&mut table);
                        let m = FiniteMagma::new(n, table.clone()).unwrap();
                        let sig = e.signature_with(&m, &mut scratch);
                        let got: Vec<usize> = pairs
                            .iter()
                            .enumerate()
                            .filter(|(_, (a, b))| sig.get(*a) && !sig.get(*b))
                            .map(|(k, _)| k)
                            .collect();
                        if !got.is_empty() {
                            acc.push((n, f.clone(), got));
                        }
                    }
                    (acc, scratch, f, table)
                },
            )
            .map(|(acc, _, _, _)| acc)
            .reduce(Vec::new, |mut a, b| {
                a.extend(b);
                a
            });
        let mut reached: std::collections::BTreeSet<usize> = Default::default();
        for (_, _, g) in &found {
            reached.extend(g.iter().copied());
        }
        println!(
            "n={n:<3} {total:>12} candidates  {:>8} discharge something  {:>4} distinct pairs  {:.2?}",
            found.len(),
            reached.len(),
            t.elapsed()
        );
        hits.extend(found);
    }

    let mut reached: std::collections::BTreeSet<usize> = Default::default();
    for (_, _, g) in &hits {
        reached.extend(g.iter().copied());
    }
    println!();
    println!(
        "{} of {} hard-core pairs reached by this family",
        reached.len(),
        hard.len()
    );

    // What is new relative to the labelled partition?
    let part = data("hard_core_partition.tsv");
    let uncovered: std::collections::HashSet<(u32, u32)> = part
        .lines()
        .skip(1)
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c.len() == 4 && c[0] == "unresolved" && c[3] == "none")
                .then(|| (c[1].parse().unwrap(), c[2].parse().unwrap()))
        })
        .collect();
    let fresh: Vec<usize> = reached
        .iter()
        .copied()
        .filter(|k| uncovered.contains(&(hard[*k].from, hard[*k].to)))
        .collect();
    println!(
        "{} of them were not reached by any earlier family",
        fresh.len()
    );
    if !hits.is_empty() {
        println!();
        println!("witnessing functions (is f linear, f(d) = b*d?):");
        let mut shown = 0;
        for (n, f, g) in &hits {
            // f is linear iff f(d) = f(1)*d for all d.
            let b = f[1 % *n] as usize;
            let linear = (0..*n).all(|d| f[d] as usize == (b * d) % *n);
            println!(
                "    n={n:<3} f={f:?}  {}  discharges {} pair(s)",
                if linear {
                    format!("LINEAR b={b}")
                } else {
                    "nonlinear".to_string()
                },
                g.len()
            );
            shown += 1;
            if shown >= 25 {
                break;
            }
        }
    }
    for k in fresh.iter().take(30) {
        let p = hard[*k];
        let best = hits
            .iter()
            .find(|(_, _, g)| g.contains(k))
            .map(|(n, f, _)| format!("n={n} f={f:?}"))
            .unwrap_or_default();
        println!("    E{} !=> E{}   {best}", p.from, p.to);
    }
}

/// Sweep quadratic magmas `x ◇ y = ax² + bxy + cy² + dx + ey + f` over `Z/N`
/// against the hard core (ETP paper Remark 5.5).
///
/// The paper found these "somewhat useful" for additional finite refutations
/// at small `N` and expects them to thin out as `N` grows. The reason to run
/// them anyway is the domain-size cliff: carrier 11 upward is where no
/// search-based model finder reaches, so if this family has anything there,
/// nobody has looked.
///
/// Usage: `pm quad [max_n] [min_n]`
fn quad() {
    use parsimagma::finite::Scratch;
    use parsimagma::quadratic::Quadratic;
    use rayon::prelude::*;

    let max_n: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);
    let min_n: u32 = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    let all_laws = parse_laws(&data("equations.txt")).unwrap();
    let hard = parse_pairs(&data("hard_core.txt")).unwrap();
    let mut ids: Vec<u32> = hard.iter().flat_map(|p| [p.from, p.to]).collect();
    ids.sort_unstable();
    ids.dedup();
    let index: std::collections::HashMap<u32, usize> =
        ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let laws: Vec<parsimagma::Law> = ids
        .iter()
        .map(|id| all_laws[*id as usize - 1].clone())
        .collect();
    let e = Engine::new(Dag::build(&laws));
    let pairs: Vec<(usize, usize)> = hard
        .iter()
        .map(|p| (index[&p.from], index[&p.to]))
        .collect();

    let part = data("hard_core_partition.tsv");
    let uncovered: std::collections::HashSet<(u32, u32)> = part
        .lines()
        .skip(1)
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c.len() == 4 && c[0] == "unresolved" && c[3] == "none")
                .then(|| (c[1].parse().unwrap(), c[2].parse().unwrap()))
        })
        .collect();

    println!("# Quadratic magmas vs the hard core");
    println!();
    println!("grid       x ◇ y = ax² + bxy + cy² + dx + ey + f over Z/N, all coefficients, N = {min_n}..{max_n}");
    println!(
        "evaluating {} laws (those in the {} hard-core pairs)",
        laws.len(),
        hard.len()
    );
    println!();

    let mut all_reached: std::collections::BTreeSet<usize> = Default::default();
    let mut fresh_hits: Vec<(u32, Quadratic, Vec<usize>)> = Vec::new();
    for n in min_n..=max_n {
        let total = Quadratic::grid_size(n);
        let t = Instant::now();
        let chunk = 1u64 << 14;
        let found: Vec<(Quadratic, Vec<usize>)> = (0..total.div_ceil(chunk))
            .into_par_iter()
            .fold(
                || {
                    (
                        Vec::new(),
                        Scratch::new(&e.dag),
                        vec![0u8; (n * n) as usize],
                    )
                },
                |(mut acc, mut scratch, mut table), c| {
                    let lo = c * chunk;
                    let hi = (lo + chunk).min(total);
                    for code in lo..hi {
                        let q = Quadratic::from_code(n, code);
                        // Linear instances are already swept exhaustively.
                        if q.is_linear() {
                            continue;
                        }
                        q.fill(&mut table);
                        let m = FiniteMagma::new(n as usize, table.clone()).unwrap();
                        let sig = e.signature_with(&m, &mut scratch);
                        let got: Vec<usize> = pairs
                            .iter()
                            .enumerate()
                            .filter(|(_, (a, b))| sig.get(*a) && !sig.get(*b))
                            .map(|(k, _)| k)
                            .collect();
                        if !got.is_empty() {
                            acc.push((q, got));
                        }
                    }
                    (acc, scratch, table)
                },
            )
            .map(|(acc, _, _)| acc)
            .reduce(Vec::new, |mut a, b| {
                a.extend(b);
                a
            });
        let mut reached: std::collections::BTreeSet<usize> = Default::default();
        for (_, g) in &found {
            reached.extend(g.iter().copied());
        }
        let new: Vec<usize> = reached
            .iter()
            .copied()
            .filter(|k| uncovered.contains(&(hard[*k].from, hard[*k].to)))
            .collect();
        println!(
            "N={n:<3} {total:>12} instances  {:>7} hit  {:>4} pairs  {:>3} NEW  {:.2?}",
            found.len(),
            reached.len(),
            new.len(),
            t.elapsed()
        );
        for (q, g) in &found {
            if g.iter()
                .any(|k| uncovered.contains(&(hard[*k].from, hard[*k].to)))
            {
                fresh_hits.push((n, *q, g.clone()));
            }
        }
        all_reached.extend(reached);
    }

    let new: Vec<usize> = all_reached
        .iter()
        .copied()
        .filter(|k| uncovered.contains(&(hard[*k].from, hard[*k].to)))
        .collect();
    println!();
    println!(
        "{} of {} hard-core pairs reached",
        all_reached.len(),
        hard.len()
    );
    println!("{} of them reached by no earlier family", new.len());
    for (n, q, g) in fresh_hits.iter().take(20) {
        let fresh: Vec<String> = g
            .iter()
            .filter(|k| uncovered.contains(&(hard[**k].from, hard[**k].to)))
            .map(|k| format!("E{}!=>E{}", hard[*k].from, hard[*k].to))
            .collect();
        println!(
            "    N={n} (a,b,c,d,e,f)=({},{},{},{},{},{})  {}",
            q.a,
            q.b,
            q.c,
            q.d,
            q.e,
            q.f,
            fresh.join(" ")
        );
    }
}
