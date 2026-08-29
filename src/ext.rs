//! Magma extensions: paper §5.6, blueprint chapter "Magma cohomology".
//!
//! An extension of a base magma `B` by `Z/m` is the magma on `B × Z/m` with
//!
//! ```text
//!     (x, s) ◇ (y, t) = (x ◇ y, α s + β t + c(x, y))
//! ```
//!
//! for fixed `α, β ∈ Z/m` and a cocycle `c: B × B → Z/m`. Elements are
//! numbered `x * m + s`, so the base coordinate is `e / m` and the fibre
//! coordinate is `e % m`.
//!
//! The point of the family is that it reaches separations no product or power
//! can. `docs/the-39.md` records the case that motivated it: the ETP's `Fin 65`
//! witness for `E1076 ⊭ E2294` is this shape, with base `4x + 2y` on `Z/5` and
//! fibre `Z/13` at `α = 5, β = 9` — and *both* ingredients satisfy E1076, E2294
//! and E4435 on their own. The separation lives entirely in the cocycle.
//!
//! What makes the family searchable rather than hopeless is that for a fixed
//! base, fibre and `(α, β)`, whether the extension satisfies a law is a *linear*
//! condition on `c`. So the cocycles that work form a subspace of `(Z/m)^{|B|²}`
//! and are found by elimination rather than by enumerating `m^{|B|²}`. For the
//! E1076 case that is 13^5 = 371,293 instead of 13^25.

use crate::finite::FiniteMagma;
use crate::law::{Law, Term};

/// An extension of `base` by `Z/m`, twisted by `cocycle`.
#[derive(Clone, Debug)]
pub struct Extension {
    pub base: FiniteMagma,
    pub m: u32,
    pub alpha: u32,
    pub beta: u32,
    /// Row-major `|B| × |B|`, entries in `0..m`.
    pub cocycle: Vec<u32>,
}

impl Extension {
    pub fn carrier(&self) -> usize {
        self.base.n * self.m as usize
    }

    /// The explicit multiplication table.
    ///
    /// # Panics
    /// If the carrier exceeds 255, which `FiniteMagma` cannot address.
    pub fn magma(&self) -> FiniteMagma {
        let (nb, m) = (self.base.n, self.m as usize);
        let n = nb * m;
        assert!(n <= 255, "extension carrier {n} exceeds the table limit");
        let mut table = vec![0u8; n * n];
        for bx in 0..nb {
            for by in 0..nb {
                let bz = self.base.op(bx as u8, by as u8) as usize;
                let c = self.cocycle[bx * nb + by] as usize;
                for s in 0..m {
                    for t in 0..m {
                        let f = (self.alpha as usize * s + self.beta as usize * t + c) % m;
                        table[(bx * m + s) * n + (by * m + t)] = (bz * m + f) as u8;
                    }
                }
            }
        }
        FiniteMagma::new(n, table).expect("extension table is well formed")
    }
}

fn eval(t: &Term, vals: &[u8], tbl: &[u8], n: usize) -> u8 {
    match t {
        Term::Var(v) => vals[*v as usize],
        Term::Op(l, r) => {
            let x = eval(l, vals, tbl, n) as usize;
            let y = eval(r, vals, tbl, n) as usize;
            tbl[x * n + y]
        }
    }
}

/// The fibre residual of `law`, one value per assignment of base elements.
///
/// Returns `None` when the base coordinates themselves disagree, i.e. the base
/// magma does not satisfy the law — no cocycle repairs that. `fibre` fixes the
/// fibre coordinate given to every variable; the residual is independent of it
/// whenever the linear part is right, and `residual_is_flat` checks that rather
/// than assuming it.
fn residual(ext: &Extension, law: &Law, fibre: u8) -> Option<Vec<u32>> {
    let (nb, m) = (ext.base.n, ext.m as usize);
    let tbl = ext.magma();
    let n = tbl.n;
    let total = nb.pow(law.arity as u32);
    let mut out = Vec::with_capacity(total);
    let mut vals = vec![0u8; law.arity as usize];
    for code in 0..total {
        let mut v = code;
        for cell in vals.iter_mut() {
            *cell = ((v % nb) * m + fibre as usize) as u8;
            v /= nb;
        }
        let l = eval(&law.lhs, &vals, &tbl.table, n) as usize;
        let r = eval(&law.rhs, &vals, &tbl.table, n) as usize;
        if l / m != r / m {
            return None;
        }
        out.push(((r % m + m) - l % m) as u32 % ext.m);
    }
    Some(out)
}

