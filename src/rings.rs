//! Concrete rings for linear models.
//!
//! Two of these have finite carriers (`Zmod`, `MatFp`), which is what makes
//! Tier S testable: the same instance decided symbolically must agree bit for
//! bit with an exhaustive table sweep. The rest are infinite, where no table
//! exists and the symbolic route is the only one.

use crate::linear::RingOps;
use crate::nc::{monomial_index, word_degrees, word_index, word_of, MAX_DEG, N_MONOMIALS, N_WORDS};

/// `Z/mZ`. Carrier size `m`, so instances are finite and cross-checkable.
#[derive(Clone, Debug)]
pub struct Zmod {
    pub m: u64,
}

impl RingOps for Zmod {
    type Elem = u64;

    fn zero(&self) -> u64 {
        0
    }
    fn one(&self) -> u64 {
        1 % self.m
    }
    fn add_assign(&self, x: &mut u64, y: &u64) {
        *x = (*x + *y) % self.m;
    }
    fn mul(&self, x: &u64, y: &u64) -> u64 {
        (*x * *y) % self.m
    }
    fn scale_add_assign(&self, acc: &mut u64, k: i32, x: &u64) {
        let km = k.rem_euclid(self.m as i32) as u64;
        *acc = (*acc + km * *x) % self.m;
    }
    fn is_zero(&self, x: &u64) -> bool {
        x.is_multiple_of(self.m)
    }
    fn carrier_size(&self) -> Option<usize> {
        Some(self.m as usize)
    }
    fn name(&self) -> String {
        format!("Z/{}", self.m)
    }
}

/// The integers. Infinite carrier; coefficients stay tiny because a law of
/// order 4 contributes at most five path words of degree at most four.
#[derive(Clone, Debug)]
pub struct Integers;

impl RingOps for Integers {
    type Elem = i128;

    fn zero(&self) -> i128 {
        0
    }
    fn one(&self) -> i128 {
        1
    }
    fn add_assign(&self, x: &mut i128, y: &i128) {
        *x += *y;
    }
    fn mul(&self, x: &i128, y: &i128) -> i128 {
        *x * *y
    }
    fn scale_add_assign(&self, acc: &mut i128, k: i32, x: &i128) {
        *acc += k as i128 * *x;
    }
    fn is_zero(&self, x: &i128) -> bool {
        *x == 0
    }
    fn name(&self) -> String {
        "Z".to_string()
    }
}

/// `Z[t]`, dense little-endian coefficient vectors. Infinite, commutative,
/// and the natural home for coefficient pairs that no finite ring realises.
#[derive(Clone, Debug)]
pub struct PolyZ;

impl PolyZ {
    pub fn con(k: i128) -> Vec<i128> {
        if k == 0 {
            vec![]
        } else {
            vec![k]
        }
    }
    /// `c0 + c1 t`.
    pub fn lin(c0: i128, c1: i128) -> Vec<i128> {
        let mut v = vec![c0, c1];
        while v.last() == Some(&0) {
            v.pop();
        }
        v
    }
}

impl RingOps for PolyZ {
    type Elem = Vec<i128>;

    fn zero(&self) -> Vec<i128> {
        vec![]
    }
    fn one(&self) -> Vec<i128> {
        vec![1]
    }
    fn add_assign(&self, x: &mut Vec<i128>, y: &Vec<i128>) {
        if y.len() > x.len() {
            x.resize(y.len(), 0);
        }
        for (i, c) in y.iter().enumerate() {
            x[i] += c;
        }
        while x.last() == Some(&0) {
            x.pop();
        }
    }
    fn mul(&self, x: &Vec<i128>, y: &Vec<i128>) -> Vec<i128> {
        if x.is_empty() || y.is_empty() {
            return vec![];
        }
        let mut out = vec![0i128; x.len() + y.len() - 1];
        for (i, a) in x.iter().enumerate() {
            for (j, b) in y.iter().enumerate() {
                out[i + j] += a * b;
            }
        }
        while out.last() == Some(&0) {
            out.pop();
        }
        out
    }
    fn scale_add_assign(&self, acc: &mut Vec<i128>, k: i32, x: &Vec<i128>) {
        if x.len() > acc.len() {
            acc.resize(x.len(), 0);
        }
        for (i, c) in x.iter().enumerate() {
            acc[i] += k as i128 * c;
        }
        while acc.last() == Some(&0) {
            acc.pop();
        }
    }
    fn is_zero(&self, x: &Vec<i128>) -> bool {
        x.iter().all(|&c| c == 0)
    }
    fn name(&self) -> String {
        "Z[t]".to_string()
    }
}

