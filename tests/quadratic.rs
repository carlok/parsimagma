//! Quadratic magmas, checked against the formula and against the linear family
//! they contain.

use parsimagma::quadratic::Quadratic;
use parsimagma::{parse_laws, Dag, Engine, FiniteMagma};

fn data(name: &str) -> String {
    parsimagma::etpdata::read_text(name)
}

#[test]
fn table_matches_the_defining_polynomial() {
    for n in 2u32..=9 {
        for code in [0u64, 1, 7, 137, 1009, 65537] {
            let q = Quadratic::from_code(n, code % Quadratic::grid_size(n));
            let m = q.magma();
            for x in 0..n {
                for y in 0..n {
                    let want =
                        (q.a * x * x + q.b * x * y + q.c * y * y + q.d * x + q.e * y + q.f) % n;
                    assert_eq!(m.op(x as u8, y as u8) as u32, want, "n={n} code={code}");
                }
            }
        }
    }
}

/// With the quadratic and constant terms zero the family degenerates to
/// `x ◇ y = dx + ey`, which is already validated, so the two must agree.
#[test]
fn degenerates_to_the_linear_family() {
    let laws = parse_laws(&data("equations.txt")).unwrap();
    let e = Engine::new(Dag::build(&laws));
    let mut checked = 0usize;
    for n in 2u32..=6 {
        for d in 0..n {
            for ee in 0..n {
                let q = Quadratic {
                    n,
                    a: 0,
                    b: 0,
                    c: 0,
                    d,
                    e: ee,
                    f: 0,
                };
                assert!(q.is_linear());
                let linear: Vec<u8> = (0..(n * n))
                    .map(|k| (((d * (k / n)) + (ee * (k % n))) % n) as u8)
                    .collect();
                let lm = FiniteMagma::new(n as usize, linear).unwrap();
                assert_eq!(lm, q.magma(), "n={n} d={d} e={ee}");
                assert_eq!(e.signature(&lm), e.signature(&q.magma()));
                checked += 1;
            }
        }
    }
    assert!(checked >= 50);
}

#[test]
fn code_decoding_covers_the_grid() {
    let n = 3u32;
    let total = Quadratic::grid_size(n);
    assert_eq!(total, 729);
    let mut seen = std::collections::HashSet::new();
    for code in 0..total {
        let q = Quadratic::from_code(n, code);
        assert!(seen.insert((q.a, q.b, q.c, q.d, q.e, q.f)));
    }
    assert_eq!(seen.len() as u64, total);
}
