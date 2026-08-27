//! Tier S validation: linear models decided symbolically.
//!
//! Two kinds of check. First, agreement with the finite engine on every
//! instance whose carrier is small enough to sweep — the symbolic route and
//! the table route must produce identical signatures, bit for bit. Second,
//! agreement with results the ETP attributes to this family by name.

use parsimagma::linear::{LinearLaws, LinearModel, RingOps};
use parsimagma::rings::{FreeComm, FreeNc, Integers, MatFp, PolyZ, Zmod};
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma, Signature};
use std::sync::OnceLock;

fn data(name: &str) -> String {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/data/etp/");
    std::fs::read_to_string(format!("{p}{name}")).unwrap()
}

struct Ctx {
    engine: Engine,
    ll: LinearLaws,
}

fn ctx() -> &'static Ctx {
    static C: OnceLock<Ctx> = OnceLock::new();
    C.get_or_init(|| {
        let laws = parse_laws(&data("equations.txt")).unwrap();
        Ctx {
            ll: LinearLaws::build(&laws),
            engine: Engine::new(Dag::build(&laws)),
        }
    })
}

fn zmod_sig(m: u64, a: u64, b: u64) -> Signature {
    LinearModel::new(Zmod { m }, a % m, b % m).signature(&ctx().ll)
}

fn int_sig(a: i128, b: i128) -> Signature {
    LinearModel::new(Integers, a, b).signature(&ctx().ll)
}

/// The load-bearing test for the whole tier: an instance over `Z/m` is both a
/// symbolic object and a finite magma, and the two routes must agree.
#[test]
fn symbolic_and_table_routes_agree_on_finite_linear_magmas() {
    let c = ctx();
    let mut instances = 0usize;
    for m in 2u64..=5 {
        for a in 0..m {
            for b in 0..m {
                let model = LinearModel::new(Zmod { m }, a, b);
                let symbolic = model.signature(&c.ll);

                let elements: Vec<u64> = (0..m).collect();
                let table = model.table(&elements);
                let magma = FiniteMagma::new(m as usize, table.clone()).unwrap();
                // Sanity: the table really is x ↦ ax + by.
                for x in 0..m {
                    for y in 0..m {
                        assert_eq!(
                            magma.op(x as u8, y as u8) as u64,
                            (a * x + b * y) % m,
                            "Z/{m} a={a} b={b} table entry ({x},{y})"
                        );
                    }
                }
                let swept = c.engine.signature(&magma);

                assert_eq!(
                    symbolic,
                    swept,
                    "Z/{m} with (a,b)=({a},{b}): symbolic says {} laws, sweep says {}",
                    symbolic.count(),
                    swept.count()
                );
                instances += 1;
            }
        }
    }
    assert_eq!(instances, 4 + 9 + 16 + 25);
}

/// Section 9 of the paper: the linear operations with `a, b ∈ {-1, 0, 1}`
/// over `Z/n` account for 3068 of the laws with full spectrum. A law holds
/// over `Z/n` for every `n` exactly when its difference polynomials vanish
/// over `Z`, so this is a check against a published count.
#[test]
fn small_integer_coefficients_reproduce_the_3068_full_spectrum_laws() {
    let c = ctx();
    let mut union = Signature::zeros(c.ll.n_laws());
    for a in [-1i128, 0, 1] {
        for b in [-1i128, 0, 1] {
            let s = int_sig(a, b);
            for i in s.iter_set() {
                union.set(i);
            }
        }
    }
    assert_eq!(
        union.count(),
        3068,
        "paper section 9 attributes 3068 laws to a,b in {{-1,0,1}}"
    );
}

/// Each coefficient pair the paper names, and the law it names it for.
#[test]
fn named_linear_models_satisfy_their_named_laws() {
    // Paper section 9, bullet by bullet.
    let e = |id: u32| id as usize - 1;

    // (0,0): the constant operation, a model of E46 (x ◇ y = z ◇ w).
    assert!(int_sig(0, 0).get(e(46)));
    // (1,0): left projection, a model of E4 (x = x ◇ y).
    assert!(int_sig(1, 0).get(e(4)));
    // (0,1): right projection, a model of E5 (x = y ◇ x).
    assert!(int_sig(0, 1).get(e(5)));
    // (1,-1): abelian group subtraction, characterised by Tarski's E543.
    assert!(int_sig(1, -1).get(e(543)));
    // (-1,1): backwards subtraction, a model of E1090, the dual of E543.
    assert!(int_sig(-1, 1).get(e(1090)));
    // (-1,-1): a model of the semi-symmetric quasigroup law E14 and the
    // totally symmetric quasigroup law E492.
    assert!(int_sig(-1, -1).get(e(14)));
    assert!(int_sig(-1, -1).get(e(492)));

    // Projections are not subtraction: (1,0) must refute E543.
    assert!(!int_sig(1, 0).get(e(543)));
    // Subtraction is not a projection.
    assert!(!int_sig(1, -1).get(e(4)));
}