/// `k x k` matrices over `F_p`, the smallest genuinely noncommutative rings.
/// The carrier has `p^(k^2)` elements — 81 already for `M_2(F_3)` — which is
/// far past what a six-variable table sweep can reach, so these instances are
/// only decidable symbolically.
#[derive(Clone, Debug)]
pub struct MatFp {
    pub p: u64,
    pub k: usize,
}

impl MatFp {
    pub fn from_rows(&self, rows: &[&[u64]]) -> Vec<u64> {
        let mut v = Vec::with_capacity(self.k * self.k);
        for r in rows {
            assert_eq!(r.len(), self.k);
            v.extend(r.iter().map(|x| x % self.p));
        }
        v
    }
}

impl RingOps for MatFp {
    type Elem = Vec<u64>;

    fn zero(&self) -> Vec<u64> {
        vec![0; self.k * self.k]
    }
    fn one(&self) -> Vec<u64> {
        let mut m = vec![0; self.k * self.k];
        for i in 0..self.k {
            m[i * self.k + i] = 1 % self.p;
        }
        m
    }
    fn add_assign(&self, x: &mut Vec<u64>, y: &Vec<u64>) {
        for i in 0..x.len() {
            x[i] = (x[i] + y[i]) % self.p;
        }
    }
    fn mul(&self, x: &Vec<u64>, y: &Vec<u64>) -> Vec<u64> {
        let k = self.k;
        let mut out = vec![0u64; k * k];
        for i in 0..k {
            for l in 0..k {
                let a = x[i * k + l];
                if a == 0 {
                    continue;
                }
                for j in 0..k {
                    out[i * k + j] = (out[i * k + j] + a * y[l * k + j]) % self.p;
                }
            }
        }
        out
    }
    fn scale_add_assign(&self, acc: &mut Vec<u64>, kk: i32, x: &Vec<u64>) {
        let s = kk.rem_euclid(self.p as i32) as u64;
        for i in 0..acc.len() {
            acc[i] = (acc[i] + s * x[i]) % self.p;
        }
    }
    fn is_zero(&self, x: &Vec<u64>) -> bool {
        x.iter().all(|&c| c.is_multiple_of(self.p))
    }
    fn carrier_size(&self) -> Option<usize> {
        Some((self.p as usize).pow((self.k * self.k) as u32))
    }
    fn name(&self) -> String {
        format!("M_{}(F_{})", self.k, self.p)
    }
}

/// The free noncommutative ring `Z<a, b>` with `a`, `b` the generators
/// themselves. A law holds here exactly when its difference polynomials
/// vanish *identically*, so this instance is the generic noncommutative
/// linear magma: it satisfies precisely the laws every linear magma
/// satisfies, and it is the strongest single member of the family.
#[derive(Clone, Debug)]
pub struct FreeNc;

/// Truncated at total degree `MAX_DEG`; products that would exceed it cannot
/// arise from a law of order 4 and are rejected rather than silently dropped.
pub type NcElem = [i128; N_WORDS];

impl FreeNc {
    pub fn gen_a(&self) -> NcElem {
        let mut e = [0i128; N_WORDS];
        e[word_index(1, 0)] = 1;
        e
    }
    pub fn gen_b(&self) -> NcElem {
        let mut e = [0i128; N_WORDS];
        e[word_index(1, 1)] = 1;
        e
    }
}

impl RingOps for FreeNc {
    type Elem = NcElem;

