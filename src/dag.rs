//! One shared subterm DAG across all 4694 laws.
//!
//! The laws contain a few thousand distinct subterms between them, so
//! evaluating each law independently re-does most of the work. Instead every
//! subterm is hash-consed once, evaluation walks the DAG bottom-up per
//! variable assignment, and a law becomes a comparison of two node values.
//!
//! The DAG is a flat array of `(op, lhs, rhs)` triples interpreted at
//! runtime, not generated code. That keeps the law list a runtime input:
//! pointing the engine at `eq_size5.txt` needs no recompile.

use crate::law::{Law, Term};
use rustc_hash::FxHashMap;

/// The maximum number of distinct variables in a law of order at most 4:
/// four operations give at most six leaves.
pub const MAX_VARS: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Node {
    Var(u8),
    /// Indices of the two operands. Both are strictly less than this node's
    /// own index, so ascending node order is a topological order.
    Op(u32, u32),
}

#[derive(Clone, Copy, Debug)]
pub struct DagLaw {
    /// ETP equation id, 1-based.
    pub id: u32,
    pub lhs: u32,
    pub rhs: u32,
    pub arity: u8,
}

pub struct Dag {
    pub nodes: Vec<Node>,
    /// `arity[i]` is one more than the largest variable index below node `i`.
    pub arity: Vec<u8>,
    pub laws: Vec<DagLaw>,
}

impl Dag {
    pub fn build(laws: &[Law]) -> Dag {
        let mut dag = Dag {
            nodes: Vec::new(),
            arity: Vec::new(),
            laws: Vec::with_capacity(laws.len()),
        };
        let mut interned: FxHashMap<Node, u32> = FxHashMap::default();

        // Variable nodes occupy indices 0..MAX_VARS so that `vals[v]` is the
        // value of variable `v` with no indirection.
        for v in 0..MAX_VARS as u8 {
            dag.nodes.push(Node::Var(v));
            dag.arity.push(v + 1);
            interned.insert(Node::Var(v), v as u32);
        }

        for law in laws {
            let lhs = dag.intern(&law.lhs, &mut interned);
            let rhs = dag.intern(&law.rhs, &mut interned);
            dag.laws.push(DagLaw {
                id: law.id,
                lhs,
                rhs,
                arity: law.arity,
            });
        }
        dag
    }

    fn intern(&mut self, t: &Term, interned: &mut FxHashMap<Node, u32>) -> u32 {
        let node = match t {
            Term::Var(v) => Node::Var(*v),
            Term::Op(l, r) => {
                let li = self.intern(l, interned);
                let ri = self.intern(r, interned);
                Node::Op(li, ri)
            }
        };
        if let Some(&i) = interned.get(&node) {
            return i;
        }
        let arity = match node {
            Node::Var(v) => v + 1,
            Node::Op(l, r) => self.arity[l as usize].max(self.arity[r as usize]),
        };
        let idx = self.nodes.len() as u32;
        self.nodes.push(node);
        self.arity.push(arity);
        interned.insert(node, idx);
        idx
    }

    pub fn n_laws(&self) -> usize {
        self.laws.len()
    }

    /// Number of interned nodes that are applications rather than variables.
    pub fn n_op_nodes(&self) -> usize {
        self.nodes.len() - MAX_VARS
    }
}

/// Laws grouped by how many variables they use, with the sub-DAG each group
/// needs. A law of arity `k` is refuted or confirmed by sweeping `n^k`
/// assignments, so grouping saves the factor `n^(6-k)` that a single
/// six-variable sweep would waste on the 4347 laws using four variables or
/// fewer.
pub struct Bucket {
    pub arity: u8,
    /// Application nodes reachable from this bucket's laws, in ascending
    /// index order, which is a valid evaluation order.
    pub order: Vec<u32>,
    /// Indices into `Dag::laws`.
    pub laws: Vec<u32>,
}

pub struct EvalPlan {
    pub buckets: Vec<Bucket>,
    pub n_laws: usize,
}

impl EvalPlan {
    pub fn build(dag: &Dag) -> EvalPlan {
        let mut buckets = Vec::new();
        for k in 1..=MAX_VARS as u8 {
            let laws: Vec<u32> = dag
                .laws
                .iter()
                .enumerate()
                .filter(|(_, l)| l.arity == k)
                .map(|(i, _)| i as u32)
                .collect();
            if laws.is_empty() {
                continue;
            }
            let mut needed = vec![false; dag.nodes.len()];
            let mut stack: Vec<u32> = Vec::new();
            for &li in &laws {
                let l = &dag.laws[li as usize];
                stack.push(l.lhs);
                stack.push(l.rhs);
            }
            while let Some(n) = stack.pop() {
                if needed[n as usize] {
                    continue;
                }
                needed[n as usize] = true;
                if let Node::Op(a, b) = dag.nodes[n as usize] {
                    stack.push(a);
                    stack.push(b);
                }
            }
            let order: Vec<u32> = (MAX_VARS as u32..dag.nodes.len() as u32)
                .filter(|&i| needed[i as usize])
                .collect();
            buckets.push(Bucket {
                arity: k,
                order,
                laws,
            });
        }
        EvalPlan {
            buckets,
            n_laws: dag.laws.len(),
        }
    }
}
