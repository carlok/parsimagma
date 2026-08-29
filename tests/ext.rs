//! Section 5.6 extensions, checked against the ETP's own witness.
//!
//! `Generated/All4x4Tables/Refutation938.lean` is a `Fin 65` magma satisfying
//! E1076 and refuting E2294 and E4435. It is an extension: base `4x + 2y` on
//! `Z/5`, fibre `Z/13` at `α = 5, β = 9`, twisted by a cocycle. If this family
//! is implemented correctly it rediscovers that witness from the ingredients,
//! and the cocycle ETP published lies in the space the solver returns.

use parsimagma::ext::{cocycle_space, Extension};
use parsimagma::{parse_laws, Engine, FiniteMagma, Law};

fn laws() -> Vec<Law> {
    parse_laws(&parsimagma::etpdata::read_text("equations.txt")).expect("law list parses")
}

fn law(ls: &[Law], id: u32) -> &Law {
    ls.iter().find(|l| l.id == id).expect("law present")
}

/// `x ◇ y = 4x + 2y` on `Z/5`, the base of Refutation938.
fn base() -> FiniteMagma {
    let n = 5;
    let mut t = vec![0u8; n * n];
    for x in 0..n {
        for y in 0..n {
            t[x * n + y] = ((4 * x + 2 * y) % n) as u8;
        }
    }
    FiniteMagma::new(n, t).unwrap()
}

/// The cocycle read off Refutation938's table.
const ETP_COCYCLE: [u32; 25] = [
    0, 0, 5, 11, 12, 1, 0, 7, 10, 0, 0, 8, 0, 0, 11, 0, 4, 3, 0, 4, 0, 10, 1, 4, 0,
];

/// Whether the extension satisfies one law.
///
/// Restricted to that single law on purpose: a full 4694-law sweep at carrier
/// 65 would need 65^6 assignments for the six-variable bucket, which is not a
/// sweep anyone runs. The three laws here use two variables.
fn sig_has(e: &Extension, ls: &[Law], id: u32) -> bool {
    let one: Vec<Law> = ls.iter().filter(|l| l.id == id).cloned().collect();
    let eng = Engine::new(parsimagma::Dag::build(&one));
    eng.signature(&e.magma()).get(0)
}

#[test]
fn reproduces_refutation938() {
    let ls = laws();
    let b = base();
    let sp = cocycle_space(&b, 13, 5, 9, law(&ls, 1076)).expect("E1076 system is solvable");

    // The blueprint's cohomology chapter predicts a space, not a point.
    assert_eq!(sp.dimension(), 5, "expected 13^5 cocycles satisfying E1076");

    // ETP's published cocycle must satisfy E1076 through this construction.
    let etp = Extension {
        base: b.clone(),
        m: 13,
        alpha: 5,
        beta: 9,
        cocycle: ETP_COCYCLE.to_vec(),
    };
    assert_eq!(etp.carrier(), 65);
    assert!(sig_has(&etp, &ls, 1076), "ETP's witness satisfies E1076");
    assert!(!sig_has(&etp, &ls, 2294), "ETP's witness refutes E2294");
    assert!(!sig_has(&etp, &ls, 4435), "ETP's witness refutes E4435");

    // Every member of the solved space satisfies E1076, and separating members
    // are common rather than rare: the search is worth running.
    let mut separating = 0;
    let mut tried = 0;
    for k in 0..40u32 {
        let coeffs = [
            k % 13,
            (k * 5) % 13,
            (k * 7) % 13,
            (k * 11) % 13,
            (k * 3) % 13,
        ];
        let e = Extension {
            base: b.clone(),
            m: 13,
            alpha: 5,
            beta: 9,
            cocycle: sp.member(&coeffs),
        };
        assert!(sig_has(&e, &ls, 1076), "solved cocycle satisfies E1076");
        tried += 1;
        if !sig_has(&e, &ls, 2294) || !sig_has(&e, &ls, 4435) {
            separating += 1;
        }
    }
    assert!(
        separating * 2 > tried,
        "most of the space should separate; got {separating} of {tried}"
    );
}

#[test]
fn base_and_fibre_alone_separate_nothing() {
    // The point of the family: both ingredients satisfy all three laws, so no
    // product or power of them can witness the separation. Only the twist does.
    let ls = laws();
    let three: Vec<Law> = ls
        .iter()
        .filter(|l| matches!(l.id, 1076 | 2294 | 4435))
        .cloned()
        .collect();
    let eng = Engine::new(parsimagma::Dag::build(&three));
    let idx = |id: u32| three.iter().position(|l| l.id == id).unwrap();

    let bs = eng.signature(&base());
    for id in [1076u32, 2294, 4435] {
        assert!(bs.get(idx(id)), "base satisfies E{id}");
    }

    let m = 13usize;
    let mut t = vec![0u8; m * m];
    for x in 0..m {
        for y in 0..m {
            t[x * m + y] = ((5 * x + 9 * y) % m) as u8;
        }
    }
    let fs = eng.signature(&FiniteMagma::new(m, t).unwrap());
    for id in [1076u32, 2294, 4435] {
        assert!(fs.get(idx(id)), "fibre satisfies E{id}");
    }
}
