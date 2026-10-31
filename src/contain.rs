//! `sundgren contain`: how much of one branch's work is in another's, both ways.
//!
//! A branch's work is the lines it added since it left the base: `git diff merge-base(base,
//! branch) branch`, normalized (see [`similarity::normalize`]). A line of A counts as found in B
//! if B added the same line, or (fuzzy) a line at least `threshold` similar.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::similarity::{self, Found};
use crate::{Error, Result, git};

pub const CONTAINMENT_SCHEMA: &str = "sundgren.containment/0";

/// Containment at or above this, both ways, is the same work; one way, one is in the other.
pub const HIGH: f64 = 0.8;
/// Below this both ways, the two are unrelated; in between, they overlap.
pub const LOW: f64 = 0.2;

/// One added line, normalized.
#[derive(Debug, Clone)]
struct Line {
    path: String,
    text: String,
    hash: u64,
}

/// What a branch added since it left the base.
#[derive(Debug, Serialize)]
pub struct Side {
    pub rev: String,
    pub oid: String,
    /// Where it left the base: merge-base(base, rev).
    pub fork: String,
    pub files: usize,
    /// Added lines kept after normalization.
    pub lines: usize,
    #[serde(skip)]
    added: Vec<Line>,
}

/// One file's share of a direction.
#[derive(Debug, Serialize, PartialEq)]
pub struct FileShare {
    pub path: String,
    pub lines: usize,
    pub exact: usize,
    pub fuzzy: usize,
}

/// How much of one side is found in the other.
#[derive(Debug, Serialize)]
pub struct Direction {
    pub lines: usize,
    pub exact: usize,
    pub fuzzy: usize,
    /// Lines found in the same order (longest common subsequence of exact lines).
    pub in_order: usize,
    /// (exact + fuzzy) / lines. Left out of the JSON, like the two below, if there are no lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containment: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_containment: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered_containment: Option<f64>,
    pub files: Vec<FileShare>,
}

#[derive(Debug, Serialize)]
pub struct ContainmentDocument {
    pub schema: &'static str,
    pub repository: PathBuf,
    pub base: String,
    pub base_oid: String,
    pub threshold: f64,
    pub a: Side,
    pub b: Side,
    /// How much of A's work is in B.
    pub a_in_b: Direction,
    pub b_in_a: Direction,
    /// same, a-in-b, b-in-a, overlap, unrelated, or empty.
    pub relation: &'static str,
}

fn side(repo: &Path, base_oid: &str, rev: &str) -> Result<Side> {
    let oid = git::resolve(repo, rev)?;
    let fork = git::merge_base(repo, base_oid, &oid)?
        .ok_or_else(|| Error::usage(format!("{rev} has no common ancestor with the base")))?;
    let files = git::added_lines(repo, &fork, &oid)?;
    let added: Vec<Line> = files
        .iter()
        .flat_map(|file| {
            file.added.iter().filter_map(|(_, raw)| {
                similarity::normalize(raw).map(|text| Line {
                    path: file.path.clone(),
                    hash: similarity::line_hash(&text),
                    text,
                })
            })
        })
        .collect();
    let mut paths: Vec<&str> = added.iter().map(|l| l.path.as_str()).collect();
    paths.dedup();
    Ok(Side {
        rev: rev.to_owned(),
        oid,
        fork,
        files: paths.len(),
        lines: added.len(),
        added,
    })
}

fn ratio(part: usize, whole: usize) -> Option<f64> {
    (whole > 0).then(|| part as f64 / whole as f64)
}

