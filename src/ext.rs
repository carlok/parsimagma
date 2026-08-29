//! Magma extensions: paper §5.6, blueprint chapter "Magma cohomology".
//!
//! An extension of a base magma `B` by an abelian group `M` is the magma on
//! `B × M` with
//!
//! ```text
//!     (x, s) ◇ (y, t) = (x ◇ y, α s + β t + c(x, y))
//! ```
//!
//! for endomorphisms `α, β` of `M` and a cocycle `c: B × B → M`. Here `M` is
//! `(Z/p)^k` and the endomorphisms are `k × k` matrices over `F_p`, which is
//! the shape the blueprint's chapter-677 lemma states; `k = 1` recovers scalar
//! multiplication on `Z/p`. Elements are numbered `x * |M| + s` with the fibre
//! coordinate written base `p`, least significant digit first.
//!
//! The point of the family is that it reaches separations no product or power
//! can. `docs/the-39.md` records the case that motivated it: the ETP's `Fin 65`
//! witness for `E1076 ⊭ E2294` is this shape with `k = 1`, base `4x + 2y` on
//! `Z/5` and fibre `Z/13` at `α = 5, β = 9` — and *both* ingredients satisfy
//! E1076, E2294 and E4435 on their own. The separation is entirely in `c`.
//!
//! What makes the family searchable rather than hopeless is that once the base,
//! the fibre and `α, β` are fixed, whether the extension satisfies a law is a
//! *linear* condition on `c`. The cocycles that work are a subspace of
//! `(F_p)^{k|B|²}`, found by elimination rather than by enumerating `p^{k|B|²}`.

use crate::finite::FiniteMagma;
use crate::law::{Law, Term};

/// `(Z/p)^k` with two endomorphisms, as `k × k` row-major matrices over `F_p`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fibre {
    pub p: u32,
    pub k: usize,
    pub alpha: Vec<u32>,
    pub beta: Vec<u32>,
}

impl Fibre {
    /// The scalar case: `Z/p` with `α, β` acting by multiplication.
    pub fn scalar(p: u32, alpha: u32, beta: u32) -> Self {
        Fibre {
            p,
            k: 1,
            alpha: vec![alpha % p],
            beta: vec![beta % p],
        }
    }

    pub fn size(&self) -> usize {
        (self.p as usize).pow(self.k as u32)
    }

    fn decode(&self, mut e: usize, out: &mut [u32]) {
        for cell in out.iter_mut().take(self.k) {
            *cell = (e % self.p as usize) as u32;
            e /= self.p as usize;
        }
    }

    fn encode(&self, v: &[u32]) -> usize {
        let mut e = 0usize;
        for i in (0..self.k).rev() {
            e = e * self.p as usize + (v[i] % self.p) as usize;
        }
        e
    }

    fn apply_into(&self, mat: &[u32], v: &[u32], out: &mut [u32]) {
        for (i, cell) in out.iter_mut().enumerate().take(self.k) {
            let mut acc = 0u32;
            for j in 0..self.k {
                acc = (acc + mat[i * self.k + j] * v[j]) % self.p;
            }
            *cell = acc;
        }
    }
}

/// An extension of `base` by `fibre`, twisted by `cocycle`.
///
/// `cocycle` is `|B|² × k` row-major: entry `(x, y)` occupies
/// `cocycle[(x * |B| + y) * k ..][.. k]`.
#[derive(Clone, Debug)]
pub struct Extension {
    pub base: FiniteMagma,
    pub fibre: Fibre,
    pub cocycle: Vec<u32>,
}

impl Extension {
    pub fn n_cocycle_vars(&self) -> usize {
        self.base.n * self.base.n * self.fibre.k
    }

    pub fn carrier(&self) -> usize {
        self.base.n * self.fibre.size()
    }

