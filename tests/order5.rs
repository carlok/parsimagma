//! Validation of the engine on the order-5 law set, which the open-question
//! scan depends on. A negative result from an unexercised code path is worth
//! nothing, so these run before any claim about the open sets.

use parsimagma::linear::{LinearLaws, LinearModel};
use parsimagma::rings::Zmod;
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma, Law};
use std::sync::OnceLock;

fn data(name: &str) -> String {
    parsimagma::etpdata::read_text(name)
}

fn eq5() -> &'static Vec<Law> {
    static L: OnceLock<Vec<Law>> = OnceLock::new();
    L.get_or_init(|| parse_laws(&data("eq_size5.txt")).unwrap())
}

#[test]
fn order5_law_list_parses_and_extends_the_order4_one() {
    let five = eq5();
    let four = parse_laws(&data("equations.txt")).unwrap();
    // 4694 laws of order at most 4, plus 57,882 of order 5.
    assert_eq!(five.len(), 62_576);
    assert_eq!(five.len() - four.len(), 57_882);
    // The order-≤4 prefix must agree law for law, or the ids in the blueprint
    // tables would point at the wrong equations.
    for (a, b) in four.iter().zip(five.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(format!("{:?}", a.lhs), format!("{:?}", b.lhs));
        assert_eq!(format!("{:?}", a.rhs), format!("{:?}", b.rhs));
    }
    let max_ops = five.iter().map(|l| l.lhs.ops() + l.rhs.ops()).max().unwrap();
    let max_arity = five.iter().map(|l| l.arity).max().unwrap();
    assert_eq!(max_ops, 5);
    assert_eq!(max_arity, 7, "order-5 laws reach seven distinct variables");
}

/// Positive control. If the scan can find nothing at all on this law set, a
/// negative result on the open sets means only that the code is broken.
#[test]
fn the_scan_finds_models_where_models_are_known_to_exist() {
    let laws: Vec<Law> = eq5().iter().skip(4694).take(4000).cloned().collect();
    let ll = LinearLaws::build(&laws);
    let mut hit_laws = std::collections::BTreeSet::new();
    let mut hit_instances = 0usize;
    for m in 2u64..=16 {
        for a in 0..m {
            for b in 0..m {
                let s = LinearModel::new(Zmod { m }, a, b).signature(&ll);
                if s.count() > 0 {
                    hit_instances += 1;
                    for i in s.iter_set() {
                        hit_laws.insert(laws[i].id);
                    }
                }
            }
        }
    }
    assert!(
        hit_laws.len() > 200,
        "only {} of 4000 order-5 laws found a Z/m linear model; the scan is not working",
        hit_laws.len()
    );
    assert!(hit_instances > 100);
}

/// The same symbolic-versus-table cross-check that validates Tier S on the
/// order-4 set, rerun on the order-5 laws the open-question scan targets.
#[test]
fn symbolic_and_table_routes_agree_on_order5_laws() {
    let laws: Vec<Law> = eq5().iter().skip(4694).take(3000).cloned().collect();
    let ll = LinearLaws::build(&laws);
    let engine = Engine::new(Dag::build(&laws));
    let mut checked = 0usize;
    for m in 2u64..=4 {
        for a in 0..m {
            for b in 0..m {
                let model = LinearModel::new(Zmod { m }, a, b);
                let elements: Vec<u64> = (0..m).collect();
                let magma = FiniteMagma::new(m as usize, model.table(&elements)).unwrap();
                assert_eq!(
                    model.signature(&ll),
                    engine.signature(&magma),
                    "Z/{m} ({a},{b}) on order-5 laws"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 4 + 9 + 16);
}

/// The ten laws the blueprint proves are Austin laws admit no nontrivial
/// finite model at all, so no `Z/m` instance may satisfy one. This is the
/// negative control for the open-question scan: it must come back empty for
/// the right reason.
#[test]
fn known_austin_laws_admit_no_finite_linear_model() {
    let spec = data("order5_open.txt");
    let ids: Vec<u32> = spec
        .lines()
        .filter(|l| l.starts_with("austin\t"))
        .map(|l| l.split('\t').nth(1).unwrap().parse().unwrap())
        .collect();
    assert_eq!(ids.len(), 10);
    let laws: Vec<Law> = ids.iter().map(|i| eq5()[*i as usize - 1].clone()).collect();
    let ll = LinearLaws::build(&laws);
    for m in 2u64..=48 {
        for a in 0..m {
            for b in 0..m {
                let s = LinearModel::new(Zmod { m }, a, b).signature(&ll);
                assert_eq!(
                    s.count(),
                    0,
                    "Z/{m} ({a},{b}) satisfies an Austin law, which has no nontrivial finite model"
                );
            }
        }
    }
}

/// The Weyl algebra is the ring the finite-quotient argument singles out, so
/// its arithmetic has to be right before any negative result over it means
/// anything.
#[test]
fn weyl_algebra_satisfies_its_defining_relation() {
    use parsimagma::linear::RingOps;
    use parsimagma::rings::WeylAlgebra;
    let r = WeylAlgebra;
    let (a, b) = (r.gen_a(), r.gen_b());

    // ba - ab = 1, the defining relation.
    let ba = r.mul(&b, &a);
    let ab = r.mul(&a, &b);
    let mut d = ba;
    r.scale_add_assign(&mut d, -1, &ab);
    assert_eq!(d, r.one(), "ba - ab must be 1");

    // ab is not 1, so a has no inverse: this is not the one-sided-inverse ring.
    assert_ne!(ab, r.one());
    // Associativity on the generators, which the normal-form reordering could
    // easily break.
    assert_eq!(r.mul(&r.mul(&a, &b), &a), r.mul(&a, &r.mul(&b, &a)));
    assert_eq!(r.mul(&r.mul(&b, &a), &b), r.mul(&b, &r.mul(&a, &b)));
    // Leibniz: b a^2 = a^2 b + 2a.
    let a2 = r.mul(&a, &a);
    let lhs = r.mul(&b, &a2);
    let mut rhs = r.mul(&a2, &b);
    r.scale_add_assign(&mut rhs, 2, &a);
    assert_eq!(lhs, rhs, "b a^2 = a^2 b + 2a");
}

/// Every law holding identically in the free noncommutative ring must hold in
/// any quotient of it, the Weyl algebra included. This checks the Weyl linear
/// magma is actually being evaluated rather than silently returning nothing.
#[test]
fn weyl_linear_magma_contains_the_generic_signature() {
    use parsimagma::linear::{LinearLaws, LinearModel};
    use parsimagma::rings::{FreeNc, WeylAlgebra};
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let ll = LinearLaws::build(&laws);
    let generic = LinearModel::new(FreeNc, FreeNc.gen_a(), FreeNc.gen_b()).signature(&ll);
    let weyl = LinearModel::new(WeylAlgebra, WeylAlgebra.gen_a(), WeylAlgebra.gen_b())
        .signature(&ll);
    assert!(generic.count() > 0, "the generic model satisfies something");
    for i in generic.iter_set() {
        assert!(weyl.get(i), "E{} holds identically but not in the Weyl algebra", i + 1);
    }
    assert!(
        weyl.count() >= generic.count(),
        "weyl {} vs generic {}",
        weyl.count(),
        generic.count()
    );
}
