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

/// Where a line is: its file and the item around it, as ids both sides share (the same path, or
/// the same item name, gets the same id). None where not known.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Place {
    pub file: Option<u32>,
    pub item: Option<u32>,
}

/// How near a line of B is to a line of A: 0 in the same file and item, 1 in the same file, 2 in
/// an item of the same name elsewhere, 3 none of these.
fn nearness(a: Place, b: Place) -> u8 {
    let file = a.file.is_some() && a.file == b.file;
    let item = a.item.is_some() && a.item == b.item;
    match (file, item) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    }
}

/// Lines and where each is; `places` is per line, or empty if not known.
#[derive(Clone, Copy)]
struct Lines<'a> {
    texts: &'a [String],
    places: &'a [Place],
}

impl Lines<'_> {
    fn place(self, line: usize) -> Place {
        self.places.get(line).copied().unwrap_or_default()
    }
}

/// For each line of `a`, whether it is in `b`: exact matches first, then, for the rest, the most
/// similar line of `b` if it scores at least `threshold` (no fuzzy matching if `threshold` > 1).
/// Among equally good lines of `b`, the first.
pub fn find_lines(a: &[String], b: &[String], threshold: f64) -> Vec<Found> {
    find_lines_placed(a, &[], b, &[], threshold)
}

/// [`find_lines`], choosing among equally good lines of `b` by where they are: the same file and
/// item first, then the same file, then an item of the same name elsewhere, then the first.
/// Places change only where a line is found, not whether or how well.
pub fn find_lines_placed(
    a: &[String],
    a_places: &[Place],
    b: &[String],
    b_places: &[Place],
    threshold: f64,
) -> Vec<Found> {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    find_lines_with(
        Lines {
            texts: a,
            places: a_places,
        },
        Lines {
            texts: b,
            places: b_places,
        },
        threshold,
        threads,
    )
}

/// The key of a line's text and place at a nearness: the lines of B under the same key as a line
/// of A are that near to it.
type Key<'a> = (u8, &'a str, Option<u32>, Option<u32>);

fn key(text: &str, place: Place, nearness: u8) -> Option<Key<'_>> {
    let (file, item) = match nearness {
        0 => (Some(place.file?), Some(place.item?)),
        1 => (Some(place.file?), None),
        2 => (None, Some(place.item?)),
        _ => (None, None),
    };
    Some((nearness, text, file, item))
}

/// Exact matches: each line of B matches at most one line of A. Nearest first, over all of A, so
/// a line whose copy is in its own file takes it before another line takes it from further away.
fn exact(a: Lines, b: Lines) -> Vec<Found> {
    // The lines of B under each key, last first: the first unused one is popped.
    let mut lines: HashMap<Key, Vec<usize>> = HashMap::new();
    for at in (0..b.texts.len()).rev() {
        for nearness in 0..4 {
            if let Some(k) = key(&b.texts[at], b.place(at), nearness) {
                lines.entry(k).or_default().push(at);
            }
        }
    }
    let mut used = vec![false; b.texts.len()];
    let mut found = vec![Found::Missing; a.texts.len()];
    for nearness in 0..4 {
        for (i, f) in found.iter_mut().enumerate() {
            if *f != Found::Missing {
                continue;
            }
            let Some(list) = key(&a.texts[i], a.place(i), nearness).and_then(|k| lines.get_mut(&k))
            else {
                continue;
            };
            while let Some(at) = list.pop() {
                if !used[at] {
                    used[at] = true;
                    *f = Found::Exact { at };
                    break;
                }
            }
        }
    }
    found
}

