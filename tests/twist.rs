//! Twisted Cartesian powers, validated against the paper's worked example and
//! against the table route wherever the carrier is small enough to have one.

use parsimagma::twist::{nand_f2, TwistedPower};
use parsimagma::{parse_laws, Dag, Engine, Law};
use std::sync::OnceLock;

fn laws() -> &'static Vec<Law> {
    static L: OnceLock<Vec<Law>> = OnceLock::new();
    L.get_or_init(|| {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/data/etp/equations.txt");
        parse_laws(&std::fs::read_to_string(p).unwrap()).unwrap()
    })
}

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(|| Engine::new(Dag::build(laws())))
}

/// Paper section 5.4, Example 5.11 and the paragraph after it. `Twist_{E1485}`
/// is cyclic of order 5 and `Twist_{E151}` is cyclic of order 2, so twisting
/// `F_2^5` under NAND by the left and right shifts keeps E1485 and breaks
/// E151. The paper adds that this implication "does not seem to be easily
/// refuted by any of the other methods discussed", which is borne out: no
/// linear or affine instance in the grid refutes it.
#[test]
fn reproduces_e1485_not_implies_e151() {
    let l = laws();
    // sigma(i) = i + 1, tau(i) = i - 1, indices modulo 5.
    let t = TwistedPower::cyclic(nand_f2(), 5, 1, 4);
    assert_eq!(t.carrier_size(), 32);
    assert!(t.satisfies(&l[1485 - 1]), "the twist must preserve E1485");
    assert!(!t.satisfies(&l[151 - 1]), "and must break E151");

    // The untwisted base is a model of E1485 too, and of E151, which is why
    // the twist is doing the work rather than the base.
    let base = TwistedPower::cyclic(nand_f2(), 1, 0, 0);
    assert!(base.satisfies(&l[1485 - 1]));
    assert!(base.satisfies(&l[151 - 1]));
}

/// A power of one coordinate with identity shifts is the base magma itself,
/// so its signature must be exactly what the finite engine computes.
#[test]
fn trivial_twist_reproduces_the_base_signature() {
    let l = laws();
    for table in [vec![1u8, 1, 1, 0], vec![0, 1, 1, 0], vec![0, 0, 1, 1]] {
        let base = parsimagma::FiniteMagma::new(2, table).unwrap();
        let t = TwistedPower::cyclic(base.clone(), 1, 0, 0);
        assert_eq!(t.signature(l), engine().signature(&base));
    }
}

/// Where the twisted carrier is small enough for a table, the coordinate-wise
/// check and the exhaustive sweep must agree on all 4694 bits.
#[test]
fn coordinate_check_agrees_with_the_table_sweep() {
    let l = laws();
    let mut checked = 0usize;
    for k in 2..=4usize {
        for s in 0..k {
            for t in 0..k {
                let tw = TwistedPower::cyclic(nand_f2(), k, s, t);
                let magma = tw.table().expect("carrier of at most 16 elements");
                assert_eq!(magma.n, 1 << k);
                assert_eq!(
                    tw.signature(l),
                    engine().signature(&magma),
                    "k={k} s={s} t={t}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 4 + 9 + 16);
}
