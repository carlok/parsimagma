//! Magma signature and coverage engine over the Equational Theories Project
//! law set.
//!
//! The central object is a *signature*: for a magma `M`, the bit vector over
//! the 4694 laws recording which ones `M` satisfies. Every separation
//! question is then a bit test, and coverage over a corpus of magmas is
//! bitset algebra over packed words.

pub mod corpus;
pub mod coverage;
pub mod dag;
pub mod etpdata;
pub mod finite;
pub mod graph;
pub mod law;
pub mod linear;
pub mod nc;
pub mod rings;
pub mod sig;
pub mod twist;

pub use dag::{Dag, EvalPlan};
pub use finite::{Engine, FiniteMagma};
pub use law::{parse_laws, Law, Term};
pub use sig::Signature;

/// The ETP order-≤4 law count.
pub const N_LAWS_ORDER4: usize = 4694;
