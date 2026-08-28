//! Loaders for the ETP data files vendored under `data/etp/`.
//!
//! Provenance and upstream commit are recorded in `data/etp/PROVENANCE.txt`.
//! These files are the differential-test oracle: the engine is not trusted
//! until it reproduces them exactly.

use crate::finite::FiniteMagma;
use std::io::Read;

/// Where the vendored and derived ETP files live.
pub const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/etp/");

/// Read a data file, transparently decompressing `<name>.gz` when the plain
/// file is absent. The three large files are stored compressed because they
/// shrink by 18x to 47x: a 4694 x 4694 bit matrix is enormously structured,
/// and keeping the repository small matters more than avoiding an inflate.
pub fn read_bytes(name: &str) -> Vec<u8> {
    let plain = format!("{DATA_DIR}{name}");
    if let Ok(b) = std::fs::read(&plain) {
        return b;
    }
    let gz = format!("{plain}.gz");
    let f = std::fs::File::open(&gz).unwrap_or_else(|e| panic!("neither {plain} nor {gz}: {e}"));
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(f)
        .read_to_end(&mut out)
        .unwrap_or_else(|e| panic!("{gz}: {e}"));
    out
}

pub fn read_text(name: &str) -> String {
    String::from_utf8(read_bytes(name)).expect("data file is not UTF-8")
}

/// One entry of `All4x4Tables/data/refutations{2x2,3x3,4x4}.txt`: a magma
/// together with the *complete* list of laws it satisfies, as computed by
/// the ETP's own brute force. This is the strongest oracle available, since
/// it pins every one of the 4694 bits rather than a single one.
#[derive(Debug, Clone)]
pub struct RefutationEntry {
    pub magma: FiniteMagma,
    /// 1-based ETP equation ids, ascending.
    pub proves: Vec<u32>,
}

/// Parse a flat integer list such as `[1, 8, 10]`.
fn parse_flat(s: &str) -> Result<Vec<u32>, String> {
    let inner = s
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .ok_or_else(|| format!("not a list: {s:?}"))?;
    inner
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<u32>().map_err(|e| format!("{t:?}: {e}")))
        .collect()
}

/// Parse a nested list such as `[[0, 0], [1, 0]]`.
fn parse_rows(s: &str) -> Result<Vec<Vec<u8>>, String> {
    let inner = s
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .ok_or_else(|| format!("not a table: {s:?}"))?;
    let mut rows = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in inner.chars() {
        match ch {
            '[' => {
                depth += 1;
                cur.clear();
            }
            ']' => {
                depth -= 1;
                rows.push(
                    cur.split(',')
                        .map(str::trim)
                        .filter(|t| !t.is_empty())
                        .map(|t| t.parse::<u8>().map_err(|e| format!("{t:?}: {e}")))
                        .collect::<Result<Vec<u8>, String>>()?,
                );
            }
            _ if depth == 1 => cur.push(ch),
            _ => {}
        }
    }
    Ok(rows)
}

pub fn parse_refutations(text: &str) -> Result<Vec<RefutationEntry>, String> {
    let mut out = Vec::new();
    let mut pending: Option<FiniteMagma> = None;
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Table ") {
            let rows = parse_rows(rest).map_err(|e| format!("line {}: {e}", i + 1))?;
            let m =
                FiniteMagma::from_rows(&rows).map_err(|e| format!("line {}: {}", i + 1, e.0))?;
            pending = Some(m);
        } else if let Some(rest) = line.strip_prefix("Proves ") {
            let proves = parse_flat(rest).map_err(|e| format!("line {}: {e}", i + 1))?;
            let magma = pending
                .take()
                .ok_or_else(|| format!("line {}: Proves without a preceding Table", i + 1))?;
            out.push(RefutationEntry { magma, proves });
        }
    }
    Ok(out)
}

/// Parse `data/etp/smallest_magma_examples.txt`: `<eq_id> [[...], ...]`.
pub fn parse_smallest_examples(text: &str) -> Result<Vec<(u32, FiniteMagma)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (id, rest) = line
            .split_once(' ')
            .ok_or_else(|| format!("line {}: no space separator", i + 1))?;
        let id: u32 = id.parse().map_err(|e| format!("line {}: {e}", i + 1))?;
        let rows = parse_rows(rest).map_err(|e| format!("line {}: {e}", i + 1))?;
        let m = FiniteMagma::from_rows(&rows).map_err(|e| format!("line {}: {}", i + 1, e.0))?;
        out.push((id, m));
    }
    Ok(out)
}

/// Parse `data/etp/smallest_magma.txt`: `<eq_id> <carrier size>`.
pub fn parse_smallest_sizes(text: &str) -> Result<Vec<(u32, usize)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let id: u32 = it
            .next()
            .ok_or_else(|| format!("line {}: empty", i + 1))?
            .parse()
            .map_err(|e| format!("line {}: {e}", i + 1))?;
        let n: usize = it
            .next()
            .ok_or_else(|| format!("line {}: no size", i + 1))?
            .parse()
            .map_err(|e| format!("line {}: {e}", i + 1))?;
        out.push((id, n));
    }
    Ok(out)
}