/// E1 holds in everything; E2 holds only in the zero ring.
#[test]
fn trivial_laws_behave() {
    for (m, a, b) in [(2u64, 0u64, 0u64), (3, 1, 2), (5, 4, 4), (4, 2, 3)] {
        let s = zmod_sig(m, a, b);
        assert!(s.get(0), "E1 must hold");
        assert!(!s.get(1), "E2 must fail off the singleton");
    }
    let zero_ring = LinearModel::new(Zmod { m: 1 }, 0, 0).signature(&ctx().ll);
    assert!(zero_ring.get(1), "E2 holds in the one-element magma");
    assert_eq!(
        zero_ring.count() as usize,
        ctx().ll.n_laws(),
        "the singleton satisfies every law"
    );
}

/// The generic instances bound the family from below: a law that holds
/// identically in the coefficients holds in every specialisation.
#[test]
fn generic_models_are_contained_in_every_specialisation() {
    let c = ctx();
    let nc = LinearModel::new(FreeNc, FreeNc.gen_a(), FreeNc.gen_b()).signature(&c.ll);
    let comm = LinearModel::new(FreeComm, FreeComm.gen_a(), FreeComm.gen_b()).signature(&c.ll);

    let contained = |small: &Signature, big: &Signature, what: &str| {
        for i in small.iter_set() {
            assert!(big.get(i), "{what}: E{} escapes", i + 1);
        }
    };

    // Z[a,b] is a quotient of Z<a,b>, so identities survive.
    contained(&nc, &comm, "free noncommutative inside free commutative");

    let m23 = MatFp { p: 3, k: 2 };
    let specialisations: Vec<Signature> = vec![
        int_sig(2, 3),
        int_sig(-1, -1),
        zmod_sig(5, 2, 3),
        zmod_sig(4, 1, 3),
        LinearModel::new(PolyZ, PolyZ::lin(0, 1), PolyZ::lin(1, -1)).signature(&c.ll),
        LinearModel::new(
            m23.clone(),
            m23.from_rows(&[&[1, 1], &[0, 1]]),
            m23.from_rows(&[&[0, 1], &[1, 0]]),
        )
        .signature(&c.ll),
    ];
    for (i, s) in specialisations.iter().enumerate() {
        contained(
            &nc,
            s,
            &format!("free noncommutative inside specialisation {i}"),
        );
    }

    // Measured, not assumed: on the order-≤4 law set the two generic models
    // agree exactly. A law separating them would need the same variable to
    // sit at path `ab` on one side and `ba` on the other with every other
    // variable's paths matching, and no law of order 4 does that. The gap
    // opens for *specialisations*, not for the generic models — see
    // `noncommutative_specialisation_escapes_every_commutative_one`.
    assert_eq!(
        nc.count(),
        comm.count(),
        "generic commutative and noncommutative linear magmas differ"
    );
    assert_eq!(nc, comm);
}

/// Paper Example 5.2: the variety for E1286 is
/// `{1 = ba^3 + bab, 0 = a + ba^2 b + b^2}` and the variety for E3 is
/// `{a + b = 1}`. Reproducing published varieties exactly is the strongest
/// available check on the polynomial extraction, independent of any ring.
#[test]
fn difference_polynomials_reproduce_published_varieties() {
    let c = ctx();
    let poly = |id: u32, var: usize| c.ll.laws[id as usize - 1].diffs[var];

    // Terms are given as (word, coefficient) with the word read root-to-leaf.
    let check = |p: parsimagma::nc::NcPoly, want: &[(&str, i32)], what: &str| {
        let mut expect = parsimagma::nc::NcPoly::ZERO;
        for (w, k) in want {
            let len = w.len() as u32;
            let bits = w
                .bytes()
                .fold(0u32, |acc, ch| (acc << 1) | u32::from(ch == b'b'));
            expect.add_word(len, bits, *k);
        }
        assert_eq!(p, expect, "{what}");
    };

    // E1286: x ≃ y ◇ (((x ◇ y) ◇ x) ◇ y)
    check(
        poly(1286, 0),
        &[("", 1), ("baaa", -1), ("bab", -1)],
        "E1286 in x",
    );
    check(
        poly(1286, 1),
        &[("a", -1), ("baab", -1), ("bb", -1)],
        "E1286 in y",
    );
    // E3: x = x ◇ x
    check(poly(3, 0), &[("", 1), ("a", -1), ("b", -1)], "E3");
    // E1117: x ≃ y ◇ ((y ◇ (x ◇ z)) ◇ z), paper Example 5.3
    check(poly(1117, 0), &[("", 1), ("baba", -1)], "E1117 in x");
    check(poly(1117, 1), &[("a", -1), ("baa", -1)], "E1117 in y");
    check(poly(1117, 2), &[("babb", -1), ("bb", -1)], "E1117 in z");
    // E2441: x ≃ (x ◇ ((x ◇ x) ◇ x)) ◇ x
    check(
        poly(2441, 0),
        &[
            ("", 1),
            ("aa", -1),
            ("abaa", -1),
            ("abab", -1),
            ("abb", -1),
            ("b", -1),
        ],
        "E2441",
    );
}

