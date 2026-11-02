//! `sundgren origin`: which file of another repository, at chosen commits, a file's lines come
//! from.
//!
//! A file's lines are normalized as for `contain` (lines that are only a comment left out in
//! Python, Rust and C files). At each reference commit of the other repository, the files with
//! the same extension are ranked by the lines they share exactly with it, rarer lines weighing
//! more; for the first few, the share of the file's lines found in them (exact or fuzzy) is
//! measured. The best one is the likely source. Its lead over the second says how clear that is:
//! boilerplate is found in many files.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::contain::{self, At, Comments, LineFound, Unit};
use crate::similarity::{self, Found};
use crate::syntax::Language;
use crate::{Error, Result, git};

pub const ORIGIN_SCHEMA: &str = "sundgren.origin/0";

/// Files measured in full at each reference, after ranking by shared lines.
const SHORTLIST: usize = 5;

/// A line in at most this many files of a reference is distinctive.
const DISTINCTIVE: usize = 2;

/// A file of a reference, its lines normalized.
struct Candidate {
    path: String,
    /// Line numbers of `texts`.
    numbers: Vec<u32>,
    texts: Vec<String>,
}

/// The files of the other repository at one commit, and in which files each line is.
struct Reference {
    rev: String,
    oid: String,
    files: Vec<Candidate>,
    /// Line hash to the files that have it, each once.
    index: HashMap<u64, Vec<usize>>,
}

/// One line of a file, normalized.
struct Line {
    number: u32,
    raw: String,
    text: String,
    hash: u64,
    item: Option<String>,
    comment: bool,
}

/// How much of a file is found in one file of a reference.
#[derive(Debug, Serialize, PartialEq)]
pub struct Source {
    pub path: String,
    pub exact: usize,
    pub fuzzy: usize,
    /// (exact + fuzzy) / the file's lines.
    pub share: f64,
    /// The file's lines found exactly here that are in at most two files of the reference, over
    /// the file's lines: the share boilerplate doesn't explain.
    pub distinctive: f64,
}

/// Where a file's lines come from at one reference.
#[derive(Debug, Serialize)]
pub struct Origin {
    pub rev: String,
    pub oid: String,
    /// The file most of its lines are found in; left out, like the rest, if none shares a line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second: Option<Source>,
    /// The best share less the second's (or the best share, without a second).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    /// Every line of the file and how it was found in the best file, with `--lines`: as
    /// `contain --lines` lists them, `at` in the best file at `oid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_found: Option<Vec<LineFound>>,
}

/// One file and where its lines come from, per reference.
#[derive(Debug, Serialize)]
pub struct FileOrigin {
    /// Relative to the repository.
    pub path: String,
    pub lines: usize,
    /// Lines that are only a comment; not in `lines` unless comments are counted.
    pub comment_lines: usize,
    /// One per reference, in the order given.
    pub origins: Vec<Origin>,
}

/// A reference commit of the other repository.
#[derive(Debug, Serialize)]
pub struct ReferenceInfo {
    pub rev: String,
    pub oid: String,
    /// Files compared with: those with an extension of the files asked about.
    pub files: usize,
}

#[derive(Debug, Serialize)]
pub struct OriginDocument {
    pub schema: &'static str,
    pub repository: PathBuf,
    pub other: PathBuf,
    pub threshold: f64,
    /// ignored or counted: whether lines that are only a comment are part of a file.
    pub comments: &'static str,
    pub references: Vec<ReferenceInfo>,
    pub files: Vec<FileOrigin>,
}

/// How to compare: as for `contain`, the fuzzy threshold (above 1: exact only), whether comment
/// lines count, and whether to list every line.
pub type Options = contain::Options;

/// The extension of the file at `path`, or its name if it has none: files compared share it.
fn kind(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.')
        .map_or(name, |(_, extension)| extension)
}

fn reference(other: &Path, rev: &str, kinds: &HashSet<&str>) -> Result<Reference> {
    let oid = git::resolve(other, rev)?;
    let paths: Vec<String> = git::tree_paths(other, &oid)?
        .into_iter()
        .filter(|path| kinds.contains(kind(path)))
        .collect();
    let names: Vec<&str> = paths.iter().map(String::as_str).collect();
    let mut files = Vec::new();
    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
    for (path, content) in paths.iter().zip(git::files_at(other, &oid, &names)?) {
        let Some(bytes) = content else {
            continue;
        };
        let source = String::from_utf8_lossy(&bytes);
        let mut candidate = Candidate {
            path: path.clone(),
            numbers: Vec::new(),
            texts: Vec::new(),
        };
        let mut seen = HashSet::new();
        for (number, raw) in (1..).zip(source.lines()) {
            if let Some(text) = similarity::normalize(raw) {
                let hash = similarity::line_hash(&text);
                if seen.insert(hash) {
                    index.entry(hash).or_default().push(files.len());
                }
                candidate.numbers.push(number);
                candidate.texts.push(text);
            }
        }
        files.push(candidate);
    }
    Ok(Reference {
        rev: rev.to_owned(),
        oid,
        files,
        index,
    })
}