/// The fibre residual of `law` under a given cocycle, at fibre coordinate zero.
/// Zero everywhere exactly when the extension satisfies the law.
pub fn law_residual(ext: &Extension, law: &Law) -> Option<Vec<u32>> {
    residual(ext, law, 0)
}

/// Whether the residual really is independent of the fibre coordinate, which
/// is what makes the system linear in `c` alone.
pub fn residual_is_flat(ext: &Extension, law: &Law) -> bool {
    let base = match residual(ext, law, 0) {
        Some(r) => r,
        None => return false,
    };
    (1..ext.m.min(4)).all(|f| residual(ext, law, f as u8).as_ref() == Some(&base))
}

/// The affine space of cocycles making the extension satisfy `law`:
/// a particular solution plus a basis of the homogeneous part, over `F_m`.
#[derive(Debug)]
pub struct CocycleSpace {
    pub particular: Vec<u32>,
    pub basis: Vec<Vec<u32>>,
    pub m: u32,
}

impl CocycleSpace {
    pub fn dimension(&self) -> usize {
        self.basis.len()
    }
    /// The `i`th member under a fixed enumeration of the space.
    pub fn member(&self, coeffs: &[u32]) -> Vec<u32> {
        let mut c = self.particular.clone();
        for (k, b) in self.basis.iter().enumerate() {
            let a = coeffs.get(k).copied().unwrap_or(0) % self.m;
            for (ci, bi) in c.iter_mut().zip(b) {
                *ci = (*ci + a * bi) % self.m;
            }
        }
        c
    }
}

fn inv_mod(a: u32, p: u32) -> u32 {
    let mut r = 1u64;
    let (mut b, mut e) = (a as u64 % p as u64, p as u64 - 2);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % p as u64;
        }
        b = b * b % p as u64;
        e >>= 1;
    }
    r as u32
}

fn is_prime(n: u32) -> bool {
    n >= 2
        && (2..)
            .take_while(|d| d * d <= n)
            .all(|d| !n.is_multiple_of(d))
}