/// Paper Example 5.2 in full: `(p, a, b) = (11, 1, 7)` is a commutative
/// linear witness for `E1286 ⊭ E3`.
#[test]
fn commutative_linear_model_reproduces_e1286_not_implies_e3() {
    let s = zmod_sig(11, 1, 7);
    assert!(s.get(1286 - 1), "Z/11 with (a,b)=(1,7) must satisfy E1286");
    assert!(!s.get(3 - 1), "and must refute E3");
}

/// Paper Example 5.3: `E1117 ⊭ E2441` needs a noncommutative model, and by
/// Remark 5.4 that separation has no finite counterexample at all. The
/// one-sided-inverse ring realises it.
#[test]
fn noncommutative_linear_model_reproduces_e1117_not_implies_e2441() {
    let c = ctx();
    let r = parsimagma::rings::OneSidedInverse;
    let s = LinearModel::new(r.clone(), r.gen_a(), r.gen_b()).signature(&c.ll);
    assert!(
        s.get(1117 - 1),
        "Z<a,b>/(ba+1) must satisfy E1117: ba = -1 puts it in the variety"
    );
    assert!(s.get(2441 - 1) == false, "and must refute E2441");

    // The paper's other claim about this instance: no finite magma can
    // witness it, so no commutative linear model can either (Remark 5.4
    // sends commutative witnesses to Z/p). Check that across the grid.
    for m in 2u64..=13 {
        for a in 0..m {
            for b in 0..m {
                let t = zmod_sig(m, a, b);
                if t.get(1117 - 1) {
                    assert!(
                        t.get(2441 - 1),
                        "Z/{m} ({a},{b}) would be a finite witness for E1117 ⊭ E2441"
                    );
                }
            }
        }
    }
}

/// Paper Remark 5.6: `E1485 ⊭ E151` is immune to linear models of either
/// kind. Every instance in the grid that satisfies E1485 must satisfy E151.
#[test]
fn e1485_not_implies_e151_is_immune_to_linear_models() {
    let c = ctx();
    let mut witnesses = 0usize;
    let mut checked = 0usize;
    let mut consider = |s: &Signature| {
        checked += 1;
        if s.get(1485 - 1) {
            witnesses += 1;
            assert!(
                !s.get(151 - 1) == false,
                "found a linear witness for E1485 ⊭ E151"
            );
        }
    };
    for m in 2u64..=13 {
        for a in 0..m {
            for b in 0..m {
                consider(&zmod_sig(m, a, b));
            }
        }
    }
    for a in -3i128..=3 {
        for b in -3i128..=3 {
            consider(&int_sig(a, b));
        }
    }
    let r = parsimagma::rings::OneSidedInverse;
    consider(&LinearModel::new(r.clone(), r.gen_a(), r.gen_b()).signature(&c.ll));
    assert!(checked > 700, "grid too small to be meaningful: {checked}");
    let _ = witnesses;
}

/// A noncommutative instance whose carrier is far past any table sweep. The
/// point of the tier: `M_2(F_3)` has 81 elements, and a six-variable sweep
/// over it would need 81^6 assignments.
#[test]
fn matrix_instances_are_decided_without_a_carrier_sweep() {
    let c = ctx();
    let r = MatFp { p: 3, k: 2 };
    assert_eq!(
        parsimagma::linear::RingOps::carrier_size(&r),
        Some(81),
        "M_2(F_3) has 3^4 elements"
    );
    let a = r.from_rows(&[&[0, 2], &[1, 0]]);
    let b = r.from_rows(&[&[1, 1], &[0, 2]]);
    let s = LinearModel::new(r, a, b).signature(&c.ll);
    assert!(s.get(0), "E1");
    assert!(!s.get(1), "E2");
    assert!(
        s.is_separating(),
        "an 81-element linear magma should separate"
    );
}

