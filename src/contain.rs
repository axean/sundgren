//! `sundgren contain`: how much of one branch's work is in another's, both ways.
//!
//! A branch's work is the lines it added since it left the base: `git diff merge-base(base,
//! branch) branch`, normalized (see [`similarity::normalize`]). A line of A counts as found in B
//! if B added the same line, or (fuzzy) a line at least `threshold` similar.
//!
//! Files in a language sundgren reads (see [`syntax`]) add to that: each added line gets the item
//! around it and whether it is only a comment, so containment is also given per item, with the
//! item of the other side most of its lines were found in, and without comment lines. The
//! numbers above do not change.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::similarity::{self, Found};
use crate::syntax::{self, Item, Kind, Language};
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
    /// The innermost item around it, an index into its side's items. None outside items, and in
    /// files sundgren doesn't read.
    item: Option<usize>,
    /// Only a comment (known only in files sundgren reads).
    comment: bool,
}

/// An item of a file sundgren read: its full name (`Class.method`, `Type::method`) and kind.
#[derive(Debug, Clone)]
struct Unit {
    name: String,
    kind: &'static str,
}

/// What sundgren read of one file: per line number, the innermost item and whether the line is
/// only a comment.
struct Reading {
    item: Vec<Option<usize>>,
    comment: Vec<bool>,
}

/// Read `source`, adding its items to `units`.
fn read(language: Language, source: &str, units: &mut Vec<Unit>) -> Reading {
    // Line numbers are 1-based; one more for a last line without a newline.
    let count = source.lines().count() + 2;
    let mut comment = vec![false; count];
    let mut code = vec![false; count];
    for token in syntax::lex(language, source) {
        for line in token.line..=token.end_line.min(count - 1) {
            if token.kind == Kind::Comment {
                comment[line] = true;
            } else {
                code[line] = true;
            }
        }
    }
    let separator = if language == Language::Rust {
        "::"
    } else {
        "."
    };
    let mut item = vec![None; count];
    // Parents first, so the innermost item of a line is the last one written.
    fn walk(
        items: &[Item],
        prefix: Option<&str>,
        separator: &str,
        units: &mut Vec<Unit>,
        lines: &mut [Option<usize>],
    ) {
        for it in items {
            let name =
                prefix.map_or_else(|| it.name.clone(), |p| format!("{p}{separator}{}", it.name));
            let index = units.len();
            units.push(Unit {
                name: name.clone(),
                kind: it.kind,
            });
            for line in lines.iter_mut().take(it.end + 1).skip(it.start) {
                *line = Some(index);
            }
            walk(&it.children, Some(&name), separator, units, lines);
        }
    }
    walk(
        &syntax::items(language, source),
        None,
        separator,
        units,
        &mut item,
    );
    let comment = comment.iter().zip(&code).map(|(&c, &k)| c && !k).collect();
    Reading { item, comment }
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
    /// Files sundgren read (Python, Rust, C): their lines have items and comment marks.
    pub files_read: usize,
    #[serde(skip)]
    added: Vec<Line>,
    #[serde(skip)]
    units: Vec<Unit>,
    #[serde(skip)]
    read: HashSet<String>,
}

/// One file's share of a direction.
#[derive(Debug, Serialize, PartialEq)]
pub struct FileShare {
    pub path: String,
    pub lines: usize,
    pub exact: usize,
    pub fuzzy: usize,
}

/// Where most of an item's found lines are on the other side.
#[derive(Debug, Serialize, PartialEq)]
pub struct Counterpart {
    pub path: String,
    /// The item there; left out if the lines are outside items, or in a file sundgren doesn't
    /// read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// How many of the item's found lines are there.
    pub lines: usize,
}

/// One item's share of a direction, in a file sundgren reads.
#[derive(Debug, Serialize, PartialEq)]
pub struct ItemShare {
    pub path: String,
    /// The item's full name; left out for the lines of the file outside any item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    pub lines: usize,
    pub exact: usize,
    pub fuzzy: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterpart: Option<Counterpart>,
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
    /// Lines that are only a comment. This and the rest are left out if sundgren read none of
    /// the side's files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_lines: Option<usize>,
    /// Containment of the lines that are not only a comment; also left out if there are none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_containment: Option<f64>,
    /// Per item of the files sundgren read, in order of first line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ItemShare>,
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

    let readable: Vec<(&str, Language)> = files
        .iter()
        .filter_map(|f| Language::from_path(&f.path).map(|l| (f.path.as_str(), l)))
        .collect();
    let readable_paths: Vec<&str> = readable.iter().map(|&(path, _)| path).collect();
    let contents = git::files_at(repo, &oid, &readable_paths)?;
    let mut units = Vec::new();
    let mut readings: HashMap<&str, Reading> = HashMap::new();
    for (&(path, language), content) in readable.iter().zip(contents) {
        if let Some(bytes) = content {
            let source = String::from_utf8_lossy(&bytes);
            readings.insert(path, read(language, &source, &mut units));
        }
    }

