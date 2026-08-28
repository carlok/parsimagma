//! Translation-invariant magmas (ETP paper section 5.3).
//!
//! On an abelian group `M`, a magma whose left translations are automorphisms
//! takes the form
//!
//! ```text
//!     x ◇ y = x + f(y - x)
//! ```
//!
//! for some `f: M -> M`, so the whole family over `Z/n` is parametrised by the
//! `n^n` functions `f`. That is small enough to sweep exhaustively for `n` up
//! to 8 and, with early exit, up to 9.
//!
//! The family matters because of where it sits. The domain-size cliff measured
//! in `docs/atp-control.md` says no search-based model finder reaches carrier
//! 11 or above on these problems, so finite models in that range are
//! effectively unsearched territory — and this family is cheap to enumerate
//! there while remaining a *stated* finite grid rather than an ad hoc
//! construction.

use crate::finite::FiniteMagma;

/// `x ◇ y = x + f(y - x)` over `Z/n`, with `f` given as a table.
#[derive(Clone, Debug)]
pub struct TranslationInvariant {
    pub n: usize,
    pub f: Vec<u8>,
}

impl TranslationInvariant {
    pub fn new(n: usize, f: Vec<u8>) -> Self {
        debug_assert_eq!(f.len(), n);
        TranslationInvariant { n, f }
    }

    /// Row-major multiplication table.
    pub fn table(&self) -> Vec<u8> {
        let n = self.n;
        let mut t = vec![0u8; n * n];
        for x in 0..n {
            for y in 0..n {
                // (y - x) mod n, then x + f(that), mod n.
                let d = (y + n - x) % n;
                t[x * n + y] = ((x + self.f[d] as usize) % n) as u8;
            }
        }
        t
    }

    /// Fill `table` in place, so a sweep over `n^n` functions allocates once.
    pub fn fill(&self, table: &mut [u8]) {
        let n = self.n;
        for x in 0..n {
            for y in 0..n {
                let d = (y + n - x) % n;
                table[x * n + y] = ((x + self.f[d] as usize) % n) as u8;
            }
        }
    }

    pub fn magma(&self) -> FiniteMagma {
        FiniteMagma::new(self.n, self.table()).expect("table is well formed by construction")
    }

    /// Number of functions `f` over a carrier of size `n`.
    pub fn grid_size(n: usize) -> u64 {
        (n as u64).pow(n as u32)
    }
}

/// Decode `index` into the permutation of `0..n` it names, by Lehmer code.
///
/// Sweeping permutations rather than all `n^n` functions is what lets this
/// family reach the carrier range that matters. `n^n` at `n = 11` is 2.85e11
/// and hopeless; `11!` is 3.99e7 and takes minutes. The restriction is not a
/// convenience: a law of the form `x = w(x, y, ...)` with `x` occurring once
/// on the right forces the relevant translations to be surjective, and for a
/// translation-invariant magma the left translation `y ↦ x + f(y - x)` is
/// bijective exactly when `f` is.
pub fn permutation(n: usize, mut index: u64, out: &mut Vec<u8>) {
    out.clear();
    let mut pool: Vec<u8> = (0..n as u8).collect();
    let mut fact: u64 = (1..=n as u64).product::<u64>();
    for k in 0..n {
        fact /= (n - k) as u64;
        let i = (index / fact) as usize;
        index %= fact;
        out.push(pool.remove(i));
    }
}

pub fn factorial(n: usize) -> u64 {
    (1..=n as u64).product()
}