/// The instance that carries most of the measured hard-core coverage, checked
/// the long way round. `Z/13` with `a = b = 7` is a 13-element magma, so its
/// signature can be computed by exhaustive sweep as well as symbolically, and
/// the two must agree on all 4694 bits. If they do not, the coverage number
/// is worthless.
#[test]
fn the_load_bearing_instance_survives_a_full_table_sweep() {
    let c = ctx();
    for (m, a, b) in [(13u64, 7u64, 7u64), (13, 4, 11), (13, 11, 4), (11, 1, 7)] {
        let model = LinearModel::new(Zmod { m }, a, b);
        let symbolic = model.signature(&c.ll);
        let elements: Vec<u64> = (0..m).collect();
        let magma = FiniteMagma::new(m as usize, model.table(&elements)).unwrap();
        let swept = c.engine.signature(&magma);
        assert_eq!(
            symbolic,
            swept,
            "Z/{m} ({a},{b}): symbolic {} laws vs swept {} laws",
            symbolic.count(),
            swept.count()
        );
    }
}

/// The same cross-check on the noncommutative path. `M_2(F_2)` has 16
/// elements, which is just inside sweep range, so the matrix arithmetic and
/// the word-order convention get validated against a table too.
#[test]
fn matrix_instances_agree_with_a_table_sweep_at_16_elements() {
    let c = ctx();
    let r = MatFp { p: 2, k: 2 };
    let elements: Vec<Vec<u64>> = (0..16usize)
        .map(|code| (0..4).map(|i| ((code >> i) & 1) as u64).collect())
        .collect();
    // Pick genuinely noncommuting pairs rather than guessing at them: over
    // F_2 many hand-picked matrices turn out to commute.
    let mut pairs = Vec::new();
    'outer: for a in &elements {
        for b in &elements {
            if r.mul(a, b) != r.mul(b, a) {
                pairs.push((a.clone(), b.clone()));
                if pairs.len() == 2 {
                    break 'outer;
                }
            }
        }
    }
    assert_eq!(pairs.len(), 2, "M_2(F_2) must contain noncommuting pairs");

    for (a, b) in pairs {
        let model = LinearModel::new(r.clone(), a, b);
        let symbolic = model.signature(&c.ll);
        let magma = FiniteMagma::new(16, model.table(&elements)).unwrap();
        assert_eq!(symbolic, c.engine.signature(&magma));
    }
}

/// Affine models get the same treatment as linear ones: every instance whose
/// carrier can be swept must agree with the sweep, and setting the constant
/// to zero must reproduce the linear model exactly.
#[test]
fn affine_models_agree_with_the_table_route() {
    use parsimagma::linear::AffineModel;
    let c = ctx();
    let mut checked = 0usize;
    for m in 2u64..=5 {
        let elements: Vec<u64> = (0..m).collect();
        for a in 0..m {
            for b in 0..m {
                // c = 0 must collapse to the linear model.
                assert_eq!(
                    AffineModel::new(Zmod { m }, a, b, 0).signature(&c.ll),
                    LinearModel::new(Zmod { m }, a, b).signature(&c.ll),
                    "Z/{m} ({a},{b}) with zero constant"
                );
                for cc in 0..m {
                    let model = AffineModel::new(Zmod { m }, a, b, cc);
                    let symbolic = model.signature(&c.ll);
                    let magma = FiniteMagma::new(m as usize, model.table(&elements)).unwrap();
                    for x in 0..m {
                        for y in 0..m {
                            assert_eq!(magma.op(x as u8, y as u8) as u64, (a * x + b * y + cc) % m);
                        }
                    }
                    assert_eq!(
                        symbolic,
                        c.engine.signature(&magma),
                        "Z/{m} affine ({a},{b},{cc})"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 8 + 27 + 64 + 125);
}

/// Exhaustive enumeration reproduces the number of magmas up to isomorphism
/// at orders 2 and 3, which is an independent check on the canonical-form
/// filter that the coverage counts depend on.
#[test]
fn canonical_filter_counts_magmas_up_to_isomorphism() {
    use parsimagma::finite::{is_canonical, permutations};
    // OEIS A001329: 1, 10, 3330, 178981952 magmas of order 1, 2, 3, 4 up to
    // isomorphism.
    for (n, expect) in [(1usize, 1usize), (2, 10), (3, 3330)] {
        let perms = permutations(n);
        let total = (n as u64).pow((n * n) as u32);
        let mut table = vec![0u8; n * n];
        let mut buf = vec![0u8; n * n];
        let mut count = 0usize;
        for code in 0..total {
            let mut v = code;
            for cell in table.iter_mut() {
                *cell = (v % n as u64) as u8;
                v /= n as u64;
            }
            if is_canonical(&table, n, &perms, &mut buf) {
                count += 1;
            }
        }
        assert_eq!(count, expect, "magmas of order {n} up to isomorphism");
    }
}