/// `find_lines_placed` with the fuzzy pass split over `threads` threads.
fn find_lines_with(a: Lines, b: Lines, threshold: f64, threads: usize) -> Vec<Found> {
    let mut found = exact(a, b);
    if threshold > 1.0 {
        return found;
    }

    // Each distinct text once, with its length and its lines.
    let mut sorted: Vec<(&str, usize)> = b
        .texts
        .iter()
        .enumerate()
        .map(|(at, l)| (l.as_str(), at))
        .collect();
    sorted.sort_unstable();
    let mut candidates: Vec<Candidate> = Vec::new();
    for (text, at) in sorted {
        match candidates.last_mut() {
            Some(last) if last.text == text => last.lines.push(at),
            _ => candidates.push(Candidate {
                text,
                length: text.chars().count(),
                lines: vec![at],
            }),
        }
    }

    // Each line's best match is independent of the others: split the lines left over threads.
    let missing: Vec<usize> = (0..a.texts.len())
        .filter(|&i| found[i] == Found::Missing)
        .collect();
    let threads = threads.clamp(1, missing.len().max(1));
    let size = missing.len().div_ceil(threads).max(1);
    let candidates = candidates.as_slice();
    let best = |i: usize| best_match(&a.texts[i], a.place(i), candidates, b, threshold);
    let matches: Vec<Option<(f64, usize)>> = if threads == 1 {
        missing.iter().map(|&i| best(i)).collect()
    } else {
        std::thread::scope(|scope| {
            let parts: Vec<_> = missing
                .chunks(size)
                .map(|part| scope.spawn(move || part.iter().map(|&i| best(i)).collect::<Vec<_>>()))
                .collect();
            parts
                .into_iter()
                .flat_map(|part| part.join().expect("fuzzy matching thread panicked"))
                .collect()
        })
    };
    for (i, matched) in missing.into_iter().zip(matches) {
        if let Some((score, at)) = matched {
            found[i] = Found::Fuzzy { score, at };
        }
    }
    found
}

/// A distinct text of B to compare with: its length in characters, and the lines that have it.
struct Candidate<'a> {
    text: &'a str,
    length: usize,
    lines: Vec<usize>,
}

/// The best normalized Levenshtein similarity of `line` to any candidate, if one reaches
/// `threshold`, and the nearest line of B with a text that scores it. Candidates whose length
/// rules them out are skipped: the similarity of two strings is at most shorter length / longer
/// length.
fn best_match(
    line: &str,
    place: Place,
    candidates: &[Candidate],
    b: Lines,
    threshold: f64,
) -> Option<(f64, usize)> {
    let length = line.chars().count();
    let comparator = levenshtein::BatchComparator::new(line.chars());
    let mut best: Option<f64> = None;
    // The candidates with the best score.
    let mut tied: Vec<usize> = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let (short, long) = if length < candidate.length {
            (length, candidate.length)
        } else {
            (candidate.length, length)
        };
        if long == 0 || (short as f64) < threshold * long as f64 {
            continue;
        }
        let cutoff = best.map_or(threshold, |b| b.max(threshold));
        let args = levenshtein::Args::default().score_cutoff(cutoff);
        let Some(score) = comparator.normalized_similarity_with_args(candidate.text.chars(), &args)
        else {
            continue;
        };
        if best.is_none_or(|b| score > b) {
            best = Some(score);
            tied.clear();
        }
        if best == Some(score) {
            tied.push(index);
        }
    }
    let score = best?;
    let at = tied
        .iter()
        .flat_map(|&c| &candidates[c].lines)
        .copied()
        .min_by_key(|&at| (nearness(place, b.place(at)), at))?;
    Some((score, at))
}

/// The Levenshtein distance of two lines: characters inserted, deleted or replaced.
pub fn edits(a: &str, b: &str) -> usize {
    levenshtein::distance(a.chars(), b.chars())
}

/// How many lines of `a` appear in `b` in the same order: the longest common subsequence of the
/// two line-hash sequences.
pub fn in_order(a: &[u64], b: &[u64]) -> usize {
    lcs_seq::similarity(a.iter().copied(), b.iter().copied())
}

#[cfg(test)]
mod tests;
