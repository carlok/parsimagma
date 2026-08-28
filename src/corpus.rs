//! The construction corpus: one row of the coverage matrix per *instance*.
//!
//! Coverage is defined over an explicit parameter grid, not over a family. A
//! linear model with free coefficients is an unbounded parametrised set, so
//! "the coverage of the linear family" has no value. Every grid below is
//! finite, stated in the output, and every total is reported against it.

use crate::finite::FiniteMagma;
use crate::law::Law;
use crate::linear::{AffineModel, LinearLaws, LinearModel, RingOps};
use crate::rings::{FreeComm, FreeNc, Integers, MatFp, OneSidedInverse, PolyZ, Zmod};
use crate::sig::Signature;
use crate::twist::TwistedPower;
use rayon::prelude::*;
use rustc_hash::FxHashSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Carrier {
    Finite(usize),
    Infinite,
}

impl Carrier {
    /// Whether a table sweep is a practical alternative to deciding this
    /// instance symbolically. Twenty-two laws use six variables, so a sweep
    /// costs `n^6` assignments; at `n = 16` that is 16.7 million and takes
    /// seconds, at `n = 81` it is 2.8e11 and takes years.
    pub fn table_checkable(&self) -> bool {
        matches!(self, Carrier::Finite(n) if *n <= 16)
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub family: &'static str,
    pub params: String,
    pub carrier: Carrier,
    pub sig: Signature,
}

/// One line of the stated grid, for the output header.
#[derive(Clone, Debug)]
pub struct GridSpec {
    pub family: &'static str,
    pub description: String,
    pub enumerated: usize,
    pub distinct: usize,
}

#[derive(Debug, Default)]
pub struct Corpus {
    pub instances: Vec<Instance>,
    pub grid: Vec<GridSpec>,
}

impl Corpus {
    /// Add every instance of one family, keeping only those whose signature
    /// is new. Identical signatures are identical rows of the coverage
    /// matrix, so deduplicating on the signature subsumes deduplicating up to
    /// isomorphism: isomorphic magmas satisfy the same laws, and two magmas
    /// with the same signature are interchangeable for every coverage
    /// question even when they are not isomorphic.
    fn add_family(
        &mut self,
        family: &'static str,
        description: String,
        it: Vec<(String, Carrier, Signature)>,
    ) {
        let mut seen: FxHashSet<Vec<u64>> = self
            .instances
            .iter()
            .map(|i| i.sig.words().to_vec())
            .collect();
        let mut enumerated = 0usize;
        let mut distinct = 0usize;
        for (params, carrier, sig) in it {
            enumerated += 1;
            // A magma satisfying nothing, or everything, separates nothing.
            if !sig.is_separating() {
                continue;
            }
            if seen.insert(sig.words().to_vec()) {
                distinct += 1;
                self.instances.push(Instance {
                    family,
                    params,
                    carrier,
                    sig,
                });
            }
        }
        self.grid.push(GridSpec {
            family,
            description,
            enumerated,
            distinct,
        });
    }
}

/// Upper bound on the modulus swept for `Z/m` linear models.
pub const ZMOD_MAX: u64 = 32;
/// Upper bound on the modulus swept for affine models, whose grid is cubic
/// in the modulus rather than quadratic.
pub const AFFINE_ZMOD_MAX: u64 = 20;
/// Coefficients swept for linear models over `Z`.
pub const INT_RANGE: i128 = 6;
/// Coefficient range for each of `c0`, `c1` in `c0 + c1 t` over `Z[t]`.
pub const POLY_RANGE: i128 = 2;

/// Build the linear and affine corpus over the stated grid.
///
/// ETP paper section 5.2. Remark 5.4 there matters for reading the result:
/// a commutative linear counterexample can always be realised in some
/// `Z/pZ`, so the commutative half of this family is exactly the part a
/// finite model builder can already find *in principle*. Whether a finite
/// model builder finds it *in practice* is a separate question, and the
/// coverage numbers answer it.
pub fn linear_corpus(ll: &LinearLaws) -> Corpus {
    let mut c = Corpus::default();

    c.add_family(
        "linear/Z_m",
        format!("x ◇ y = ax + by over Z/m, m = 2..{ZMOD_MAX}, all (a,b) in (Z/m)^2"),
        (2..=ZMOD_MAX)
            .flat_map(|m| (0..m).flat_map(move |a| (0..m).map(move |b| (m, a, b))))
            .map(|(m, a, b)| {
                (
                    format!("m={m} a={a} b={b}"),
                    Carrier::Finite(m as usize),
                    LinearModel::new(Zmod { m }, a, b).signature(ll),
                )
            })
            .collect(),
    );

    c.add_family(
        "affine/Z_m",
        format!("x ◇ y = ax + by + c over Z/m, m = 2..{AFFINE_ZMOD_MAX}, all (a,b,c) in (Z/m)^3"),
        (2..=AFFINE_ZMOD_MAX)
            .flat_map(|m| {
                (0..m).flat_map(move |a| {
                    (0..m).flat_map(move |b| (1..m).map(move |cc| (m, a, b, cc)))
                })
            })
            .map(|(m, a, b, cc)| {
                (
                    format!("m={m} a={a} b={b} c={cc}"),
                    Carrier::Finite(m as usize),
                    AffineModel::new(Zmod { m }, a, b, cc).signature(ll),
                )
            })
            .collect(),
    );

    c.add_family(
        "linear/Z",
        format!("x ◇ y = ax + by over Z, a,b in [-{INT_RANGE},{INT_RANGE}]"),
        (-INT_RANGE..=INT_RANGE)
            .flat_map(|a| (-INT_RANGE..=INT_RANGE).map(move |b| (a, b)))
            .map(|(a, b)| {
                (
                    format!("a={a} b={b}"),
                    Carrier::Infinite,
                    LinearModel::new(Integers, a, b).signature(ll),
                )
            })
            .collect(),
    );

    c.add_family(
        "affine/Z",
        format!("x ◇ y = ax + by + c over Z, a,b,c in [-{INT_RANGE},{INT_RANGE}], c != 0"),
        (-INT_RANGE..=INT_RANGE)
            .flat_map(|a| {
                (-INT_RANGE..=INT_RANGE).flat_map(move |b| {
                    (-INT_RANGE..=INT_RANGE)
                        .filter(|&cc| cc != 0)
                        .map(move |cc| (a, b, cc))
                })
            })
            .map(|(a, b, cc)| {
                (
                    format!("a={a} b={b} c={cc}"),
                    Carrier::Infinite,
                    AffineModel::new(Integers, a, b, cc).signature(ll),
                )
            })
            .collect(),
    );

    c.add_family(
        "linear/Z[t]",
        format!(
            "x ◇ y = ax + by over Z[t], a,b = c0 + c1 t with c0,c1 in [-{POLY_RANGE},{POLY_RANGE}]"
        ),
        (-POLY_RANGE..=POLY_RANGE)
            .flat_map(|a0| {
                (-POLY_RANGE..=POLY_RANGE).flat_map(move |a1| {
                    (-POLY_RANGE..=POLY_RANGE).flat_map(move |b0| {
                        (-POLY_RANGE..=POLY_RANGE).map(move |b1| (a0, a1, b0, b1))
                    })
                })
            })
            .map(|(a0, a1, b0, b1)| {
                (
                    format!("a={a0}+{a1}t b={b0}+{b1}t"),
                    Carrier::Infinite,
                    LinearModel::new(PolyZ, PolyZ::lin(a0, a1), PolyZ::lin(b0, b1)).signature(ll),
                )
            })
            .collect(),
    );

    for p in [2u64, 3] {
        let r = MatFp { p, k: 2 };
        let size = r.carrier_size().unwrap();
        let all = enumerate_matrices(p, 2);
        let name = if p == 2 {
            "linear/M2(F2)"
        } else {
            "linear/M2(F3)"
        };
        let mut rows = Vec::with_capacity(all.len() * all.len());
        for a in &all {
            for b in &all {
                rows.push((
                    format!("a={a:?} b={b:?}"),
                    Carrier::Finite(size),
                    LinearModel::new(r.clone(), a.clone(), b.clone()).signature(ll),
                ));
            }
        }
        c.add_family(
            name,
            format!("x ◇ y = ax + by over M_2(F_{p}), all (a,b); carrier has {size} elements"),
            rows,
        );
    }

    // Affine over M_2(F_2) only: the F_3 grid would be 81^3 = 531441 rows for
    // very little, since the constant adds one condition rather than a
    // dimension of freedom.
    {
        let r = MatFp { p: 2, k: 2 };
        let all = enumerate_matrices(2, 2);
        let mut rows = Vec::new();
        for a in &all {
            for b in &all {
                for cc in all.iter().skip(1) {
                    rows.push((
                        format!("a={a:?} b={b:?} c={cc:?}"),
                        Carrier::Finite(16),
                        AffineModel::new(r.clone(), a.clone(), b.clone(), cc.clone()).signature(ll),
                    ));
                }
            }
        }
        c.add_family(
            "affine/M2(F2)",
            "x ◇ y = ax + by + c over M_2(F_2), all (a,b,c) with c nonzero".to_string(),
            rows,
        );
    }

    c.add_family(
        "linear/generic",
        "the generic linear magmas: Z[a,b] and Z<a,b> with a,b the generators".to_string(),
        vec![
            (
                "Z[a,b]".to_string(),
                Carrier::Infinite,
                LinearModel::new(FreeComm, FreeComm.gen_a(), FreeComm.gen_b()).signature(ll),
            ),
            (
                "Z<a,b>".to_string(),
                Carrier::Infinite,
                LinearModel::new(FreeNc, FreeNc.gen_a(), FreeNc.gen_b()).signature(ll),
            ),
        ],
    );

    c.add_family(
        "linear/one-sided",
        "x ◇ y = ax + by over Z<a,b>/(ba+1), the shift-operator ring of paper \
         Example 5.3; b is a one-sided inverse of a and nothing more"
            .to_string(),
        vec![(
            "a,b generators".to_string(),
            Carrier::Infinite,
            LinearModel::new(
                OneSidedInverse,
                OneSidedInverse.gen_a(),
                OneSidedInverse.gen_b(),
            )
            .signature(ll),
        )],
    );

    c
}

fn enumerate_matrices(p: u64, k: usize) -> Vec<Vec<u64>> {
    let cells = k * k;
    let total = (p as usize).pow(cells as u32);
    (0..total)
        .map(|mut code| {
            (0..cells)
                .map(|_| {
                    let d = (code % p as usize) as u64;
                    code /= p as usize;
                    d
                })
                .collect()
        })
        .collect()
}

/// Coordinate count swept for twisted Cartesian powers. `Twist_{E1485}` is
/// cyclic of order 5, so anything short of `k = 5` cannot reproduce the
/// paper's own worked example; the grid runs to 8 to leave room for twisting
/// semigroups of larger order.
pub const TWIST_MAX_K: usize = 8;

/// Add twisted Cartesian powers (paper section 5.4) to a corpus.
///
/// Base magmas are every magma on two elements and every magma on three
/// elements up to isomorphism; twists are the cyclic shifts `i -> i + s` and
/// `i -> i + t`, which are the endomorphisms of `M^k` the construction always
/// supplies. Order-3 bases are capped at `k = 4` because the per-law cost is
/// `k · n^(leaves)` and `3^6` is eleven times `2^6`.
pub fn add_twist_family(c: &mut Corpus, laws: &[Law], order3_bases: &[FiniteMagma]) {
    let two: Vec<FiniteMagma> = (0..16u32)
        .map(|bits| FiniteMagma::new(2, (0..4).map(|i| ((bits >> i) & 1) as u8).collect()).unwrap())
        .collect();

    let mut jobs: Vec<(usize, usize, usize, usize)> = Vec::new();
    for (bi, _) in two.iter().enumerate() {
        for k in 2..=TWIST_MAX_K {
            for s in 0..k {
                for t in 0..k {
                    jobs.push((bi, k, s, t));
                }
            }
        }
    }
    let rows: Vec<(String, Carrier, Signature)> = jobs
        .par_iter()
        .map(|&(bi, k, s, t)| {
            let tw = TwistedPower::cyclic(two[bi].clone(), k, s, t);
            (
                format!("base2#{bi} k={k} shifts=({s},{t})"),
                Carrier::Finite(tw.carrier_size()),
                tw.signature(laws),
            )
        })
        .collect();
    c.add_family(
        "twist/M2^k",
        format!(
            "x ◇' y = x[i+s] ◇ y[i+t] on M^k for every 2-element base M,              k = 2..{TWIST_MAX_K}, all cyclic shift pairs (paper §5.4)"
        ),
        rows,
    );

    if order3_bases.is_empty() {
        return;
    }
    let mut jobs3: Vec<(usize, usize, usize, usize)> = Vec::new();
    for bi in 0..order3_bases.len() {
        for k in 2..=4usize {
            for s in 0..k {
                for t in 0..k {
                    jobs3.push((bi, k, s, t));
                }
            }
        }
    }
    let rows3: Vec<(String, Carrier, Signature)> = jobs3
        .par_iter()
        .map(|&(bi, k, s, t)| {
            let tw = TwistedPower::cyclic(order3_bases[bi].clone(), k, s, t);
            (
                format!("base3#{bi} k={k} shifts=({s},{t})"),
                Carrier::Finite(tw.carrier_size()),
                tw.signature(laws),
            )
        })
        .collect();
    c.add_family(
        "twist/M3^k",
        format!(
            "the same over 3-element bases up to isomorphism, k = 2..4 \
             ({} bases)",
            order3_bases.len()
        ),
        rows3,
    );
}

/// Every magma on three elements that is the least member of its isomorphism
/// class, for use as a twist base. There are 3330 of them (OEIS A001329).
pub fn order3_canonical() -> Vec<FiniteMagma> {
    use crate::finite::{is_canonical, permutations};
    let perms = permutations(3);
    let mut table = vec![0u8; 9];
    let mut buf = vec![0u8; 9];
    let mut out = Vec::new();
    for code in 0..3u64.pow(9) {
        let mut v = code;
        for cell in table.iter_mut() {
            *cell = (v % 3) as u8;
            v /= 3;
        }
        if is_canonical(&table, 3, &perms, &mut buf) {
            out.push(FiniteMagma::new(3, table.clone()).unwrap());
        }
    }
    out
}
