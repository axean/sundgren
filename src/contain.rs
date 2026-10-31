//! `sundgren contain`: how much of one branch's work is in another's, both ways.
//!
//! A branch's work is the lines it added since it left the base: `git diff merge-base(base,
//! branch) branch`, normalized (see [`similarity::normalize`]). A line of A counts as found in B
//! if B added the same line, or (fuzzy) a line at least `threshold` similar.
//!
//! Files in a language sundgren reads (see [`syntax`]) add to that: each added line gets the item
//! around it and whether it is only a comment. Lines that are only a comment are left out of the
//! work unless comments are counted ([`Comments`]), and containment is also given per item, with
//! the item of the other side most of its lines were found in.

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

/// Whether lines that are only a comment are part of a branch's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comments {
    /// Left out (the default): license headers and other comments are not work.
    Ignore,
    Count,
}

/// One added line, normalized.
#[derive(Debug, Clone)]
struct Line {
    path: String,
    /// Its line number in the branch's file, and the line as it is there.
    number: u32,
    raw: String,
    text: String,
    hash: u64,
    /// The innermost item around it, an index into its side's items. None outside items, and in
    /// files sundgren doesn't read.
    item: Option<usize>,
    /// Only a comment (known only in files sundgren reads).
    comment: bool,
}

/// An item of a file sundgren read: its full name (`Class.method`, `Type::method`), kind and
/// lines.
#[derive(Debug, Clone)]
struct Unit {
    name: String,
    kind: &'static str,
    start: usize,
    end: usize,
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
                start: it.start,
                end: it.end,
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
    /// Commits since then.
    pub commits: usize,
    pub files: usize,
    /// Added lines kept after normalization: the work.
    pub lines: usize,
    /// Added lines that are only a comment, in files sundgren reads. Not in `lines` unless
    /// comments are counted.
    pub comment_lines: usize,
    /// Files sundgren read (Python, Rust, C): their lines have items and comment marks.
    pub files_read: usize,
    #[serde(skip)]
    added: Vec<Line>,
    /// Comment lines left out of `added`.
    #[serde(skip)]
    ignored: Vec<Line>,
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
    /// The item's first and last line, if there is an item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
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
    /// The item's first and last line, if there is an item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
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
    /// Per item of the files sundgren read, in order of first line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ItemShare>,
    /// Every added line and how it was found, with `--lines`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_found: Option<Vec<LineFound>>,
}

/// A line of a file at the other side's tip.
#[derive(Debug, Serialize, PartialEq)]
pub struct At {
    pub path: String,
    pub line: u32,
}

/// One added line and how it was found on the other side.
#[derive(Debug, Serialize, PartialEq)]
pub struct LineFound {
    pub path: String,
    pub line: u32,
    /// The line as it is in the file.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// exact, fuzzy, missing, or comment (only a comment, left out of the work).
    pub found: &'static str,
    /// The similarity of a fuzzy match: 1 - edits / the longer line's length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// The characters to insert, delete or replace to turn one line into the other, with
    /// whitespace collapsed (Levenshtein distance), for a fuzzy match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edits: Option<usize>,
    /// Where it was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
}

#[derive(Debug, Serialize)]
pub struct ContainmentDocument {
    pub schema: &'static str,
    pub repository: PathBuf,
    pub base: String,
    pub base_oid: String,
    pub threshold: f64,
    /// ignored or counted: whether lines that are only a comment are part of the work.
    pub comments: &'static str,
    pub a: Side,
    pub b: Side,
    /// How much of A's work is in B.
    pub a_in_b: Direction,
    pub b_in_a: Direction,
    /// same, a-in-b, b-in-a, overlap, unrelated, or empty.
    pub relation: &'static str,
}

fn side(repo: &Path, base_oid: &str, rev: &str, comments: Comments) -> Result<Side> {
    let oid = git::resolve(repo, rev)?;
    let fork = git::merge_base(repo, base_oid, &oid)?
        .ok_or_else(|| Error::usage(format!("{rev} has no common ancestor with the base")))?;
    let files = git::added_lines(repo, &fork, &oid)?;
    let commits = git::commit_count(repo, &fork, &oid)?;

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
                let at = *number as usize;
                similarity::normalize(raw).map(|text| Line {
                    path: file.path.clone(),
                    number: *number,
                    raw: raw.trim_end().to_owned(),
                    hash: similarity::line_hash(&text),
                    text,
                    item: reading.and_then(|r| r.item.get(at).copied().flatten()),
                    comment: reading.is_some_and(|r| r.comment.get(at) == Some(&true)),
                })
            })
        })
        .collect();
    let comment_lines = added.iter().filter(|l| l.comment).count();
    let (added, ignored): (Vec<Line>, Vec<Line>) = match comments {
        Comments::Count => (added, Vec::new()),
        Comments::Ignore => added.into_iter().partition(|l| !l.comment),
    };
    let mut paths: Vec<&str> = added.iter().map(|l| l.path.as_str()).collect();
    paths.dedup();
    Ok(Side {
        rev: rev.to_owned(),
        oid,
        fork,
        commits,
        files: paths.len(),
        lines: added.len(),
        comment_lines,
        files_read: readings.len(),
        read: readings.keys().map(|&path| path.to_owned()).collect(),
        added,
        ignored,
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
                    start: unit.map(|u| u.start),
                    end: unit.map(|u| u.end),
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
            .map(|((path, item), lines)| {
                let unit = item.map(|i| &b.units[i]);
                Counterpart {
                    path,
                    item: unit.map(|u| u.name.clone()),
                    start: unit.map(|u| u.start),
                    end: unit.map(|u| u.end),
                    lines,
                }
            });
    }
    shares
}