fn direction(a: &[Line], b: &[Line], threshold: f64) -> Direction {
    let a_texts: Vec<String> = a.iter().map(|l| l.text.clone()).collect();
    let b_texts: Vec<String> = b.iter().map(|l| l.text.clone()).collect();
    let found = similarity::find_lines(&a_texts, &b_texts, threshold);
    let a_hashes: Vec<u64> = a.iter().map(|l| l.hash).collect();
    let b_hashes: Vec<u64> = b.iter().map(|l| l.hash).collect();
    let in_order = similarity::in_order(&a_hashes, &b_hashes);

    let mut files: Vec<FileShare> = Vec::new();
    for (line, found) in a.iter().zip(&found) {
        if files.last().is_none_or(|f| f.path != line.path) {
            files.push(FileShare {
                path: line.path.clone(),
                lines: 0,
                exact: 0,
                fuzzy: 0,
            });
        }
        if let Some(file) = files.last_mut() {
            file.lines += 1;
            match found {
                Found::Exact => file.exact += 1,
                Found::Fuzzy { .. } => file.fuzzy += 1,
                Found::Missing => {}
            }
        }
    }
    let exact = files.iter().map(|f| f.exact).sum();
    let fuzzy = files.iter().map(|f| f.fuzzy).sum();
    Direction {
        lines: a.len(),
        exact,
        fuzzy,
        in_order,
        containment: ratio(exact + fuzzy, a.len()),
        exact_containment: ratio(exact, a.len()),
        ordered_containment: ratio(in_order, a.len()),
        files,
    }
}

/// The relation the two containments suggest (thresholds [`HIGH`] and [`LOW`]).
pub fn relation(a_in_b: Option<f64>, b_in_a: Option<f64>) -> &'static str {
    let (Some(ab), Some(ba)) = (a_in_b, b_in_a) else {
        return "empty";
    };
    match (ab >= HIGH, ba >= HIGH) {
        (true, true) => "same",
        (true, false) => "a-in-b",
        (false, true) => "b-in-a",
        _ if ab.max(ba) >= LOW => "overlap",
        _ => "unrelated",
    }
}

/// Compare the work of `a` and `b` since each left `base`.
pub fn containment_document(
    repo: &Path,
    base: &str,
    a: &str,
    b: &str,
    threshold: f64,
) -> Result<ContainmentDocument> {
    let repository = git::toplevel(repo)?;
    let base_oid = git::resolve(&repository, base)?;
    let a = side(&repository, &base_oid, a)?;
    let b = side(&repository, &base_oid, b)?;
    let a_in_b = direction(&a.added, &b.added, threshold);
    let b_in_a = direction(&b.added, &a.added, threshold);
    let relation = relation(a_in_b.containment, b_in_a.containment);
    Ok(ContainmentDocument {
        schema: CONTAINMENT_SCHEMA,
        repository,
        base: base.to_owned(),
        base_oid,
        threshold,
        a,
        b,
        a_in_b,
        b_in_a,
        relation,
    })
}

fn percent(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| format!("{:.0}%", v * 100.0))
}

/// A short human summary.
pub fn text(doc: &ContainmentDocument) -> String {
    let mut out = String::new();
    let short = |oid: &str| oid.chars().take(8).collect::<String>();
    out.push_str(&format!(
        "base {} ({}), fuzzy threshold {}\n",
        doc.base,
        short(&doc.base_oid),
        doc.threshold
    ));
    for (name, side) in [("A", &doc.a), ("B", &doc.b)] {
        out.push_str(&format!(
            "{name} {} ({}): {} lines in {} files since {}\n",
            side.rev,
            short(&side.oid),
            side.lines,
            side.files,
            short(&side.fork)
        ));
    }
    for (label, d) in [("A in B", &doc.a_in_b), ("B in A", &doc.b_in_a)] {
        out.push_str(&format!(
            "{label}: {} (exact {}, in order {}): {} exact + {} fuzzy of {} lines\n",
            percent(d.containment),
            percent(d.exact_containment),
            percent(d.ordered_containment),
            d.exact,
            d.fuzzy,
            d.lines
        ));
    }
    out.push_str(&format!("relation: {}\n", doc.relation));
    for (label, d) in [("A in B", &doc.a_in_b), ("B in A", &doc.b_in_a)] {
        out.push_str(&format!("\n{label}, per file:\n"));
        for f in &d.files {
            out.push_str(&format!(
                "  {:>4}% {:>5} lines  {}\n",
                ((f.exact + f.fuzzy) * 100)
                    .checked_div(f.lines)
                    .unwrap_or(0),
                f.lines,
                f.path
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests;
