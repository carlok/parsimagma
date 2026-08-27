//! Coverage: which separations each construction instance discharges.
//!
//! The matrix has one row per instance and one column per target separation.
//! Two targets are reported: the 1062-pair hard core left undecided by
//! Vampire, which is the number the project is about, and the whole set of
//! 13,855,357 false implications, for context.

use crate::corpus::Corpus;
use crate::graph::{separates, ImplicationGraph, Pair};
use crate::sig::Signature;

pub struct CoverageMatrix {
    pub targets: Vec<Pair>,
    /// `rows[i]` lists the target indices instance `i` discharges. Rows that
    /// discharge nothing are kept so that row numbering matches the corpus.
    pub rows: Vec<Vec<u32>>,
}

impl CoverageMatrix {
    pub fn build(corpus: &Corpus, targets: &[Pair]) -> CoverageMatrix {
        let rows = corpus
            .instances
            .iter()
            .map(|inst| {
                targets
                    .iter()
                    .enumerate()
                    .filter(|(_, &p)| separates(&inst.sig, p))
                    .map(|(k, _)| k as u32)
                    .collect()
            })
            .collect();
        CoverageMatrix {
            targets: targets.to_vec(),
            rows,
        }
    }

    /// Target indices discharged by at least one instance.
    pub fn covered(&self) -> Vec<u32> {
        let mut hit = vec![false; self.targets.len()];
        for r in &self.rows {
            for &k in r {
                hit[k as usize] = true;
            }
        }
        (0..self.targets.len() as u32)
            .filter(|&k| hit[k as usize])
            .collect()
    }

    /// Greedy set cover over the covered targets. Returns the chosen rows in
    /// selection order with the number of newly covered targets each brought.
    ///
    /// This is an upper bound on the size of a smallest cover, within a
    /// factor of `ln(n)`. It is not a minimum, and nothing here should be
    /// read as one: pairing it with an LP lower bound to report the gap is
    /// Phase B work.
    pub fn greedy_cover(&self) -> Vec<(usize, usize)> {
        let mut remaining: Vec<bool> = vec![false; self.targets.len()];
        for r in &self.rows {
            for &k in r {
                remaining[k as usize] = true;
            }
        }
        let mut left = remaining.iter().filter(|&&b| b).count();
        let mut chosen = Vec::new();
        while left > 0 {
            let (best, gain) = self
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| (i, r.iter().filter(|&&k| remaining[k as usize]).count()))
                .max_by_key(|&(_, g)| g)
                .unwrap();
            if gain == 0 {
                break;
            }
            for &k in &self.rows[best] {
                if remaining[k as usize] {
                    remaining[k as usize] = false;
                    left -= 1;
                }
            }
            chosen.push((best, gain));
        }
        chosen
    }
}

/// Union coverage over the whole implication graph: which of the false
/// implications the corpus discharges, accumulated as a bit matrix.
pub struct GraphCoverage {
    n: usize,
    row_words: usize,
    bits: Vec<u64>,
}

impl GraphCoverage {
    pub fn new(n: usize) -> Self {
        let row_words = n.div_ceil(64);
        GraphCoverage {
            n,
            row_words,
            bits: vec![0u64; n * row_words],
        }
    }

    /// Fold in one signature: it discharges `E_i ⊧ E_j` for every satisfied
    /// `i` and refuted `j`, so each satisfied row takes the complement of
    /// the signature, word at a time.
    pub fn add(&mut self, sig: &Signature) {
        let w = sig.words();
        let last_bits = self.n % 64;
        for i in sig.iter_set() {
            let base = i * self.row_words;
            for (k, &word) in w.iter().enumerate().take(self.row_words) {
                let mut c = !word;
                if k == self.row_words - 1 && last_bits != 0 {
                    c &= (1u64 << last_bits) - 1;
                }
                self.bits[base + k] |= c;
            }
        }
    }

    /// Consume the accumulator, for merging partial results across threads.
    pub fn into_bits(self) -> Vec<u64> {
        self.bits
    }

    pub fn bits(&self) -> &[u64] {
        &self.bits
    }

    /// Merge another accumulator's bits in place.
    pub fn or_bits(&mut self, other: &[u64]) {
        for (a, b) in self.bits.iter_mut().zip(other) {
            *a |= *b;
        }
    }

    pub fn count(&self) -> u64 {
        self.bits.iter().map(|b| b.count_ones() as u64).sum()
    }

    /// Every covered pair must be a pair the ETP records as false. A covered
    /// pair that the graph says is true would mean the engine is wrong.
    pub fn check_against(&self, g: &ImplicationGraph) -> Result<(), (usize, usize)> {
        for i in 0..self.n {
            for j in 0..self.n {
                if self.bits[i * self.row_words + (j >> 6)] >> (j & 63) & 1 == 1 && g.holds(i, j) {
                    return Err((i + 1, j + 1));
                }
            }
        }
        Ok(())
    }
}
