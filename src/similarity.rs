//! Line normalization and containment: how much of one set of lines is found in another.
//! The only module that uses rapidfuzz, so it can be swapped or vendored in one place.

use std::collections::HashMap;

use rapidfuzz::distance::{lcs_seq, levenshtein};

/// A line as compared: whitespace collapsed. None for lines without any letter or digit
/// (blank lines, lone braces), which would match everywhere.
pub fn normalize(line: &str) -> Option<String> {
    if !line.chars().any(char::is_alphanumeric) {
        return None;
    }
    Some(line.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// 64-bit FNV-1a: a stable hash of a normalized line.
pub fn line_hash(line: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in line.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// How one line of A was found in B.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Found {
    /// The same normalized line; each line of B matches at most one line of A.
    Exact,
    /// A line of B at least `threshold` similar (normalized Levenshtein); `score` is the best.
    Fuzzy {
        score: f64,
    },
    Missing,
}

/// For each line of `a`, whether it is in `b`: exact matches first, then, for the rest, the most
/// similar line of `b` if it scores at least `threshold` (no fuzzy matching if `threshold` > 1).
pub fn find_lines(a: &[String], b: &[String], threshold: f64) -> Vec<Found> {
    let mut available: HashMap<&str, usize> = HashMap::new();
    for line in b {
        *available.entry(line.as_str()).or_default() += 1;
    }
    let mut found: Vec<Found> = a
        .iter()
        .map(|line| match available.get_mut(line.as_str()) {
            Some(count) if *count > 0 => {
                *count -= 1;
                Found::Exact
            }
            _ => Found::Missing,
        })
        .collect();
    if threshold > 1.0 {
        return found;
    }

    let mut candidates: Vec<(&str, usize)> =
        b.iter().map(|l| (l.as_str(), l.chars().count())).collect();
    candidates.sort_unstable();
    candidates.dedup();
    for (line, slot) in a.iter().zip(found.iter_mut()) {
        if *slot != Found::Missing {
            continue;
        }
        if let Some(score) = best_match(line, &candidates, threshold) {
            *slot = Found::Fuzzy { score };
        }
    }
    found
}

/// The best normalized Levenshtein similarity of `line` to any candidate, if one reaches
/// `threshold`. Candidates whose length rules them out are skipped: the similarity of two
/// strings is at most shorter length / longer length.
fn best_match(line: &str, candidates: &[(&str, usize)], threshold: f64) -> Option<f64> {
    let length = line.chars().count();
    let comparator = levenshtein::BatchComparator::new(line.chars());
    let mut best: Option<f64> = None;
    for &(candidate, candidate_length) in candidates {
        let (short, long) = if length < candidate_length {
            (length, candidate_length)
        } else {
            (candidate_length, length)
        };
        if long == 0 || (short as f64) < threshold * long as f64 {
            continue;
        }
        let cutoff = best.map_or(threshold, |b| b.max(threshold));
        let args = levenshtein::Args::default().score_cutoff(cutoff);
        if let Some(score) = comparator.normalized_similarity_with_args(candidate.chars(), &args)
            && best.is_none_or(|b| score > b)
        {
            best = Some(score);
        }
    }
    best
}

/// How many lines of `a` appear in `b` in the same order: the longest common subsequence of the
/// two line-hash sequences.
pub fn in_order(a: &[u64], b: &[u64]) -> usize {
    lcs_seq::similarity(a.iter().copied(), b.iter().copied())
}

#[cfg(test)]
mod tests;
