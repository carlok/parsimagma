//! Noncommutative polynomials in two indeterminates, truncated at degree 4.
//!
//! A word in `{a, b}` of length at most 4 is all that a law of order at most
//! 4 can produce: the coefficient of a variable occurrence in a linear magma
//! is the root-to-leaf path to that occurrence, and the path is at most as
//! long as the term is deep. There are `2^0 + ... + 2^4 = 31` such words, so
//! a polynomial is a fixed array and every operation is branch-free.

/// Longest root-to-leaf path in a term of order at most 4.
pub const MAX_DEG: usize = 4;
/// Number of words in `{a, b}` of length at most `MAX_DEG`.
pub const N_WORDS: usize = (1 << (MAX_DEG + 1)) - 1;

/// Index of the word of length `len` whose letters, read root-to-leaf, are
/// the bits of `bits` from most to least significant (`0` = a, `1` = b).
#[inline]
pub const fn word_index(len: u32, bits: u32) -> usize {
    ((1u32 << len) - 1 + bits) as usize
}

/// `(len, bits)` for a word index.
pub const fn word_of(index: usize) -> (u32, u32) {
    let mut len = 0u32;
    let mut base = 0u32;
    while len <= MAX_DEG as u32 {
        let width = 1u32 << len;
        if (index as u32) < base + width {
            return (len, index as u32 - base);
        }
        base += width;
        len += 1;
    }
    panic!("word index out of range")
}

/// Degrees `(deg_a, deg_b)` of a word, for use in commutative rings.
pub const fn word_degrees(index: usize) -> (u32, u32) {
    let (len, bits) = word_of(index);
    let db = (bits as u32).count_ones();
    (len - db, db)
}

/// A truncated noncommutative polynomial with integer coefficients.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NcPoly {
    pub c: [i32; N_WORDS],
}

impl NcPoly {
    pub const ZERO: NcPoly = NcPoly { c: [0; N_WORDS] };

    #[inline]
    pub fn add_word(&mut self, len: u32, bits: u32, k: i32) {
        self.c[word_index(len, bits)] += k;
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.c.iter().all(|&x| x == 0)
    }

    pub fn sub(&self, other: &NcPoly) -> NcPoly {
        let mut out = NcPoly::ZERO;
        for i in 0..N_WORDS {
            out.c[i] = self.c[i] - other.c[i];
        }
        out
    }

    /// Image in the commutative quotient: 15 monomials `a^i b^j`, `i+j <= 4`.
    /// Zero here but not in `self` is exactly the gap between a commutative
    /// and a noncommutative linear model.
    pub fn commutative_image(&self) -> [i32; 15] {
        let mut out = [0i32; 15];
        for (i, &k) in self.c.iter().enumerate() {
            if k != 0 {
                let (da, db) = word_degrees(i);
                out[monomial_index(da, db)] += k;
            }
        }
        out
    }
}

/// Index of `a^da b^db` among monomials of total degree at most 4.
#[inline]
pub const fn monomial_index(da: u32, db: u32) -> usize {
    let d = (da + db) as usize;
    // Monomials of total degree < d come first: 1 + 2 + ... + d of them.
    d * (d + 1) / 2 + db as usize
}
