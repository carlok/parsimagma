//! Translation-invariant magmas, checked against the definition and against
//! the constructions they generalise.

use parsimagma::transinv::TranslationInvariant;
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma};

fn data(name: &str) -> String {
    parsimagma::etpdata::read_text(name)
}

#[test]
fn table_matches_the_defining_formula() {
    for n in 2usize..=6 {
        // A deterministic spread of functions f.
        for seed in 0..12u64 {
            let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
            let f: Vec<u8> = (0..n)
                .map(|_| {
                    s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ((s >> 33) as usize % n) as u8
                })
                .collect();
            let ti = TranslationInvariant::new(n, f.clone());
            let m = ti.magma();
            for x in 0..n {
                for y in 0..n {
                    let d = (y + n - x) % n;
                    assert_eq!(m.op(x as u8, y as u8) as usize, (x + f[d] as usize) % n);
                }
            }
            // Left translation by t is an automorphism, which is the defining
            // property of the family.
            for t in 0..n {
                for x in 0..n {
                    for y in 0..n {
                        let lx = (x + t) % n;
                        let ly = (y + t) % n;
                        assert_eq!(
                            m.op(lx as u8, ly as u8) as usize,
                            (m.op(x as u8, y as u8) as usize + t) % n,
                            "left translation by {t} is not an automorphism"
                        );
                    }
                }
            }
        }
    }
}

/// A linear magma `x ◇ y = ax + by` with `a + b = 1` is translation-invariant
/// with `f(d) = b·d`, so the two families must agree there. This ties the new
/// code to the already-validated linear path.
#[test]
fn linear_models_with_a_plus_b_one_are_translation_invariant() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let e = Engine::new(Dag::build(&laws));
    let mut checked = 0usize;
    for n in 2usize..=7 {
        for b in 0..n {
            let a = (n + 1 - b % n) % n; // a = 1 - b mod n
            let linear: Vec<u8> = (0..n * n)
                .map(|k| (((a * (k / n)) + (b * (k % n))) % n) as u8)
                .collect();
            let lm = FiniteMagma::new(n, linear).unwrap();
            let ti = TranslationInvariant::new(n, (0..n).map(|d| ((b * d) % n) as u8).collect());
            assert_eq!(lm, ti.magma(), "n={n} a={a} b={b}");
            assert_eq!(e.signature(&lm), e.signature(&ti.magma()));
            checked += 1;
        }
    }
    assert!(checked >= 20);
}

#[test]
fn grid_size_is_n_to_the_n() {
    assert_eq!(TranslationInvariant::grid_size(2), 4);
    assert_eq!(TranslationInvariant::grid_size(8), 16_777_216);
    assert_eq!(TranslationInvariant::grid_size(9), 387_420_489);
}

#[test]
fn permutation_decoding_is_a_bijection() {
    use parsimagma::transinv::{factorial, permutation};
    for n in 1usize..=7 {
        let total = factorial(n);
        let mut seen = std::collections::HashSet::new();
        let mut buf = Vec::new();
        for i in 0..total {
            permutation(n, i, &mut buf);
            assert_eq!(buf.len(), n);
            let mut sorted = buf.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..n as u8).collect::<Vec<_>>(), "not a permutation");
            assert!(seen.insert(buf.clone()), "duplicate at index {i}");
        }
        assert_eq!(seen.len() as u64, total);
    }
}