/// The lines of `source` at `path`, with items and comment marks in files sundgren reads.
fn lines(path: &str, source: &str) -> Vec<Line> {
    let mut units: Vec<Unit> = Vec::new();
    let reading = Language::from_path(path).map(|l| contain::read(l, source, &mut units));
    (1..)
        .zip(source.lines())
        .filter_map(|(number, raw): (u32, &str)| {
            let at = number as usize;
            similarity::normalize(raw).map(|text| Line {
                number,
                raw: raw.trim_end().to_owned(),
                hash: similarity::line_hash(&text),
                text,
                item: reading
                    .as_ref()
                    .and_then(|r| r.item.get(at).copied().flatten())
                    .map(|i| units[i].name.clone()),
                comment: reading
                    .as_ref()
                    .is_some_and(|r| r.comment.get(at) == Some(&true)),
            })
        })
        .collect()
}

/// The files of `reference` like `path` (see [`kind`]) sharing the most lines with `lines`, a
/// line in fewer files weighing more; at most [`SHORTLIST`], none without a shared line.
fn shortlist(path: &str, lines: &[Line], reference: &Reference) -> Vec<usize> {
    let wanted = kind(path);
    let mut weight: HashMap<usize, f64> = HashMap::new();
    let distinct: HashSet<u64> = lines.iter().map(|l| l.hash).collect();
    for hash in distinct {
        if let Some(files) = reference.index.get(&hash) {
            for &file in files {
                if kind(&reference.files[file].path) == wanted {
                    *weight.entry(file).or_default() += 1.0 / files.len() as f64;
                }
            }
        }
    }
    let mut ranked: Vec<(usize, f64)> = weight.into_iter().collect();
    ranked.sort_by(|(a, x), (b, y)| y.total_cmp(x).then(a.cmp(b)));
    ranked
        .into_iter()
        .take(SHORTLIST)
        .map(|(file, _)| file)
        .collect()
}

/// How much of `lines` is in `candidate`.
fn measure(
    lines: &[Line],
    texts: &[String],
    candidate: &Candidate,
    reference: &Reference,
    threshold: f64,
) -> (Source, Vec<Found>) {
    let found = similarity::find_lines(texts, &candidate.texts, threshold);
    let mut source = Source {
        path: candidate.path.clone(),
        exact: 0,
        fuzzy: 0,
        share: 0.0,
        distinctive: 0.0,
    };
    let mut distinctive = 0;
    for (line, found) in lines.iter().zip(&found) {
        match found {
            Found::Exact { .. } => {
                source.exact += 1;
                if reference
                    .index
                    .get(&line.hash)
                    .is_some_and(|files| files.len() <= DISTINCTIVE)
                {
                    distinctive += 1;
                }
            }
            Found::Fuzzy { .. } => source.fuzzy += 1,
            Found::Missing => {}
        }
    }
    let whole = lines.len().max(1) as f64;
    source.share = (source.exact + source.fuzzy) as f64 / whole;
    source.distinctive = f64::from(distinctive) / whole;
    (source, found)
}

/// Every line of the file, in order: how it was found in `candidate`, or that it is a comment
/// left out.
fn lines_found(
    counted: &[Line],
    found: &[Found],
    ignored: &[Line],
    path: &str,
    candidate: &Candidate,
) -> Vec<LineFound> {
    let detail = |line: &Line, found: &'static str| LineFound {
        path: path.to_owned(),
        line: line.number,
        text: line.raw.clone(),
        item: line.item.clone(),
        found,
        score: None,
        edits: None,
        at: None,
    };
    let mut out: Vec<LineFound> = counted
        .iter()
        .zip(found)
        .map(|(line, found)| {
            let mut d = match found {
                Found::Exact { .. } => detail(line, "exact"),
                Found::Fuzzy { score, at } => LineFound {
                    score: Some(*score),
                    edits: Some(similarity::edits(&line.text, &candidate.texts[*at])),
                    ..detail(line, "fuzzy")
                },
                Found::Missing => detail(line, "missing"),
            };
            d.at = found.at().map(|at| At {
                path: candidate.path.clone(),
                line: candidate.numbers[at],
            });
            d
        })
        .chain(ignored.iter().map(|line| detail(line, "comment")))
        .collect();
    out.sort_by_key(|d| d.line);
    out
}

