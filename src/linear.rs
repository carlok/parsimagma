//! Tier S, linear models: `x ◇ y = a·x + b·y` over a ring.
//!
//! ETP paper section 5.2. In a linear magma every word `w(x_1..x_n)` is
//! itself linear,
//!
//! ```text
//!     w(x_1, ..., x_n) = Σ_i P_{w,i}(a, b) · x_i
//! ```
//!
//! where `P_{w,i}` sums one word in `{a, b}` per occurrence of `x_i`, namely
//! the root-to-leaf path to that occurrence (left = a, right = b, read
//! root-to-leaf, so the product is taken in that order and the construction
//! works verbatim in a noncommutative ring).
//!
//! Because the carrier is the whole ring and the ring is unital, setting one
//! variable to 1 and the rest to 0 shows that the law `w1 ≃ w2` holds in the
//! magma **iff** `P_{w1,i} = P_{w2,i}` in the ring for every `i`. So the
//! whole family reduces to deciding whether finitely many polynomials in the
//! coefficients vanish — no table, no carrier enumeration, and it works for
//! infinite rings where a table sweep is not available at all.
//!
//! The difference polynomials are computed once for the whole law list; an
//! instance then costs 31 ring multiplications plus a scan.

use crate::law::{Law, Term};
use crate::nc::{word_index, NcPoly, MAX_DEG, N_WORDS};
use crate::sig::Signature;

/// The difference polynomials of one law, one per variable.
#[derive(Clone, Debug)]
pub struct LawDiff {
    pub id: u32,
    pub arity: u8,
    /// `diffs[i] = P_{lhs,i} - P_{rhs,i}`.
    pub diffs: Vec<NcPoly>,
    /// `S_lhs - S_rhs`, where `S_w` sums the path word of every *internal*
    /// node of `w`. In an affine model `x ◇ y = ax + by + c` the constant
    /// part of a word is `S_w · c`, so the law constrains the coefficients by
    /// `(S_lhs - S_rhs) · c = 0` on top of the linear conditions.
    pub const_diff: NcPoly,
    /// `const_diff` as sparse terms.
    pub const_terms: Vec<(u8, i32)>,
    /// The same polynomials as `(word index, coefficient)` terms. A law of
    /// order 4 has at most five path words per side, so these run to a
    /// handful of entries against 31 dense slots, and evaluating an instance
    /// touches only the terms that exist.
    pub terms: Vec<Vec<(u8, i32)>>,
}

/// Difference polynomials for a whole law list. Built once, reused by every
/// instance of every linear family.
pub struct LinearLaws {
    pub laws: Vec<LawDiff>,
}

impl LinearLaws {
    pub fn build(laws: &[Law]) -> LinearLaws {
        let mut out = Vec::with_capacity(laws.len());
        for law in laws {
            let k = law.arity as usize;
            let mut lhs = vec![NcPoly::ZERO; k];
            let mut rhs = vec![NcPoly::ZERO; k];
            let mut s_lhs = NcPoly::ZERO;
            let mut s_rhs = NcPoly::ZERO;
            collect(&law.lhs, 0, 0, &mut lhs, &mut s_lhs);
            collect(&law.rhs, 0, 0, &mut rhs, &mut s_rhs);
            let const_diff = s_lhs.sub(&s_rhs);
            let const_terms: Vec<(u8, i32)> = const_diff
                .c
                .iter()
                .enumerate()
                .filter(|(_, &k)| k != 0)
                .map(|(w, &k)| (w as u8, k))
                .collect();
            let diffs: Vec<NcPoly> = (0..k).map(|i| lhs[i].sub(&rhs[i])).collect();
            let terms = diffs
                .iter()
                .map(|p| {
                    p.c.iter()
                        .enumerate()
                        .filter(|(_, &k)| k != 0)
                        .map(|(w, &k)| (w as u8, k))
                        .collect()
                })
                .collect();
            out.push(LawDiff {
                id: law.id,
                arity: law.arity,
                diffs,
                const_diff,
                const_terms,
                terms,
            });
        }
        LinearLaws { laws: out }
    }

    pub fn n_laws(&self) -> usize {
        self.laws.len()
    }
}

/// Accumulate the path word of every leaf into `out`, and of every internal
/// node into `internal`. `len`/`bits` describe the path from the root so far.
fn collect(t: &Term, len: u32, bits: u32, out: &mut [NcPoly], internal: &mut NcPoly) {
    match t {
        Term::Var(v) => out[*v as usize].add_word(len, bits, 1),
        Term::Op(l, r) => {
            debug_assert!(
                (len as usize) < MAX_DEG,
                "path deeper than a law of order 4"
            );
            internal.add_word(len, bits, 1);
            collect(l, len + 1, bits << 1, out, internal);
            collect(r, len + 1, (bits << 1) | 1, out, internal);
        }
    }
}

/// A unital ring, described by operations on an element type. Dispatch is on
/// the ring rather than the element so a runtime-chosen modulus or matrix
/// size costs nothing per operation.
pub trait RingOps {
    type Elem: Clone + PartialEq;

    fn zero(&self) -> Self::Elem;
    fn one(&self) -> Self::Elem;
    fn add_assign(&self, x: &mut Self::Elem, y: &Self::Elem);
    fn mul(&self, x: &Self::Elem, y: &Self::Elem) -> Self::Elem;
    /// `k` copies of `x`, for integer `k` of either sign.
    fn scale_add_assign(&self, acc: &mut Self::Elem, k: i32, x: &Self::Elem);
    fn is_zero(&self, x: &Self::Elem) -> bool;

