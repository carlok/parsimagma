//! Quadratic magmas over `Z/N` (ETP paper Remark 5.5).
//!
//! ```text
//!     x ◇ y = a x² + b x y + c y² + d x + e y + f   (mod N)
//! ```
//!
//! The paper notes these gave "somewhat useful" additional finite refutations
//! for small `N`, and warns that they become rare as `N` grows because the
//! polynomial attached to a word has degree exponential in the word's order.
//! The reason to sweep them anyway is the domain-size cliff: carrier 11 to 32
//! is where no search-based model finder reaches, so if the family has
//! anything to give there, nobody has looked.
//!
//! Unlike the linear family this does not reduce to coefficient identities —
//! a word of order 4 reaches degree 16 — so instances are decided by building
//! the table and sweeping, which caps the practical carrier size.

use crate::finite::FiniteMagma;

#[derive(Clone, Copy, Debug)]
pub struct Quadratic {
    pub n: u32,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
    pub e: u32,
    pub f: u32,
}

impl Quadratic {
    /// Decode the six coefficients from a single index in `0..N^6`.
    pub fn from_code(n: u32, mut code: u64) -> Self {
        let m = n as u64;
        let mut next = || {
            let v = (code % m) as u32;
            code /= m;
            v
        };
        Quadratic {
            n,
            a: next(),
            b: next(),
            c: next(),
            d: next(),
            e: next(),
            f: next(),
        }
    }

    pub fn grid_size(n: u32) -> u64 {
        (n as u64).pow(6)
    }

    #[inline]
    pub fn op(&self, x: u32, y: u32) -> u32 {
        let n = self.n;
        (self.a * x % n * x
            + self.b * x % n * y
            + self.c * y % n * y
            + self.d * x
            + self.e * y
            + self.f)
            % n
    }

    pub fn fill(&self, table: &mut [u8]) {
        let n = self.n as usize;
        for x in 0..n {
            for y in 0..n {
                table[x * n + y] = self.op(x as u32, y as u32) as u8;
            }
        }
    }

    pub fn magma(&self) -> FiniteMagma {
        let n = self.n as usize;
        let mut t = vec![0u8; n * n];
        self.fill(&mut t);
        FiniteMagma::new(n, t).expect("table is well formed by construction")
    }

    /// Is this instance linear, i.e. does it lie in the already-swept family?
    pub fn is_linear(&self) -> bool {
        self.a == 0 && self.b == 0 && self.c == 0 && self.f == 0
    }
}
