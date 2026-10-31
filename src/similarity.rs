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

/// How one line of A was found in B; `at` is the index of the line of B.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Found {
    /// The same normalized line; each line of B matches at most one line of A.
    Exact {
        at: usize,
    },
    /// A line of B at least `threshold` similar (normalized Levenshtein); `score` is the best.
    Fuzzy {
        score: f64,
        at: usize,
    },
    Missing,
}

impl Found {
    /// The line of B it was found at.
    pub fn at(self) -> Option<usize> {
        match self {
            Found::Exact { at } | Found::Fuzzy { at, .. } => Some(at),
            Found::Missing => None,
        }
    }
}

/// For each line of `a`, whether it is in `b`: exact matches first, then, for the rest, the most
/// similar line of `b` if it scores at least `threshold` (no fuzzy matching if `threshold` > 1).
pub fn find_lines(a: &[String], b: &[String], threshold: f64) -> Vec<Found> {
    // The lines of B with each text, last first: the first unused one is popped.
    let mut available: HashMap<&str, Vec<usize>> = HashMap::new();
    for (at, line) in b.iter().enumerate().rev() {
        available.entry(line.as_str()).or_default().push(at);
    }
    let mut found: Vec<Found> = a
        .iter()
        .map(|line| {
            available
                .get_mut(line.as_str())
                .and_then(Vec::pop)
                .map_or(Found::Missing, |at| Found::Exact { at })
        })
        .collect();
    if threshold > 1.0 {
        return found;
    }

    // Each distinct text once, with its length and its first line.
    let mut candidates: Vec<Candidate> = b
        .iter()
        .enumerate()
        .map(|(at, l)| Candidate {
            text: l.as_str(),
            length: l.chars().count(),
            at,
        })
        .collect();
    candidates.sort_unstable_by(|x, y| x.text.cmp(y.text).then(x.at.cmp(&y.at)));
    candidates.dedup_by(|later, first| later.text == first.text);
    for (line, slot) in a.iter().zip(found.iter_mut()) {
        if *slot != Found::Missing {
            continue;
        }
        if let Some((score, at)) = best_match(line, &candidates, threshold) {
            *slot = Found::Fuzzy { score, at };
        }
    }
    found
}

/// A line of B to compare with: its text, its length in characters, and where it is.
struct Candidate<'a> {
    text: &'a str,
    length: usize,
    at: usize,
}

/// The best normalized Levenshtein similarity of `line` to any candidate, and that candidate's
/// line, if one reaches `threshold`. Candidates whose length rules them out are skipped: the
/// similarity of two strings is at most shorter length / longer length.
fn best_match(line: &str, candidates: &[Candidate], threshold: f64) -> Option<(f64, usize)> {
    let length = line.chars().count();
    let comparator = levenshtein::BatchComparator::new(line.chars());
    let mut best: Option<(f64, usize)> = None;
    for candidate in candidates {
        let (short, long) = if length < candidate.length {
            (length, candidate.length)
        } else {
            (candidate.length, length)
        };
        if long == 0 || (short as f64) < threshold * long as f64 {
            continue;
        }
        let cutoff = best.map_or(threshold, |(b, _)| b.max(threshold));
        let args = levenshtein::Args::default().score_cutoff(cutoff);
        if let Some(score) =
            comparator.normalized_similarity_with_args(candidate.text.chars(), &args)
            && best.is_none_or(|(b, _)| score > b)
        {
            best = Some((score, candidate.at));
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
