//! Differential tests against the ETP's own published data.
//!
//! Anything below 100% agreement is a blocking bug, not a curiosity. The
//! headline corpus is the 824 magmas of `All4x4Tables`, each stored with the
//! complete set of laws it satisfies, so agreement is checked on all 4694
//! bits rather than on a single one.

use parsimagma::etpdata::{parse_refutations, parse_smallest_examples, parse_smallest_sizes};
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma, Signature, N_LAWS_ORDER4};
use std::collections::BTreeSet;
use std::sync::OnceLock;

fn data(name: &str) -> String {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/data/etp/");
    std::fs::read_to_string(format!("{p}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(|| {
        let laws = parse_laws(&data("equations.txt")).expect("law list parses");
        Engine::new(Dag::build(&laws))
    })
}

#[test]
fn law_list_matches_etp_shape() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    assert_eq!(laws.len(), N_LAWS_ORDER4);

    // Ids are the 1-based line numbers.
    for (i, l) in laws.iter().enumerate() {
        assert_eq!(l.id as usize, i + 1);
    }

    // Paper section 1.2: laws of order at most 4, i.e. at most four
    // operations across both sides.
    let mut by_ops = [0usize; 5];
    let mut by_arity = [0usize; 7];
    for l in &laws {
        let ops = l.lhs.ops() + l.rhs.ops();
        assert!(ops <= 4, "E{} has {ops} operations", l.id);
        by_ops[ops] += 1;
        by_arity[l.arity as usize] += 1;
        // Canonical variable indices are contiguous from 0.
        let m = l.lhs.max_var().max(l.rhs.max_var());
        assert_eq!(m + 1, l.arity, "E{} arity mismatch", l.id);
    }
    assert_eq!(by_ops, [2, 5, 39, 364, 4284]);
    assert_eq!(by_arity, [0, 31, 779, 2090, 1447, 325, 22]);

    // Spot-check the first three laws and the last, against equations.txt.
    assert_eq!(format!("{:?}", laws[0].lhs), "a");
    assert_eq!(format!("{:?}", laws[0].rhs), "a");
    assert_eq!(format!("{:?}", laws[1].rhs), "b");
    assert_eq!(format!("{:?}", laws[2].rhs), "(a \u{25c7} a)");
    assert_eq!(
        format!("{:?} = {:?}", laws[4693].lhs, laws[4693].rhs),
        "((a \u{25c7} b) \u{25c7} c) = ((d \u{25c7} e) \u{25c7} f)"
    );
}

#[test]
fn dag_shares_subterms() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let dag = Dag::build(&laws);
    let independent: usize = laws.iter().map(|l| l.lhs.ops() + l.rhs.ops()).sum();
    // The whole point of the DAG: distinct subterms number in the low
    // thousands, well under the ~18k applications the laws contain in total.
    assert!(
        dag.n_op_nodes() < independent / 4,
        "{} shared nodes vs {} independent applications",
        dag.n_op_nodes(),
        independent
    );
}

/// The headline test: every magma in `All4x4Tables` must produce exactly the
/// law set the ETP recorded for it.
#[test]
fn signatures_match_all4x4tables_exactly() {
    let e = engine();
    let mut checked = 0usize;
    let mut agreed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for file in [
        "refutations2x2.txt",
        "refutations3x3.txt",
        "refutations4x4.txt",
    ] {
        let entries = parse_refutations(&data(file)).unwrap();
        assert!(!entries.is_empty(), "{file} is empty");
        for (k, entry) in entries.iter().enumerate() {
            checked += 1;
            let ours: BTreeSet<u32> = e
                .signature(&entry.magma)
                .satisfied_ids()
                .into_iter()
                .collect();
            let theirs: BTreeSet<u32> = entry.proves.iter().copied().collect();
            if ours == theirs {
                agreed += 1;
            } else if failures.len() < 5 {
                let extra: Vec<_> = ours.difference(&theirs).take(8).collect();
                let missing: Vec<_> = theirs.difference(&ours).take(8).collect();
                failures.push(format!(
                    "{file}#{k} n={} we-say-yes-they-say-no={extra:?} we-say-no-they-say-yes={missing:?}",
                    entry.magma.n
                ));
            }
        }
    }

    assert_eq!(checked, 824, "expected the 10 + 299 + 515 magma corpus");
    assert_eq!(
        agreed,
        checked,
        "agreement {agreed}/{checked}; first failures:\n{}",
        failures.join("\n")
    );
}