    fn zero(&self) -> NcElem {
        [0; N_WORDS]
    }
    fn one(&self) -> NcElem {
        let mut e = [0i128; N_WORDS];
        e[0] = 1;
        e
    }
    fn add_assign(&self, x: &mut NcElem, y: &NcElem) {
        for i in 0..N_WORDS {
            x[i] += y[i];
        }
    }
    fn mul(&self, x: &NcElem, y: &NcElem) -> NcElem {
        let mut out = [0i128; N_WORDS];
        for i in 0..N_WORDS {
            if x[i] == 0 {
                continue;
            }
            let (li, bi) = word_of(i);
            for j in 0..N_WORDS {
                if y[j] == 0 {
                    continue;
                }
                let (lj, bj) = word_of(j);
                assert!(
                    (li + lj) as usize <= MAX_DEG,
                    "word of length {} exceeds the degree bound",
                    li + lj
                );
                out[word_index(li + lj, (bi << lj) | bj)] += x[i] * y[j];
            }
        }
        out
    }
    fn scale_add_assign(&self, acc: &mut NcElem, k: i32, x: &NcElem) {
        for i in 0..N_WORDS {
            acc[i] += k as i128 * x[i];
        }
    }
    fn is_zero(&self, x: &NcElem) -> bool {
        x.iter().all(|&c| c == 0)
    }
    fn name(&self) -> String {
        "Z<a,b> (generic noncommutative)".to_string()
    }
}

/// The free commutative ring `Z[a, b]`. The generic commutative linear magma.
/// Any law it satisfies but `FreeNc` does not is one whose linear proof needs
/// commutativity.
#[derive(Clone, Debug)]
pub struct FreeComm;

pub type CommElem = [i128; N_MONOMIALS];

impl FreeComm {
    pub fn gen_a(&self) -> CommElem {
        let mut e = [0i128; N_MONOMIALS];
        e[monomial_index(1, 0)] = 1;
        e
    }
    pub fn gen_b(&self) -> CommElem {
        let mut e = [0i128; N_MONOMIALS];
        e[monomial_index(0, 1)] = 1;
        e
    }
}

/// `(deg_a, deg_b)` for each of the 15 monomial slots.
fn monomial_degrees() -> [(u32, u32); N_MONOMIALS] {
    let mut out = [(0u32, 0u32); N_MONOMIALS];
    for d in 0..=MAX_DEG as u32 {
        for db in 0..=d {
            out[monomial_index(d - db, db)] = (d - db, db);
        }
    }
    out
}

impl RingOps for FreeComm {
    type Elem = CommElem;

    fn zero(&self) -> CommElem {
        [0; N_MONOMIALS]
    }
    fn one(&self) -> CommElem {
        let mut e = [0i128; N_MONOMIALS];
        e[monomial_index(0, 0)] = 1;
        e
    }
    fn add_assign(&self, x: &mut CommElem, y: &CommElem) {
        for i in 0..N_MONOMIALS {
            x[i] += y[i];
        }
    }
    fn mul(&self, x: &CommElem, y: &CommElem) -> CommElem {
        let degs = monomial_degrees();
        let mut out = [0i128; N_MONOMIALS];
        for i in 0..N_MONOMIALS {
            if x[i] == 0 {
                continue;
            }
            for j in 0..N_MONOMIALS {
                if y[j] == 0 {
                    continue;
                }
                let (ai, bi) = degs[i];
                let (aj, bj) = degs[j];
                assert!(
                    (ai + bi + aj + bj) as usize <= MAX_DEG,
                    "monomial degree exceeds the bound"
                );
                out[monomial_index(ai + aj, bi + bj)] += x[i] * y[j];
            }
        }
        out
    }
    fn scale_add_assign(&self, acc: &mut CommElem, k: i32, x: &CommElem) {
        for i in 0..N_MONOMIALS {
            acc[i] += k as i128 * x[i];
        }
    }
    fn is_zero(&self, x: &CommElem) -> bool {
        x.iter().all(|&c| c == 0)
    }
    fn name(&self) -> String {
        "Z[a,b] (generic commutative)".to_string()
    }
}

