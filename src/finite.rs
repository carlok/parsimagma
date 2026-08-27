//! Finite-magma signature engine (Tier F, scalar path).
//!
//! For a magma of size `n`, the signature is computed by sweeping variable
//! assignments against the shared subterm DAG. Refuting a law costs one bad
//! assignment; confirming it costs the full sweep, so the sweep visits
//! assignments in a stride order rather than lexicographic order, which
//! surfaces violations early for the many laws whose counterexamples cluster
//! at low-index assignments.

use crate::dag::{Dag, EvalPlan, Node, MAX_VARS};
use crate::sig::Signature;

/// A finite magma with carrier `{0, ..., n-1}` and `table[x * n + y] = x ◇ y`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FiniteMagma {
    pub n: usize,
    pub table: Vec<u8>,
}

#[derive(Debug)]
pub struct BadTable(pub String);

impl FiniteMagma {
    pub fn new(n: usize, table: Vec<u8>) -> Result<Self, BadTable> {
        if n == 0 || n > 255 {
            return Err(BadTable(format!("carrier size {n} out of range 1..=255")));
        }
        if table.len() != n * n {
            return Err(BadTable(format!(
                "table has {} entries, expected {}",
                table.len(),
                n * n
            )));
        }
        if let Some(&bad) = table.iter().find(|&&v| v as usize >= n) {
            return Err(BadTable(format!("entry {bad} outside carrier of size {n}")));
        }
        Ok(FiniteMagma { n, table })
    }

    /// Build from a row-major nested list, the form used by the ETP data
    /// files (`Table [[0, 0], [1, 0]]`).
    pub fn from_rows(rows: &[Vec<u8>]) -> Result<Self, BadTable> {
        let n = rows.len();
        let mut table = Vec::with_capacity(n * n);
        for r in rows {
            if r.len() != n {
                return Err(BadTable(format!(
                    "row of length {} in a {n}-element table",
                    r.len()
                )));
            }
            table.extend_from_slice(r);
        }
        FiniteMagma::new(n, table)
    }

    #[inline]
    pub fn op(&self, x: u8, y: u8) -> u8 {
        self.table[x as usize * self.n + y as usize]
    }
}

/// Reusable scratch space, so a survey over many magmas allocates once.
pub struct Scratch {
    vals: Vec<u8>,
    live: Vec<bool>,
    digits: [u8; MAX_VARS],
}

impl Scratch {
    pub fn new(dag: &Dag) -> Self {
        Scratch {
            vals: vec![0u8; dag.nodes.len()],
            live: vec![false; dag.laws.len()],
            digits: [0u8; MAX_VARS],
        }
    }
}

pub struct Engine {
    pub dag: Dag,
    pub plan: EvalPlan,
}

impl Engine {
    pub fn new(dag: Dag) -> Self {
        let plan = EvalPlan::build(&dag);
        Engine { dag, plan }
    }

    pub fn n_laws(&self) -> usize {
        self.dag.n_laws()
    }

    /// Number of DAG node evaluations an exhaustive signature costs, as an
    /// upper bound ignoring early exit. Callers use this to decide whether a
    /// magma is small enough to sweep.
    pub fn cost(&self, n: usize) -> u128 {
        self.plan
            .buckets
            .iter()
            .map(|b| (n as u128).pow(b.arity as u32) * b.order.len() as u128)
            .sum()
    }

    pub fn signature(&self, m: &FiniteMagma) -> Signature {
        let mut scratch = Scratch::new(&self.dag);
        self.signature_with(m, &mut scratch)
    }

