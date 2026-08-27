//! The ETP implication graph, as ground truth.
//!
//! `implications.bits` is a packed `4694 x 4694` bit matrix: bit `j` of row
//! `i` is set exactly when `E_(i+1) ⊧ E_(j+1)` holds for arbitrary magmas.
//! It is derived from Janota's exhaustive Vampire run (arXiv:2508.15856,
//! `data/2025-08-11-vampire.json.gz`), which proved every implication that
//! holds; the 1062 pairs it left undecided are all false, and are listed
//! separately in `hard_core.txt`.
//!
//! Two uses. As an oracle: every separation a signature claims must land on
//! a pair the graph records as false, and a claim against a true implication
//! is a blocking bug. As a target: coverage is measured against the 1062-pair
//! hard core.

use crate::sig::Signature;

pub struct ImplicationGraph {
    pub n: usize,
    row_bytes: usize,
    bits: Vec<u8>,
}

impl ImplicationGraph {
    pub fn from_bytes(n: usize, bits: Vec<u8>) -> Self {
        let row_bytes = n.div_ceil(8);
        assert_eq!(
            bits.len(),
            n * row_bytes,
            "implication matrix has wrong size"
        );
        ImplicationGraph { n, row_bytes, bits }
    }

    /// Does `E_(i+1) ⊧ E_(j+1)` hold? Indices are 0-based.
    #[inline]
    pub fn holds(&self, i: usize, j: usize) -> bool {
        self.bits[i * self.row_bytes + (j >> 3)] >> (j & 7) & 1 == 1
    }

    pub fn count_true(&self) -> u64 {
        self.bits.iter().map(|b| b.count_ones() as u64).sum()
    }

    /// Row `i` as a signature-shaped bit vector, so that set algebra against
    /// a magma signature is word-parallel.
    pub fn row(&self, i: usize) -> Signature {
        Signature::from_bytes(
            self.n,
            &self.bits[i * self.row_bytes..(i + 1) * self.row_bytes],
        )
    }
}

/// An ordered pair `(i, j)` denoting the implication `E_i ⊧ E_j`, 1-based.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Pair {
    pub from: u32,
    pub to: u32,
}

pub fn parse_pairs(text: &str) -> Result<Vec<Pair>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let from = it
            .next()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| format!("line {}: bad source", n + 1))?;
        let to = it
            .next()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| format!("line {}: bad target", n + 1))?;
        out.push(Pair { from, to });
    }
    Ok(out)
}

/// Does `sig` separate `E_from` from `E_to`? True when the magma satisfies
/// the hypothesis and refutes the conclusion.
#[inline]
pub fn separates(sig: &Signature, p: Pair) -> bool {
    sig.get(p.from as usize - 1) && !sig.get(p.to as usize - 1)
}
