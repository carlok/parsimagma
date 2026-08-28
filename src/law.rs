//! Parsing of the ETP law list (`data/etp/equations.txt`).
//!
//! Each line is `lhs = rhs` over the single binary operation `◇` with
//! variables drawn from `u v w x y z`. Line number is the canonical ETP
//! equation id (1-based), per `scripts/generate_eqs_list.py` in
//! teorth/equational_theories.
//!
//! Every side of every law in the order-≤4 set is fully parenthesised: no
//! side carries more than one top-level `◇`, so the grammar is unambiguous
//! and needs no associativity convention. `parse_side` rejects anything it
//! cannot read rather than guessing.

use std::fmt;

/// A term over one binary operation. Variables carry a *canonical* index,
/// assigned by first appearance scanning lhs then rhs, so that laws which
/// differ only by variable renaming share structure in the DAG.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Var(u8),
    Op(Box<Term>, Box<Term>),
}

impl Term {
    pub fn ops(&self) -> usize {
        match self {
            Term::Var(_) => 0,
            Term::Op(l, r) => 1 + l.ops() + r.ops(),
        }
    }

    pub fn max_var(&self) -> u8 {
        match self {
            Term::Var(v) => *v,
            Term::Op(l, r) => l.max_var().max(r.max_var()),
        }
    }
}

impl fmt::Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(v) => write!(f, "{}", (b'a' + v) as char),
            Term::Op(l, r) => write!(f, "({:?} \u{25c7} {:?})", l, r),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Law {
    /// ETP equation id, 1-based.
    pub id: u32,
    pub lhs: Term,
    pub rhs: Term,
    /// Number of distinct variables. Canonical indices are `0..arity`.
    pub arity: u8,
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "equations.txt:{}: {}", self.line, self.msg)
    }
}

impl std::error::Error for ParseError {}

/// Parse the whole law list. The returned vector is indexed by `id - 1`.
pub fn parse_laws(text: &str) -> Result<Vec<Law>, ParseError> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let lineno = i + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        out.push(
            parse_law(line, out.len() as u32 + 1)
                .map_err(|msg| ParseError { line: lineno, msg })?,
        );
    }
    Ok(out)
}

fn parse_law(line: &str, id: u32) -> Result<Law, String> {
    let (l, r) = line
        .split_once('=')
        .ok_or_else(|| format!("no `=` in {line:?}"))?;

    // Canonical variable numbering: order of first appearance across lhs
    // then rhs. This is what makes `x = x ◇ y` and `y = y ◇ z` share a
    // DAG node.
    let mut names: Vec<char> = Vec::new();
    for ch in line.chars() {
        if ch.is_ascii_lowercase() && !names.contains(&ch) {
            names.push(ch);
        }
    }
    if names.is_empty() {
        return Err("no variables".to_string());
    }
    if names.len() > crate::dag::MAX_VARS {
        return Err(format!(
            "{} distinct variables, expected at most {}",
            names.len(),
            crate::dag::MAX_VARS
        ));
    }

    let lhs = parse_side(l.trim(), &names)?;
    let rhs = parse_side(r.trim(), &names)?;
    Ok(Law {
        id,
        lhs,
        rhs,
        arity: names.len() as u8,
    })
}

fn parse_side(s: &str, names: &[char]) -> Result<Term, String> {
    let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let (t, used) = parse_term(&chars, names)?;
    if used != chars.len() {
        return Err(format!("trailing input in {s:?} after position {used}"));
    }
    Ok(t)
}

const DIAMOND: char = '\u{25c7}';

/// An *atom* is a variable or a fully parenthesised application. The law
/// grammar is `side := atom | atom ◇ atom`, and inside parentheses the body
/// is always `atom ◇ atom`. Keeping atoms and sides distinct is what makes
/// the grammar unambiguous without an associativity rule.
fn parse_atom(c: &[char], names: &[char]) -> Result<(Term, usize), String> {
    match c.first() {
        None => Err("empty term".to_string()),
        Some('(') => {
            let (l, n1) = parse_atom(&c[1..], names)?;
            let mut i = 1 + n1;
            if c.get(i) != Some(&DIAMOND) {
                return Err(format!("expected \u{25c7} at position {i}"));
            }
            i += 1;
            let (r, n2) = parse_atom(&c[i..], names)?;
            i += n2;
            if c.get(i) != Some(&')') {
                return Err(format!("expected `)` at position {i}"));
            }
            Ok((Term::Op(Box::new(l), Box::new(r)), i + 1))
        }
        Some(&ch) if ch.is_ascii_lowercase() => {
            let idx = names
                .iter()
                .position(|&n| n == ch)
                .ok_or_else(|| format!("unknown variable {ch:?}"))? as u8;
            Ok((Term::Var(idx), 1))
        }
        Some(&ch) => Err(format!("unexpected {ch:?}")),
    }
}

/// Returns the term and how many characters it consumed.
fn parse_term(c: &[char], names: &[char]) -> Result<(Term, usize), String> {
    let (head, i) = parse_atom(c, names)?;
    // A single top-level `◇` with no enclosing parentheses is allowed, as in
    // `x ◇ y = ...`. A second one would be ambiguous; `parse_side`'s length
    // check rejects it because it is left unconsumed.
    if c.get(i) == Some(&DIAMOND) {
        let (rhs, n) = parse_atom(&c[i + 1..], names)?;
        return Ok((Term::Op(Box::new(head), Box::new(rhs)), i + 1 + n));
    }
    Ok((head, i))
}