/// Solve for the cocycles under which the extension satisfies `law`.
///
/// `m` must be prime, so that the fibre is a field and elimination applies.
/// Returns `None` when the base magma fails the law, when the residual is not
/// flat in the fibre coordinate, or when the system is inconsistent.
pub fn cocycle_space(
    base: &FiniteMagma,
    m: u32,
    alpha: u32,
    beta: u32,
    law: &Law,
) -> Option<CocycleSpace> {
    assert!(is_prime(m), "fibre modulus {m} is not prime");
    let nv = base.n * base.n;
    let zero = Extension {
        base: base.clone(),
        m,
        alpha,
        beta,
        cocycle: vec![0; nv],
    };
    if !residual_is_flat(&zero, law) {
        return None;
    }
    let r0 = residual(&zero, law, 0)?;

    // Column j is the effect of bumping cocycle entry j by one. The map is
    // linear, so probing the unit vectors determines it.
    let mut cols: Vec<Vec<u32>> = Vec::with_capacity(nv);
    for j in 0..nv {
        let mut e = zero.clone();
        e.cocycle[j] = 1;
        let rj = residual(&e, law, 0)?;
        cols.push(
            rj.iter()
                .zip(&r0)
                .map(|(a, b)| (a + m - b % m) % m)
                .collect(),
        );
    }

    // Augmented system: sum_j c_j * col_j = -r0.
    let neq = r0.len();
    let mut a: Vec<Vec<u32>> = (0..neq)
        .map(|i| {
            let mut row: Vec<u32> = (0..nv).map(|j| cols[j][i]).collect();
            row.push((m - r0[i] % m) % m);
            row
        })
        .collect();

    let mut pivots: Vec<usize> = Vec::new();
    let mut r = 0usize;
    for c in 0..nv {
        let Some(pr) = (r..neq).find(|&i| !a[i][c].is_multiple_of(m)) else {
            continue;
        };
        a.swap(r, pr);
        let inv = inv_mod(a[r][c], m);
        for v in a[r].iter_mut() {
            *v = *v * inv % m;
        }
        for i in 0..neq {
            if i != r && !a[i][c].is_multiple_of(m) {
                let f = a[i][c];
                let pivot = a[r].clone();
                for (aij, prj) in a[i].iter_mut().zip(&pivot) {
                    *aij = (*aij + m - f * prj % m) % m;
                }
            }
        }
        pivots.push(c);
        r += 1;
        if r == neq {
            break;
        }
    }
    // Inconsistent if some row is 0 = nonzero.
    if a.iter()
        .any(|row| row[..nv].iter().all(|v| v % m == 0) && row[nv] % m != 0)
    {
        return None;
    }

    let free: Vec<usize> = (0..nv).filter(|c| !pivots.contains(c)).collect();
    let solve = |freevals: &[u32]| -> Vec<u32> {
        let mut x = vec![0u32; nv];
        for (k, &c) in free.iter().enumerate() {
            x[c] = freevals[k] % m;
        }
        for (i, &c) in pivots.iter().enumerate() {
            let s: u32 = free
                .iter()
                .map(|&j| a[i][j] * x[j] % m)
                .fold(0, |acc, v| (acc + v) % m);
            x[c] = (a[i][nv] + m - s) % m;
        }
        x
    };
    let particular = solve(&vec![0; free.len()]);
    let basis: Vec<Vec<u32>> = (0..free.len())
        .map(|k| {
            let mut fv = vec![0u32; free.len()];
            fv[k] = 1;
            let s = solve(&fv);
            s.iter()
                .zip(&particular)
                .map(|(a, b)| (a + m - b) % m)
                .collect()
        })
        .collect();
    Some(CocycleSpace {
        particular,
        basis,
        m,
    })
}

/// The verdict for one (setting, target) question.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every cocycle satisfying the hypothesis also satisfies the target, so
    /// this setting provably cannot separate the pair.
    Blocked,
    /// A cocycle satisfying the hypothesis and refuting the target.
    Separates(Vec<u32>),
    /// The target's residual depends on the fibre coordinate, so testing it at
    /// coordinate zero decides nothing. Report rather than guess.
    Undecided,
}

/// Whether *every* cocycle satisfying the hypothesis also satisfies `target`.
///
/// The target's residual is affine in the cocycle and the hypothesis's solution
/// set is an affine subspace, so the restriction is affine and vanishes
/// identically exactly when it vanishes at the particular solution and at each
/// `particular + basis_i`. That is `dim + 1` probes, and it decides the question
/// where sampling can only fail to find something.
///
/// The probes read the residual at fibre coordinate zero, which is faithful only
/// when the target's residual is flat in that coordinate. A nonzero residual is
/// a genuine refutation either way, so `Separates` is always sound; `Blocked`
/// is claimed only once flatness is checked, and `Undecided` is returned
/// otherwise.
pub fn separates(
    base: &FiniteMagma,
    m: u32,
    alpha: u32,
    beta: u32,
    sp: &CocycleSpace,
    target: &Law,
) -> Verdict {
    let mut probes: Vec<Vec<u32>> = vec![sp.particular.clone()];
    for b in &sp.basis {
        probes.push(
            sp.particular
                .iter()
                .zip(b)
                .map(|(p, q)| (p + q) % m)
                .collect(),
        );
    }
    let mut flat = true;
    for c in probes {
        let e = Extension {
            base: base.clone(),
            m,
            alpha,
            beta,
            cocycle: c.clone(),
        };
        match law_residual(&e, target) {
            // The base itself refutes the target, so the extension does too.
            None => return Verdict::Separates(c),
            Some(r) if r.iter().any(|&v| v % m != 0) => return Verdict::Separates(c),
            _ => {
                if flat && !residual_is_flat(&e, target) {
                    flat = false;
                }
            }
        }
    }
    if flat {
        Verdict::Blocked
    } else {
        Verdict::Undecided
    }
}