fn origin(
    path: &str,
    counted: &[Line],
    ignored: &[Line],
    reference: &Reference,
    options: Options,
) -> Origin {
    let texts: Vec<String> = counted.iter().map(|l| l.text.clone()).collect();
    let mut measured: Vec<(Source, Vec<Found>, usize)> = shortlist(path, counted, reference)
        .into_iter()
        .map(|file| {
            let candidate = &reference.files[file];
            let (source, found) = measure(counted, &texts, candidate, reference, options.threshold);
            (source, found, file)
        })
        .collect();
    measured.sort_by(|(a, _, _), (b, _, _)| {
        b.share
            .total_cmp(&a.share)
            .then(b.distinctive.total_cmp(&a.distinctive))
            .then(a.path.cmp(&b.path))
    });
    let mut measured = measured.into_iter();
    let best = measured.next();
    let second = measured.next().map(|(source, _, _)| source);
    let gap = best
        .as_ref()
        .map(|(b, _, _)| b.share - second.as_ref().map_or(0.0, |s| s.share));
    let lines_found = best
        .as_ref()
        .filter(|_| options.lines)
        .map(|(_, found, file)| {
            lines_found(counted, found, ignored, path, &reference.files[*file])
        });
    Origin {
        rev: reference.rev.clone(),
        oid: reference.oid.clone(),
        best: best.map(|(source, _, _)| source),
        second,
        gap,
        lines_found,
    }
}

/// The path of `file` relative to `top`, which must contain it.
fn relative(top: &Path, file: &Path) -> Result<String> {
    let full = file
        .canonicalize()
        .map_err(|e| Error::usage(format!("{}: {e}", file.display())))?;
    full.strip_prefix(top)
        .ok()
        .and_then(Path::to_str)
        .map(|p| p.replace('\\', "/"))
        .ok_or_else(|| {
            Error::usage(format!(
                "{} is not in the repository at {}",
                file.display(),
                top.display()
            ))
        })
}

/// Where the lines of `files` (in the work tree of the repository at `repo`) come from in the
/// repository at `other`, at each of `revs`.
pub fn origin_document(
    repo: &Path,
    files: &[PathBuf],
    other: &Path,
    revs: &[String],
    options: Options,
) -> Result<OriginDocument> {
    let repository = git::toplevel(repo)?;
    let top = repository.canonicalize()?;
    let other = git::toplevel(other)?;
    let mut read = Vec::new();
    for file in files {
        let path = relative(&top, file)?;
        let bytes =
            std::fs::read(file).map_err(|e| Error::usage(format!("{}: {e}", file.display())))?;
        let source = String::from_utf8_lossy(&bytes).into_owned();
        read.push((path, source));
    }
    let kinds: HashSet<&str> = read.iter().map(|(path, _)| kind(path)).collect();
    let references = revs
        .iter()
        .map(|rev| reference(&other, rev, &kinds))
        .collect::<Result<Vec<_>>>()?;
    let files = read
        .iter()
        .map(|(path, source)| {
            let all = lines(path, source);
            let comment_lines = all.iter().filter(|l| l.comment).count();
            let (counted, ignored): (Vec<Line>, Vec<Line>) = match options.comments {
                Comments::Count => (all, Vec::new()),
                Comments::Ignore => all.into_iter().partition(|l| !l.comment),
            };
            let origins = references
                .iter()
                .map(|reference| origin(path, &counted, &ignored, reference, options))
                .collect();
            FileOrigin {
                path: path.clone(),
                lines: counted.len(),
                comment_lines,
                origins,
            }
        })
        .collect();
    Ok(OriginDocument {
        schema: ORIGIN_SCHEMA,
        repository,
        other,
        threshold: options.threshold,
        comments: match options.comments {
            Comments::Ignore => "ignored",
            Comments::Count => "counted",
        },
        references: references
            .iter()
            .map(|r| ReferenceInfo {
                rev: r.rev.clone(),
                oid: r.oid.clone(),
                files: r.files.len(),
            })
            .collect(),
        files,
    })
}

fn percent(value: f64) -> String {
    format!("{:.0}%", value * 100.0)
}

/// A short human summary: per file, per reference, the best source, its share and its lead.
pub fn text(doc: &OriginDocument) -> String {
    let mut out = String::new();
    let short = |oid: &str| oid.chars().take(8).collect::<String>();
    for file in &doc.files {
        out.push_str(&format!("{}: {} lines\n", file.path, file.lines));
        for origin in &file.origins {
            let at = format!("{} ({})", origin.rev, short(&origin.oid));
            let Some(best) = &origin.best else {
                out.push_str(&format!("  {at}: no file shares a line\n"));
                continue;
            };
            out.push_str(&format!(
                "  {at}: {} from {} ({} distinctive)",
                percent(best.share),
                best.path,
                percent(best.distinctive),
            ));
            match &origin.second {
                Some(second) => out.push_str(&format!(
                    ", ahead by {} of {} ({})\n",
                    percent(origin.gap.unwrap_or_default()),
                    second.path,
                    percent(second.share),
                )),
                None => out.push_str(", no other file shares a line\n"),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