/// Kept for the doc link from `NcPoly::commutative_image`.
pub fn word_degrees_of(index: usize) -> (u32, u32) {
    word_degrees(index)
}

/// `Z<a, b> / (ba + 1)`: the free ring in which `b` is a one-sided inverse of
/// `a`, with `ab` left free. Every word reduces to a normal form `a^i b^j`,
/// since `ba -> -1` rewrites any descent.
///
/// This is the abstract shape of the shift operators used in ETP paper
/// Example 5.3: taking `a = L`, `b = -R` on integer sequences gives a
/// one-sided inverse and nothing more. It is the smallest ring that realises
/// the separation `E1117 ⊭ E2441`, and the paper's Remark 5.4 shows that
/// separation admits *no* finite counterexample at all — a Tier S instance
/// with no Tier F shadow.
#[derive(Clone, Debug)]
pub struct OneSidedInverse;

/// Coefficients of the normal forms `a^i b^j` with `i + j <= MAX_DEG`,
/// indexed by [`monomial_index`] just as the commutative monomials are.
pub type OsiElem = [i128; N_MONOMIALS];

impl OneSidedInverse {
    pub fn gen_a(&self) -> OsiElem {
        let mut e = [0i128; N_MONOMIALS];
        e[monomial_index(1, 0)] = 1;
        e
    }
    pub fn gen_b(&self) -> OsiElem {
        let mut e = [0i128; N_MONOMIALS];
        e[monomial_index(0, 1)] = 1;
        e
    }

    /// `(a^i b^j)(a^k b^l) = (-1)^m a^(i + (k-j)^+) b^((j-k)^+ + l)` where
    /// `m = min(j, k)`, obtained by cancelling `m` copies of `ba = -1` at the
    /// interface.
    fn mul_normal(i: u32, j: u32, k: u32, l: u32) -> (i128, u32, u32) {
        let m = j.min(k);
        let sign = if m % 2 == 0 { 1i128 } else { -1 };
        (sign, i + (k - m), (j - m) + l)
    }
}

impl RingOps for OneSidedInverse {
    type Elem = OsiElem;

    fn zero(&self) -> OsiElem {
        [0; N_MONOMIALS]
    }
    fn one(&self) -> OsiElem {
        let mut e = [0i128; N_MONOMIALS];
        e[monomial_index(0, 0)] = 1;
        e
    }
    fn add_assign(&self, x: &mut OsiElem, y: &OsiElem) {
        for n in 0..N_MONOMIALS {
            x[n] += y[n];
        }
    }
    fn mul(&self, x: &OsiElem, y: &OsiElem) -> OsiElem {
        let degs = normal_form_degrees();
        let mut out = [0i128; N_MONOMIALS];
        for (n, &xn) in x.iter().enumerate() {
            if xn == 0 {
                continue;
            }
            for (q, &yq) in y.iter().enumerate() {
                if yq == 0 {
                    continue;
                }
                let (i, j) = degs[n];
                let (k, l) = degs[q];
                let (sign, ni, nj) = OneSidedInverse::mul_normal(i, j, k, l);
                assert!(
                    (ni + nj) as usize <= MAX_DEG,
                    "normal form a^{ni} b^{nj} exceeds the degree bound"
                );
                out[monomial_index(ni, nj)] += sign * xn * yq;
            }
        }
        out
    }
    fn scale_add_assign(&self, acc: &mut OsiElem, k: i32, x: &OsiElem) {
        for n in 0..N_MONOMIALS {
            acc[n] += k as i128 * x[n];
        }
    }
    fn is_zero(&self, x: &OsiElem) -> bool {
        x.iter().all(|&c| c == 0)
    }
    fn name(&self) -> String {
        "Z<a,b>/(ba+1)".to_string()
    }
}

/// `(i, j)` exponents for each normal-form slot `a^i b^j`.
fn normal_form_degrees() -> [(u32, u32); N_MONOMIALS] {
    let mut out = [(0u32, 0u32); N_MONOMIALS];
    for d in 0..=MAX_DEG as u32 {
        for j in 0..=d {
            out[monomial_index(d - j, j)] = (d - j, j);
        }
    }
    out
}