/// Every published smallest model must in fact satisfy its law, and its
/// carrier size must be the one recorded.
#[test]
fn smallest_models_satisfy_their_law() {
    let e = engine();
    let examples = parse_smallest_examples(&data("smallest_magma_examples.txt")).unwrap();
    let sizes: std::collections::BTreeMap<u32, usize> =
        parse_smallest_sizes(&data("smallest_magma.txt"))
            .unwrap()
            .into_iter()
            .collect();

    assert_eq!(
        examples.len(),
        3198,
        "4694 laws minus the 1496 equivalent to E2"
    );
    let mut by_size = std::collections::BTreeMap::new();
    for (id, m) in &examples {
        assert!(m.n >= 2, "E{id} example has trivial carrier");
        assert_eq!(sizes.get(id), Some(&m.n), "E{id} size disagrees");
        *by_size.entry(m.n).or_insert(0usize) += 1;
        let sig = e.signature(m);
        assert!(
            sig.get(*id as usize - 1),
            "the published smallest model of E{id} does not satisfy E{id}"
        );
    }
    // Paper Table 1.
    assert_eq!(
        by_size.into_iter().collect::<Vec<_>>(),
        vec![(2, 3136), (3, 32), (4, 14), (5, 14), (7, 2)]
    );
}

#[test]
fn e1_always_holds_and_e2_only_for_singletons() {
    let e = engine();
    // A deterministic spread of magmas: all 2-element tables, plus a
    // scattering of 3- and 4-element ones from a fixed LCG.
    let mut magmas = Vec::new();
    for bits in 0u32..16 {
        let table: Vec<u8> = (0..4).map(|i| ((bits >> i) & 1) as u8).collect();
        magmas.push(FiniteMagma::new(2, table).unwrap());
    }
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for n in [1usize, 3, 4, 5] {
        for _ in 0..40 {
            let table: Vec<u8> = (0..n * n)
                .map(|_| {
                    state = state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    ((state >> 33) as usize % n) as u8
                })
                .collect();
            magmas.push(FiniteMagma::new(n, table).unwrap());
        }
    }

    for m in &magmas {
        let sig = e.signature(m);
        assert!(sig.get(0), "E1 (x = x) must hold in every magma");
        assert_eq!(
            sig.get(1),
            m.n == 1,
            "E2 (x = y) holds exactly in singletons, n={}",
            m.n
        );
    }
}

#[test]
fn signature_bytes_round_trip() {
    let mut s = Signature::zeros(N_LAWS_ORDER4);
    for i in [0usize, 1, 63, 64, 100, 4693] {
        s.set(i);
    }
    let bytes = s.to_bytes();
    assert_eq!(bytes.len(), 587, "4694 bits pack into 587 bytes");
    assert_eq!(Signature::from_bytes(N_LAWS_ORDER4, &bytes), s);
    assert_eq!(s.count(), 6);
    assert_eq!(s.separations(), 6 * (4694 - 6));
}

/// Reconstructing anti-implications from signatures and checking them
/// against the published graph. Every separation a magma claims must be a
/// pair the ETP records as false; a claim against a true implication would
/// mean the engine is wrong somewhere, and there is no benign reading of it.
#[test]
fn no_signature_contradicts_the_published_implication_graph() {
    use parsimagma::graph::ImplicationGraph;
    let e = engine();
    let bits = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/etp/implications.bits"
    ))
    .unwrap();
    let g = ImplicationGraph::from_bytes(N_LAWS_ORDER4, bits);

    // Janota's run: 8,173,585 non-reflexive implications hold, plus the 4694
    // reflexive ones this matrix also records.
    assert_eq!(g.count_true(), 8_173_585 + 4694);

    let mut magmas: Vec<FiniteMagma> = Vec::new();
    for f in [
        "refutations2x2.txt",
        "refutations3x3.txt",
        "refutations4x4.txt",
    ] {
        magmas.extend(
            parse_refutations(&data(f))
                .unwrap()
                .into_iter()
                .map(|r| r.magma),
        );
    }
    magmas.extend(
        parse_smallest_examples(&data("smallest_magma_examples.txt"))
            .unwrap()
            .into_iter()
            .map(|(_, m)| m),
    );

    let mut separations: u64 = 0;
    for m in &magmas {
        let sig = e.signature(m);
        // Word-parallel: `sat & !implied_by_i` must be empty for every
        // satisfied law i, since anything M satisfies must entail anything
        // else M satisfies.
        for i in sig.iter_set() {
            let row = g.row(i);
            for j in 0..N_LAWS_ORDER4 {
                if row.get(j) && !sig.get(j) {
                    panic!(
                        "magma of size {} satisfies E{} and refutes E{}, \
                         but the ETP graph says E{} implies E{}",
                        m.n,
                        i + 1,
                        j + 1,
                        i + 1,
                        j + 1
                    );
                }
            }
            separations += (N_LAWS_ORDER4 - sig.count() as usize) as u64;
        }
    }
    assert!(
        separations > 1_000_000_000,
        "expected the corpus to witness billions of separations, got {separations}"
    );
}