/// Every line of A, in file order: how it was found in B, or that it is a comment left out.
fn lines_found(a: &Side, b: &Side, found: &[Found]) -> Vec<LineFound> {
    let detail = |line: &Line, found: &'static str| LineFound {
        path: line.path.clone(),
        line: line.number,
        text: line.raw.clone(),
        item: line.item.map(|i| a.units[i].name.clone()),
        found,
        score: None,
        edits: None,
        at: None,
    };
    let mut out: Vec<(usize, LineFound)> = Vec::new();
    // Lines keep the diff's file order; within a file, line numbers order them.
    let file_order: HashMap<&str, usize> = a
        .added
        .iter()
        .chain(&a.ignored)
        .map(|l| l.path.as_str())
        .fold(HashMap::new(), |mut order, path| {
            let next = order.len();
            order.entry(path).or_insert(next);
            order
        });
    for (line, found) in a.added.iter().zip(found) {
        let mut d = match found {
            Found::Exact { .. } => detail(line, "exact"),
            Found::Fuzzy { score, at } => LineFound {
                score: Some(*score),
                edits: b
                    .added
                    .get(*at)
                    .map(|there| similarity::edits(&line.text, &there.text)),
                ..detail(line, "fuzzy")
            },
            Found::Missing => detail(line, "missing"),
        };
        d.at = found.at().and_then(|at| b.added.get(at)).map(|there| At {
            path: there.path.clone(),
            line: there.number,
        });
        out.push((file_order[line.path.as_str()], d));
    }
    for line in &a.ignored {
        out.push((file_order[line.path.as_str()], detail(line, "comment")));
    }
    out.sort_by_key(|(file, d)| (*file, d.line));
    out.into_iter().map(|(_, d)| d).collect()
}

fn direction(a_side: &Side, b_side: &Side, threshold: f64, detail: bool) -> Direction {
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
    Direction {
        lines: a.len(),
        exact,
        fuzzy,
        in_order,
        containment: ratio(exact + fuzzy, a.len()),
        exact_containment: ratio(exact, a.len()),
        ordered_containment: ratio(in_order, a.len()),
        files,
        items: items(a_side, b_side, &found),
        lines_found: detail.then(|| lines_found(a_side, b_side, &found)),
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

/// How to compare: the fuzzy threshold (above 1: exact only), whether comment lines are work,
/// and whether to list every line and how it was found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub threshold: f64,
    pub comments: Comments,
    pub lines: bool,
}

impl Options {
    /// Fuzzy matching at `threshold`, comments ignored, no line list.
    pub fn new(threshold: f64) -> Self {
        Options {
            threshold,
            comments: Comments::Ignore,
            lines: false,
        }
    }
}

/// Compare the work of `a` and `b` since each left `base`.
pub fn containment_document(
    repo: &Path,
    base: &str,
    a: &str,
    b: &str,
    options: Options,
) -> Result<ContainmentDocument> {
    let repository = git::toplevel(repo)?;
    let base_oid = git::resolve(&repository, base)?;
    let a = side(&repository, &base_oid, a, options.comments)?;
    let b = side(&repository, &base_oid, b, options.comments)?;
    let threshold = options.threshold;
    let a_in_b = direction(&a, &b, threshold, options.lines);
    let b_in_a = direction(&b, &a, threshold, options.lines);
    let relation = relation(a_in_b.containment, b_in_a.containment);
    Ok(ContainmentDocument {
        schema: CONTAINMENT_SCHEMA,
        repository,
        base: base.to_owned(),
        base_oid,
        threshold,
        comments: match options.comments {
            Comments::Ignore => "ignored",
            Comments::Count => "counted",
        },
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
        "base {} ({}), fuzzy threshold {}, comments {}\n",
        doc.base,
        short(&doc.base_oid),
        doc.threshold,
        doc.comments
    ));
    for (name, side) in [("A", &doc.a), ("B", &doc.b)] {
        out.push_str(&format!(
            "{name} {} ({}): {} lines in {} files, {} commits since {} ({} comment lines)\n",
            side.rev,
            short(&side.oid),
            side.lines,
            side.files,
            side.commits,
            short(&side.fork),
            side.comment_lines
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