    /// The full 4694-bit signature of `m`.
    pub fn signature_with(&self, m: &FiniteMagma, s: &mut Scratch) -> Signature {
        let n = m.n;
        let mut sig = Signature::zeros(self.dag.laws.len());

        for bucket in &self.plan.buckets {
            let total: u64 = (n as u64).pow(bucket.arity as u32);
            assert!(
                total <= u32::MAX as u64,
                "carrier size {n} with {} variables needs {total} assignments; \
                 too large for an exhaustive sweep",
                bucket.arity
            );

            for &li in &bucket.laws {
                s.live[li as usize] = true;
            }
            let mut n_live = bucket.laws.len();
            let stride = coprime_stride(total, n as u64);

            let mut acc: u64 = 0;
            for _ in 0..total {
                let idx = acc;
                acc += stride;
                if acc >= total {
                    acc -= total;
                }

                let mut rem = idx;
                for d in s.digits.iter_mut().take(bucket.arity as usize) {
                    *d = (rem % n as u64) as u8;
                    rem /= n as u64;
                }
                s.vals[..bucket.arity as usize].copy_from_slice(&s.digits[..bucket.arity as usize]);

                for &nid in &bucket.order {
                    let Node::Op(a, b) = self.dag.nodes[nid as usize] else {
                        unreachable!("bucket order holds only application nodes")
                    };
                    let x = s.vals[a as usize] as usize;
                    let y = s.vals[b as usize] as usize;
                    s.vals[nid as usize] = m.table[x * n + y];
                }

                for &li in &bucket.laws {
                    if !s.live[li as usize] {
                        continue;
                    }
                    let law = &self.dag.laws[li as usize];
                    if s.vals[law.lhs as usize] != s.vals[law.rhs as usize] {
                        s.live[li as usize] = false;
                        n_live -= 1;
                    }
                }
                if n_live == 0 {
                    break;
                }
            }

            for &li in &bucket.laws {
                if s.live[li as usize] {
                    sig.set(li as usize);
                    s.live[li as usize] = false;
                }
            }
        }
        sig
    }
}

/// A stride that walks all of `0..total` in an order uncorrelated with the
/// carrier's numbering. Any value coprime to `n` is coprime to `n^k` and so
/// generates the additive group; the golden-ratio starting point is
/// deterministic, which keeps runs reproducible without a seed.
fn coprime_stride(total: u64, n: u64) -> u64 {
    if total <= 2 {
        return 1;
    }
    let mut s = (total as f64 * 0.618_033_988_749_894_9) as u64;
    if s == 0 {
        s = 1;
    }
    while gcd(s, n) != 1 {
        s += 1;
        if s >= total {
            return 1;
        }
    }
    s
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Isomorphism handling for the exhaustive sweep.
///
/// Brute-force canonical form is `n!` per table and dies past roughly `n = 9`,
/// so this is only used at the sizes where exhaustive enumeration is
/// affordable in the first place. Above the cap, the corpus is deduplicated
/// on the signature instead, which is coarser than isomorphism but is exactly
/// the equivalence coverage cares about: two magmas with the same signature
/// discharge the same separations.
pub const EXACT_ISO_CAP: usize = 6;

/// All permutations of `0..n`, as a flat `n! x n` table.
pub fn permutations(n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur: Vec<u8> = (0..n as u8).collect();
    permute(&mut cur, 0, &mut out);
    out
}

fn permute(cur: &mut Vec<u8>, k: usize, out: &mut Vec<Vec<u8>>) {
    if k == cur.len() {
        out.push(cur.clone());
        return;
    }
    for i in k..cur.len() {
        cur.swap(k, i);
        permute(cur, k + 1, out);
        cur.swap(k, i);
    }
}

/// Is `table` the lexicographically smallest member of its isomorphism class?
///
/// Relabelling by `p` sends `x ◇ y = z` to `p(x) ◇ p(y) = p(z)`, so the
/// relabelled table is `t'[p(x)*n + p(y)] = p(t[x*n + y])`. Comparing against
/// every permutation costs `n! * n^2` byte operations, which at `n = 4` is
/// some four hundred — two orders of magnitude cheaper than computing a
/// signature, so filtering first turns a 4.3-billion-table sweep into a
/// 179-million-table one.
pub fn is_canonical(table: &[u8], n: usize, perms: &[Vec<u8>], buf: &mut [u8]) -> bool {
    debug_assert!(
        n <= EXACT_ISO_CAP,
        "exact canonicalisation capped at {EXACT_ISO_CAP}"
    );
    for p in perms {
        if p.iter().enumerate().all(|(i, &v)| i as u8 == v) {
            continue;
        }
        for x in 0..n {
            for y in 0..n {
                buf[p[x] as usize * n + p[y] as usize] = p[table[x * n + y] as usize];
            }
        }
        if buf[..n * n] < table[..n * n] {
            return false;
        }
    }
    true
}