    /// The explicit multiplication table.
    ///
    /// # Panics
    /// If the carrier exceeds 255, which `FiniteMagma` cannot address.
    pub fn magma(&self) -> FiniteMagma {
        let (nb, f) = (self.base.n, &self.fibre);
        let fs = f.size();
        let n = nb * fs;
        assert!(n <= 255, "extension carrier {n} exceeds the table limit");
        let mut table = vec![0u8; n * n];
        let (mut sv, mut tv) = (vec![0u32; f.k], vec![0u32; f.k]);
        let (mut av, mut bv) = (vec![0u32; f.k], vec![0u32; f.k]);
        let mut res = vec![0u32; f.k];
        for bx in 0..nb {
            for by in 0..nb {
                let bz = self.base.op(bx as u8, by as u8) as usize;
                let c = &self.cocycle[(bx * nb + by) * f.k..][..f.k];
                for s in 0..fs {
                    f.decode(s, &mut sv);
                    f.apply_into(&f.alpha, &sv, &mut av);
                    for t in 0..fs {
                        f.decode(t, &mut tv);
                        f.apply_into(&f.beta, &tv, &mut bv);
                        for i in 0..f.k {
                            res[i] = (av[i] + bv[i] + c[i]) % f.p;
                        }
                        table[(bx * fs + s) * n + (by * fs + t)] = (bz * fs + f.encode(&res)) as u8;
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

/// The fibre residual of `law`, one `k`-vector per assignment of base elements,
/// flattened.
///
/// Returns `None` when the base coordinates themselves disagree, i.e. the base
/// magma does not satisfy the law — no cocycle repairs that. `fibre_coord` fixes
/// the fibre coordinate given to every variable.
fn residual(ext: &Extension, law: &Law, fibre_coord: usize) -> Option<Vec<u32>> {
    let (nb, f) = (ext.base.n, &ext.fibre);
    let fs = f.size();
    let tbl = ext.magma();
    let n = tbl.n;
    let total = nb.pow(law.arity as u32);
    let mut out = Vec::with_capacity(total * f.k);
    let mut vals = vec![0u8; law.arity as usize];
    let (mut lv, mut rv) = (vec![0u32; f.k], vec![0u32; f.k]);
    for code in 0..total {
        let mut v = code;
        for cell in vals.iter_mut() {
            *cell = ((v % nb) * fs + fibre_coord) as u8;
            v /= nb;
        }
        let l = eval(&law.lhs, &vals, &tbl.table, n) as usize;
        let r = eval(&law.rhs, &vals, &tbl.table, n) as usize;
        if l / fs != r / fs {
            return None;
        }
        f.decode(l % fs, &mut lv);
        f.decode(r % fs, &mut rv);
        for i in 0..f.k {
            out.push((rv[i] + f.p - lv[i]) % f.p);
        }
    }
    Some(out)
}

/// The fibre residual under a given cocycle, at fibre coordinate zero. Zero
/// everywhere exactly when the extension satisfies the law.
pub fn law_residual(ext: &Extension, law: &Law) -> Option<Vec<u32>> {
    residual(ext, law, 0)
}

/// Whether the residual is independent of the fibre coordinate, which is what
/// makes the system linear in `c` alone.
pub fn residual_is_flat(ext: &Extension, law: &Law) -> bool {
    let base = match residual(ext, law, 0) {
        Some(r) => r,
        None => return false,
    };
    (1..ext.fibre.size().min(4)).all(|f| residual(ext, law, f).as_ref() == Some(&base))
}

/// The affine space of cocycles making the extension satisfy a law: a
/// particular solution plus a basis of the homogeneous part, over `F_p`.
#[derive(Debug)]
pub struct CocycleSpace {
    pub particular: Vec<u32>,
    pub basis: Vec<Vec<u32>>,
    pub p: u32,
}

impl CocycleSpace {
    pub fn dimension(&self) -> usize {
        self.basis.len()
    }
    pub fn member(&self, coeffs: &[u32]) -> Vec<u32> {
        let mut c = self.particular.clone();
        for (k, b) in self.basis.iter().enumerate() {
            let a = coeffs.get(k).copied().unwrap_or(0) % self.p;
            for (ci, bi) in c.iter_mut().zip(b) {
                *ci = (*ci + a * bi) % self.p;
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

pub fn is_prime(n: u32) -> bool {
    n >= 2
        && (2..)
            .take_while(|d| d * d <= n)
            .all(|d| !n.is_multiple_of(d))
}

/// Solve for the cocycles under which the extension satisfies `law`.
///
/// `p` must be prime, so that the fibre's coefficient ring is a field and
/// elimination applies. Returns `None` when the base magma fails the law, when
/// the residual is not flat in the fibre coordinate, or when the system is
/// inconsistent.
pub fn cocycle_space(base: &FiniteMagma, fibre: &Fibre, law: &Law) -> Option<CocycleSpace> {
    assert!(is_prime(fibre.p), "fibre modulus {} is not prime", fibre.p);
    let p = fibre.p;
    let nv = base.n * base.n * fibre.k;
    let zero = Extension {
        base: base.clone(),
        fibre: fibre.clone(),
        cocycle: vec![0; nv],
    };
    if !residual_is_flat(&zero, law) {
        return None;
    }
    let r0 = residual(&zero, law, 0)?;

    let mut cols: Vec<Vec<u32>> = Vec::with_capacity(nv);
    for j in 0..nv {
        let mut e = zero.clone();
        e.cocycle[j] = 1;
        let rj = residual(&e, law, 0)?;
        cols.push(
            rj.iter()
                .zip(&r0)
                .map(|(a, b)| (a + p - b % p) % p)
                .collect(),
        );
    }

    let neq = r0.len();
    let mut a: Vec<Vec<u32>> = (0..neq)
        .map(|i| {
            let mut row: Vec<u32> = (0..nv).map(|j| cols[j][i]).collect();
            row.push((p - r0[i] % p) % p);
            row
        })
        .collect();

    let mut pivots: Vec<usize> = Vec::new();
    let mut r = 0usize;
    for c in 0..nv {
        let Some(pr) = (r..neq).find(|&i| !a[i][c].is_multiple_of(p)) else {
            continue;
        };
        a.swap(r, pr);
        let inv = inv_mod(a[r][c], p);
        for v in a[r].iter_mut() {
            *v = *v * inv % p;
        }
        for i in 0..neq {
            if i != r && !a[i][c].is_multiple_of(p) {
                let f = a[i][c];
                let pivot = a[r].clone();
                for (aij, prj) in a[i].iter_mut().zip(&pivot) {
                    *aij = (*aij + p - f * prj % p) % p;
                }
            }
        }
        pivots.push(c);
        r += 1;
        if r == neq {
            break;
        }
    }
    if a.iter()
        .any(|row| row[..nv].iter().all(|v| v.is_multiple_of(p)) && !row[nv].is_multiple_of(p))
    {
        return None;
    }

    let free: Vec<usize> = (0..nv).filter(|c| !pivots.contains(c)).collect();
    let solve = |freevals: &[u32]| -> Vec<u32> {
        let mut x = vec![0u32; nv];
        for (k, &c) in free.iter().enumerate() {
            x[c] = freevals[k] % p;
        }
        for (i, &c) in pivots.iter().enumerate() {
            let s: u32 = free
                .iter()
                .map(|&j| a[i][j] * x[j] % p)
                .fold(0, |acc, v| (acc + v) % p);
            x[c] = (a[i][nv] + p - s) % p;
        }
        x
    };
    let particular = solve(&vec![0; free.len()]);
    let basis: Vec<Vec<u32>> = (0..free.len())
        .map(|k| {
            let mut fv = vec![0u32; free.len()];
            fv[k] = 1;
            solve(&fv)
                .iter()
                .zip(&particular)
                .map(|(a, b)| (a + p - b) % p)
                .collect()
        })
        .collect();
    Some(CocycleSpace {
        particular,
        basis,
        p,
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
/// The probes read the residual at fibre coordinate zero, faithful only when the
/// target's residual is flat there. A nonzero residual refutes the target either
/// way, so `Separates` is always sound; `Blocked` is claimed only once flatness
/// is checked, and `Undecided` returned otherwise.
pub fn separates(base: &FiniteMagma, fibre: &Fibre, sp: &CocycleSpace, target: &Law) -> Verdict {
    let p = fibre.p;
    let mut probes: Vec<Vec<u32>> = vec![sp.particular.clone()];
    for b in &sp.basis {
        probes.push(
            sp.particular
                .iter()
                .zip(b)
                .map(|(x, q)| (x + q) % p)
                .collect(),
        );
    }
    let mut flat = true;
    for c in probes {
        let e = Extension {
            base: base.clone(),
            fibre: fibre.clone(),
            cocycle: c.clone(),
        };
        match law_residual(&e, target) {
            None => return Verdict::Separates(c),
            Some(r) if r.iter().any(|&v| !v.is_multiple_of(p)) => return Verdict::Separates(c),
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