    let added: Vec<Line> = files
        .iter()
        .flat_map(|file| {
            let reading = readings.get(file.path.as_str());
            file.added.iter().filter_map(move |(number, raw)| {
                let number = *number as usize;
                similarity::normalize(raw).map(|text| Line {
                    path: file.path.clone(),
                    hash: similarity::line_hash(&text),
                    text,
                    item: reading.and_then(|r| r.item.get(number).copied().flatten()),
                    comment: reading.is_some_and(|r| r.comment.get(number) == Some(&true)),
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
        files_read: readings.len(),
        read: readings.keys().map(|&path| path.to_owned()).collect(),
        added,
        units,
    })
}

fn ratio(part: usize, whole: usize) -> Option<f64> {
    (whole > 0).then(|| part as f64 / whole as f64)
}

/// A's lines per item, with where in B their found lines are.
fn items(a: &Side, b: &Side, found: &[Found]) -> Vec<ItemShare> {
    type Place = (String, Option<usize>);
    let mut shares: Vec<ItemShare> = Vec::new();
    let mut landed: Vec<HashMap<Place, usize>> = Vec::new();
    let mut index: HashMap<Place, usize> = HashMap::new();
    for (line, found) in a.added.iter().zip(found) {
        if !a.read.contains(&line.path) {
            continue;
        }
        let slot = *index
            .entry((line.path.clone(), line.item))
            .or_insert_with(|| {
                let unit = line.item.map(|i| &a.units[i]);
                shares.push(ItemShare {
                    path: line.path.clone(),
                    item: unit.map(|u| u.name.clone()),
                    kind: unit.map(|u| u.kind),
                    lines: 0,
                    exact: 0,
                    fuzzy: 0,
                    counterpart: None,
                });
                landed.push(HashMap::new());
                shares.len() - 1
            });
        let share = &mut shares[slot];
        share.lines += 1;
        match found {
            Found::Exact { .. } => share.exact += 1,
            Found::Fuzzy { .. } => share.fuzzy += 1,
            Found::Missing => {}
        }
        if let Some(there) = found.at().and_then(|at| b.added.get(at)) {
            *landed[slot]
                .entry((there.path.clone(), there.item))
                .or_default() += 1;
        }
    }
    for (share, places) in shares.iter_mut().zip(landed) {
        // The most lines; on a tie, the first place in order, so the result is stable.
        share.counterpart = places
            .into_iter()
            .max_by(|(p, n), (q, m)| n.cmp(m).then_with(|| q.cmp(p)))
            .map(|((path, item), lines)| Counterpart {
                path,
                item: item.map(|i| b.units[i].name.clone()),
                lines,
            });
    }
    shares
}

fn direction(a_side: &Side, b_side: &Side, threshold: f64) -> Direction {
    let (a, b) = (a_side.added.as_slice(), b_side.added.as_slice());
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
                Found::Exact { .. } => file.exact += 1,
                Found::Fuzzy { .. } => file.fuzzy += 1,
                Found::Missing => {}
            }
        }
    }
    let exact = files.iter().map(|f| f.exact).sum();
    let fuzzy = files.iter().map(|f| f.fuzzy).sum();
    let read = a_side.files_read > 0;
    let comments = a.iter().filter(|l| l.comment).count();
    let code_found = a
        .iter()
        .zip(&found)
        .filter(|(l, f)| !l.comment && **f != Found::Missing)
        .count();
    Direction {
        lines: a.len(),
        exact,
        fuzzy,
        in_order,
        containment: ratio(exact + fuzzy, a.len()),
        exact_containment: ratio(exact, a.len()),
        ordered_containment: ratio(in_order, a.len()),
        files,
        comment_lines: read.then_some(comments),
        code_containment: ratio(code_found, a.len() - comments).filter(|_| read),
        items: items(a_side, b_side, &found),
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
    let a_in_b = direction(&a, &b, threshold);
    let b_in_a = direction(&b, &a, threshold);
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
    for (label, d) in [("A in B", &doc.a_in_b), ("B in A", &doc.b_in_a)] {
        if let Some(comments) = d.comment_lines {
            out.push_str(&format!(
                "{label} without comment lines: {} ({comments} comment lines)\n",
                percent(d.code_containment)
            ));
        }
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
    for (label, d) in [("A in B", &doc.a_in_b), ("B in A", &doc.b_in_a)] {
        if d.items.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{label}, per item:\n"));
        for item in &d.items {
            let name = |path: &str, item: Option<&String>| {
                format!("{path} {}", item.map_or("(outside items)", String::as_str))
            };
            // The counterpart's path only if it is another file.
            let there = item.counterpart.as_ref().map_or_else(String::new, |c| {
                if c.path == item.path {
                    format!(
                        "  -> {}",
                        c.item.as_ref().map_or("(outside items)", String::as_str)
                    )
                } else {
                    format!("  -> {}", name(&c.path, c.item.as_ref()))
                }
            });
            out.push_str(&format!(
                "  {:>4}% {:>5} lines  {}{there}\n",
                ((item.exact + item.fuzzy) * 100)
                    .checked_div(item.lines)
                    .unwrap_or(0),
                item.lines,
                name(&item.path, item.item.as_ref()),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests;