    /// Cardinality of the carrier, when finite. Used to decide whether an
    /// instance can be cross-checked against the finite engine.
    fn carrier_size(&self) -> Option<usize> {
        None
    }
    fn name(&self) -> String;
}

/// A linear magma: a ring plus the two coefficients.
pub struct LinearModel<R: RingOps> {
    pub ring: R,
    pub a: R::Elem,
    pub b: R::Elem,
}

impl<R: RingOps> LinearModel<R> {
    pub fn new(ring: R, a: R::Elem, b: R::Elem) -> Self {
        LinearModel { ring, a, b }
    }

    /// Which laws this linear magma satisfies.
    pub fn signature(&self, ll: &LinearLaws) -> Signature {
        let words = word_values(&self.ring, &self.a, &self.b);
        let mut sig = Signature::zeros(ll.n_laws());
        let mut acc = self.ring.zero();
        for (i, law) in ll.laws.iter().enumerate() {
            if law
                .terms
                .iter()
                .all(|t| terms_are_zero(&self.ring, t, &words, &mut acc))
            {
                sig.set(i);
            }
        }
        sig
    }

    /// The multiplication table, when the carrier is small enough to write
    /// one down. This is what makes the tier cross-checkable: the same
    /// instance decided symbolically and by exhaustive sweep must agree bit
    /// for bit.
    pub fn table(&self, elements: &[R::Elem]) -> Vec<u8> {
        let n = elements.len();
        let mut t = vec![0u8; n * n];
        for (i, x) in elements.iter().enumerate() {
            for (j, y) in elements.iter().enumerate() {
                let mut v = self.ring.mul(&self.a, x);
                let by = self.ring.mul(&self.b, y);
                self.ring.add_assign(&mut v, &by);
                let k = elements
                    .iter()
                    .position(|e| *e == v)
                    .expect("product left the enumerated carrier");
                t[i * n + j] = k as u8;
            }
        }
        t
    }
}

/// Values of all 31 words in `{a, b}`, built by appending one letter at a
/// time so the product is taken root-to-leaf.
pub fn word_values<R: RingOps>(ring: &R, a: &R::Elem, b: &R::Elem) -> Vec<R::Elem> {
    let mut w = vec![ring.zero(); N_WORDS];
    w[0] = ring.one();
    for len in 0..MAX_DEG as u32 {
        for bits in 0..(1u32 << len) {
            let src = w[word_index(len, bits)].clone();
            w[word_index(len + 1, bits << 1)] = ring.mul(&src, a);
            w[word_index(len + 1, (bits << 1) | 1)] = ring.mul(&src, b);
        }
    }
    w
}

fn terms_are_zero<R: RingOps>(
    ring: &R,
    t: &[(u8, i32)],
    words: &[R::Elem],
    acc: &mut R::Elem,
) -> bool {
    *acc = ring.zero();
    for &(wi, k) in t {
        ring.scale_add_assign(acc, k, &words[wi as usize]);
    }
    ring.is_zero(acc)
}

/// An affine magma `x ◇ y = a·x + b·y + c`, ETP paper section 5.2. The linear
/// conditions are unchanged; the constant contributes exactly one more,
/// `(S_lhs - S_rhs)·c = 0`, which is why affine costs almost nothing on top.
pub struct AffineModel<R: RingOps> {
    pub ring: R,
    pub a: R::Elem,
    pub b: R::Elem,
    pub c: R::Elem,
}

impl<R: RingOps> AffineModel<R> {
    pub fn new(ring: R, a: R::Elem, b: R::Elem, c: R::Elem) -> Self {
        AffineModel { ring, a, b, c }
    }

    pub fn signature(&self, ll: &LinearLaws) -> Signature {
        let words = word_values(&self.ring, &self.a, &self.b);
        let mut sig = Signature::zeros(ll.n_laws());
        let mut acc = self.ring.zero();
        for (i, law) in ll.laws.iter().enumerate() {
            if !law
                .terms
                .iter()
                .all(|t| terms_are_zero(&self.ring, t, &words, &mut acc))
            {
                continue;
            }
            acc = self.ring.zero();
            for &(wi, k) in &law.const_terms {
                self.ring.scale_add_assign(&mut acc, k, &words[wi as usize]);
            }
            let shifted = self.ring.mul(&acc, &self.c);
            if self.ring.is_zero(&shifted) {
                sig.set(i);
            }
        }
        sig
    }

    /// Multiplication table, for the finite instances that can be swept.
    pub fn table(&self, elements: &[R::Elem]) -> Vec<u8> {
        let n = elements.len();
        let mut t = vec![0u8; n * n];
        for (i, x) in elements.iter().enumerate() {
            for (j, y) in elements.iter().enumerate() {
                let mut v = self.ring.mul(&self.a, x);
                let by = self.ring.mul(&self.b, y);
                self.ring.add_assign(&mut v, &by);
                self.ring.add_assign(&mut v, &self.c);
                let k = elements
                    .iter()
                    .position(|e| *e == v)
                    .expect("left the carrier");
                t[i * n + j] = k as u8;
            }
        }
        t
    }
}
