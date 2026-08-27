//! Twisted Cartesian powers (ETP paper section 5.4).
//!
//! Given a magma `M` satisfying a law `E` and two endomorphisms `T, U`, the
//! twisted operation `x ◇' y := Tx ◇ Uy` satisfies `E` again provided `T, U`
//! obey the relations defining the *twisting semigroup* `Twist_E`. A Cartesian
//! power `M^k` always supplies such endomorphisms as coordinate shifts, so the
//! construction is: take a small model of `E`, raise it to a power, twist by
//! two shifts, and the result satisfies `E` while typically failing any `E'`
//! whose twisting semigroup imposes relations the shifts do not obey.
//!
//! The paper's worked case is `E1485 ⊭ E151`, which it says "does not seem to
//! be easily refuted by any of the other methods discussed": `F_2` under NAND,
//! raised to the fifth power, twisted by the left and right shifts, because
//! `Twist_{E1485}` is cyclic of order 5 and `Twist_{E151}` is cyclic of
//! order 2.
//!
//! # Why this is not just a table sweep
//!
//! The twisted magma has `n^k` elements — 32 for the paper's example — and 22
//! of the laws use six variables, so a sweep would need `32^6 ≈ 1.1e9`
//! assignments. Instead the check decomposes by coordinate. Writing `σ`, `τ`
//! for the two shifts, the value of a term at coordinate `i` is
//!
//! ```text
//!     eval(Var v, i)      = x_v[i]
//!     eval(Op(l, r), i)   = eval(l, σ(i)) ◇_M eval(r, τ(i))
//! ```
//!
//! so at a fixed root coordinate the law reads at most six entries of the
//! input tuples — one per leaf — and the law holds in the twisted magma
//! exactly when the base identity holds at every coordinate for every
//! assignment to those entries. That is `k · n^(leaves)` base operations per
//! law rather than `n^(k·vars)`.

use crate::finite::FiniteMagma;
use crate::law::{Law, Term};
use crate::sig::Signature;

/// `(x ◇' y)[i] = base(x[sigma[i]], y[tau[i]])` on the carrier `M^k`.
#[derive(Clone, Debug)]
pub struct TwistedPower {
    pub base: FiniteMagma,
    pub k: usize,
    pub sigma: Vec<usize>,
    pub tau: Vec<usize>,
}

impl TwistedPower {
    /// Twist by the cyclic shifts `i -> i + s` and `i -> i + t` modulo `k`,
    /// which is the shape the paper uses and always gives endomorphisms of
    /// the Cartesian power.
    pub fn cyclic(base: FiniteMagma, k: usize, s: usize, t: usize) -> Self {
        TwistedPower {
            base,
            k,
            sigma: (0..k).map(|i| (i + s) % k).collect(),
            tau: (0..k).map(|i| (i + t) % k).collect(),
        }
    }

    pub fn carrier_size(&self) -> usize {
        self.base.n.pow(self.k as u32)
    }

    /// Collect the `(variable, coordinate)` slots a term reads when evaluated
    /// at root coordinate `i`.
    fn slots(&self, t: &Term, i: usize, out: &mut Vec<(u8, usize)>) {
        match t {
            Term::Var(v) => {
                if !out.contains(&(*v, i)) {
                    out.push((*v, i));
                }
            }
            Term::Op(l, r) => {
                self.slots(l, self.sigma[i], out);
                self.slots(r, self.tau[i], out);
            }
        }
    }

    fn eval(&self, t: &Term, i: usize, slots: &[(u8, usize)], vals: &[u8]) -> u8 {
        match t {
            Term::Var(v) => {
                let k = slots
                    .iter()
                    .position(|&s| s == (*v, i))
                    .expect("slot was collected");
                vals[k]
            }
            Term::Op(l, r) => {
                let a = self.eval(l, self.sigma[i], slots, vals);
                let b = self.eval(r, self.tau[i], slots, vals);
                self.base.op(a, b)
            }
        }
    }

    /// Does the twisted magma satisfy this law?
    pub fn satisfies(&self, law: &Law) -> bool {
        let n = self.base.n as u64;
        let mut slots = Vec::with_capacity(8);
        let mut vals = Vec::with_capacity(8);
        for i in 0..self.k {
            slots.clear();
            self.slots(&law.lhs, i, &mut slots);
            self.slots(&law.rhs, i, &mut slots);
            let total = n.pow(slots.len() as u32);
            vals.clear();
            vals.resize(slots.len(), 0);
            for code in 0..total {
                let mut v = code;
                for cell in vals.iter_mut() {
                    *cell = (v % n) as u8;
                    v /= n;
                }
                if self.eval(&law.lhs, i, &slots, &vals) != self.eval(&law.rhs, i, &slots, &vals) {
                    return false;
                }
            }
        }
        true
    }

    pub fn signature(&self, laws: &[Law]) -> Signature {
        let mut sig = Signature::zeros(laws.len());
        for (idx, law) in laws.iter().enumerate() {
            if self.satisfies(law) {
                sig.set(idx);
            }
        }
        sig
    }

    /// The explicit multiplication table, when the carrier is small enough to
    /// write one down. Elements are numbered by reading the coordinate tuple
    /// as a base-`n` numeral, least significant coordinate first.
    pub fn table(&self) -> Option<FiniteMagma> {
        let size = self.carrier_size();
        if size > 255 {
            return None;
        }
        let n = self.base.n;
        let decode = |mut code: usize| -> Vec<u8> {
            (0..self.k)
                .map(|_| {
                    let d = (code % n) as u8;
                    code /= n;
                    d
                })
                .collect()
        };
        let mut table = vec![0u8; size * size];
        for x in 0..size {
            let xs = decode(x);
            for y in 0..size {
                let ys = decode(y);
                let mut code = 0usize;
                for i in (0..self.k).rev() {
                    code = code * n + self.base.op(xs[self.sigma[i]], ys[self.tau[i]]) as usize;
                }
                table[x * size + y] = code as u8;
            }
        }
        FiniteMagma::new(size, table).ok()
    }
}

/// `F_2` under NAND, `x ◇ y = 1 - xy`: the base model of E1485 the paper uses.
pub fn nand_f2() -> FiniteMagma {
    FiniteMagma::new(2, vec![1, 1, 1, 0]).expect("2-element table")
}
