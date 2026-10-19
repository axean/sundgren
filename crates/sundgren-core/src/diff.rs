//! (Snapshots, Snapshots, LineMaps, outcomes) -> Delta. Port of `sundgren/core/diff.py`.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::hashing::sha256_file_or_none;
use crate::linemap::{LineMap, identity, linemap_for};
use crate::model::{BranchKey, Count, FileCoverage, LineNo, Outcome, ReportWarning, Snapshot};
use crate::pytext::PyText;
use crate::rollup::function_deltas;
use crate::rollup::{FunctionDelta, FunctionStatus, LineSets, Lines, Stats, TreeNode, build_tree};
use crate::symbols::{Symbolizer, SymbolizerError, resolve_spans};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    Base,
    Patched,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Base => "base",
            Side::Patched => "patched",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineCategory {
    KeptHit,
    KeptMiss,
    Lost,
    Gained,
    Unmeasured,
    NewHit,
    NewMiss,
    DeletedHit,
    DeletedMiss,
}

impl LineCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            LineCategory::KeptHit => "kept_hit",
            LineCategory::KeptMiss => "kept_miss",
            LineCategory::Lost => "lost",
            LineCategory::Gained => "gained",
            LineCategory::Unmeasured => "unmeasured",
            LineCategory::NewHit => "new_hit",
            LineCategory::NewMiss => "new_miss",
            LineCategory::DeletedHit => "deleted_hit",
            LineCategory::DeletedMiss => "deleted_miss",
        }
    }

    pub fn is_kept(self) -> bool {
        matches!(self, LineCategory::KeptHit | LineCategory::KeptMiss)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BranchCategory {
    KeptHit,
    KeptMiss,
    Lost,
    Gained,
    New,
    Deleted,
    Flaky,
    Unmeasured,
}

impl BranchCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            BranchCategory::KeptHit => "branch_kept_hit",
            BranchCategory::KeptMiss => "branch_kept_miss",
            BranchCategory::Lost => "branch_lost",
            BranchCategory::Gained => "branch_gained",
            BranchCategory::New => "branch_new",
            BranchCategory::Deleted => "branch_deleted",
            BranchCategory::Flaky => "branch_flaky",
            BranchCategory::Unmeasured => "branch_unmeasured",
        }
    }

    pub fn is_kept(self) -> bool {
        matches!(self, BranchCategory::KeptHit | BranchCategory::KeptMiss)
    }
}

/// (base, patched); None = absent side (or '-' for branches).
pub type Pair = (Option<Count>, Option<Count>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineDelta {
    /// Patched line; None if deleted.
    pub line: Option<LineNo>,
    /// None if added.
    pub base_line: Option<LineNo>,
    pub category: LineCategory,
    pub hits: Pair,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BranchDelta {
    pub line: Option<LineNo>,
    pub base_line: Option<LineNo>,
    /// Patched-side ids, base side if there is no patched branch.
    pub block: String,
    pub branch: String,
    pub category: BranchCategory,
    pub taken: Pair,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlakyLine {
    pub line: Option<LineNo>,
    pub base_line: Option<LineNo>,
    /// From run 0.
    pub hits: Pair,
    /// Side(s) whose repeats disagreed.
    pub sides: Vec<Side>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileDelta {
    /// Patched path, base path if deleted.
    pub path: PyText,
    /// None for a new file.
    pub base_path: Option<PyText>,
    pub sha256: (Option<String>, Option<String>),
    pub source_changed: bool,
    pub inferred_side: Option<Side>,
    /// kept_* included; flaky lines are only in `flaky`.
    pub lines: Vec<LineDelta>,
    pub branches: Vec<BranchDelta>,
    pub functions: Vec<FunctionDelta>,
    pub flaky: Vec<FlakyLine>,
    pub stats: Stats,
}

impl FileDelta {
    /// Anything beyond kept_* lines and branches.
    pub fn changed(&self) -> bool {
        self.lines.iter().any(|e| !e.category.is_kept())
            || self.branches.iter().any(|b| !b.category.is_kept())
            || self
                .functions
                .iter()
                .any(|f| f.status != FunctionStatus::Unchanged)
            || !self.flaky.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutcomePair {
    pub id: String,
    pub base: Outcome,
    pub patched: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Delta {
    /// Every non-external file on either side, sorted by path.
    pub files: Vec<FileDelta>,
    /// Every test, sorted by id.
    pub tests: Vec<OutcomePair>,
    pub tree: TreeNode,
    pub external_files: i64,
    pub warnings: Vec<ReportWarning>,
}

#[derive(Debug)]
pub enum DiffError {
    /// A side has no snapshot (ValueError in Python).
    NoSnapshots,
    /// A symbolizer failed with something other than a syntax error.
    Symbolizer(SymbolizerError),
    /// Reading a source file for its sha256 failed.
    Io(std::io::Error),
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::NoSnapshots => f.write_str("diff needs at least one snapshot per side"),
            DiffError::Symbolizer(e) => write!(f, "{e}"),
            DiffError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DiffError {}

/// Finds the registered symbolizer for a path, if any.
pub type SymbolizerLookup<'a> = &'a dyn Fn(&PyText) -> Option<Box<dyn Symbolizer + 'a>>;

/// Inputs besides the snapshots and line maps; the defaults mean "unknown".
#[derive(Default, Clone, Copy)]
pub struct DiffOptions<'a> {
    pub base_root: Option<&'a Path>,
    pub patched_root: Option<&'a Path>,
    pub base_outcomes: Option<&'a BTreeMap<String, Outcome>>,
    pub patched_outcomes: Option<&'a BTreeMap<String, Outcome>>,
}

/// Classify every line, branch and function; [0] of each side is canonical, the rest repeats.
pub fn diff(
    base: &[Snapshot],
    patched: &[Snapshot],
    linemaps: &BTreeMap<PyText, LineMap>,
    opts: DiffOptions<'_>,
    symbolizers: SymbolizerLookup<'_>,
) -> Result<Delta, DiffError> {
    if base.is_empty() || patched.is_empty() {
        return Err(DiffError::NoSnapshots);
    }
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    for pair in pair_files(&base[0], &patched[0], linemaps, &mut warnings) {
        files.push(diff_file(
            &pair,
            base,
            patched,
            &opts,
            symbolizers,
            &mut warnings,
        )?);
    }
    files.sort_by(|a, b| {
        let empty = PyText::default();
        (&a.path, a.base_path.as_ref().unwrap_or(&empty))
            .cmp(&(&b.path, b.base_path.as_ref().unwrap_or(&empty)))
    });
    let external: HashSet<&str> = [&base[0], &patched[0]]
        .iter()
        .flat_map(|s| s.files.iter())
        .filter(|(_, fc)| fc.external)
        .map(|(p, _)| p.as_str())
        .collect();
    let tests = diff_outcomes(opts.base_outcomes, opts.patched_outcomes, &mut warnings);
    let tree = build_tree(files.iter().map(|f| (f.path.clone(), f.stats)));
    let warnings: BTreeSet<ReportWarning> = warnings.into_iter().collect();
    Ok(Delta {
        files,
        tests,
        tree,
        external_files: external.len() as i64,
        warnings: warnings.into_iter().collect(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Measured,
    /// Missing from coverage but the file exists: all MISS.
    Inferred,
    /// File not on this side.
    Nonexistent,
    /// Missing from coverage, existence unknown.
    Unmeasured,
}

struct FilePair<'a> {
    base_path: Option<PyText>,
    patched_path: Option<PyText>,
    lm: Cow<'a, LineMap>,
}

fn warning(code: &str, message: PyText) -> ReportWarning {
    ReportWarning {
        code: code.to_string(),
        message,
    }
}

fn internal(snapshot: &Snapshot) -> impl Iterator<Item = &String> {
    snapshot
        .files
        .iter()
        .filter(|(_, fc)| !fc.external)
        .map(|(p, _)| p)
}

fn pair_files<'a>(
    base: &Snapshot,
    patched: &Snapshot,
    linemaps: &'a BTreeMap<PyText, LineMap>,
    warnings: &mut Vec<ReportWarning>,
) -> Vec<FilePair<'a>> {
    let mut pairs: Vec<FilePair<'a>> = Vec::new();
    let mut index: HashMap<PyText, usize> = HashMap::new();
    let mut claimed: HashSet<PyText> = HashSet::new();
    for p in internal(patched) {
        let p = PyText::from(p.clone());
        let mut lm = linemap_for(linemaps, &p);
        if lm.new_path.as_ref() != Some(&p) {
            // The patch says p is gone; the snapshot wins.
            lm = Cow::Owned(identity(p.clone()));
        }
        if let Some(old) = &lm.old_path {
            claimed.insert(old.clone());
        }
        let pair = FilePair {
            base_path: lm.old_path.clone(),
            patched_path: Some(p.clone()),
            lm,
        };
        match index.get(&p) {
            Some(&i) => pairs[i] = pair,
            None => {
                index.insert(p, pairs.len());
                pairs.push(pair);
            }
        }
    }
    for b in internal(base) {
        let b = PyText::from(b.clone());
        if claimed.contains(&b) {
            continue;
        }
        let mut lm = linemap_for(linemaps, &b);
        if lm.old_path.as_ref() != Some(&b) {
            lm = Cow::Owned(identity(b.clone()));
        }
        let key = lm.new_path.clone().unwrap_or_else(|| b.clone());
        if index.contains_key(&key) {
            let mut msg = b.clone();
            msg.push_str(": base file collides with patched ");
            msg.push_text(&key);
            msg.push_str("; ignored");
            warnings.push(warning("unpaired_file", msg));
            continue;
        }
        index.insert(key, pairs.len());
        pairs.push(FilePair {
            base_path: Some(b),
            patched_path: lm.new_path.clone(),
            lm,
        });
    }
    pairs
}

/// `path` followed by `rest`.
fn cat(path: &PyText, rest: &str) -> PyText {
    let mut out = path.clone();
    out.push_str(rest);
    out
}

fn get<'s>(snapshot: &'s Snapshot, path: Option<&PyText>) -> Option<&'s FileCoverage> {
    let fc = snapshot.files.get(path?.as_str()?)?;
    (!fc.external).then_some(fc)
}

fn state(fc: Option<&FileCoverage>, path: Option<&PyText>, root: Option<&Path>) -> State {
    if fc.is_some() {
        return State::Measured;
    }
    let Some(path) = path else {
        return State::Nonexistent;
    };
    let Some(root) = root else {
        return State::Unmeasured;
    };
    // A path os.fsencode can't encode is not a file, as in pathlib.
    if path.to_os_path().is_some_and(|p| root.join(p).is_file()) {
        State::Inferred
    } else {
        State::Nonexistent
    }
}

/// Items "on" in some repeats but not all; a run without the file has nothing on.
fn unstable<T: Ord + Clone>(
    runs: &[Snapshot],
    path: Option<&PyText>,
    on: impl Fn(&FileCoverage) -> BTreeSet<T>,
) -> BTreeSet<T> {
    if runs.len() < 2 || path.is_none() {
        return BTreeSet::new();
    }
    let sets: Vec<BTreeSet<T>> = runs
        .iter()
        .map(|run| get(run, path).map(&on).unwrap_or_default())
        .collect();
    let union: BTreeSet<T> = sets.iter().flatten().cloned().collect();
    union
        .into_iter()
        .filter(|x| !sets.iter().all(|s| s.contains(x)))
        .collect()
}

fn hit_lines(fc: &FileCoverage) -> BTreeSet<LineNo> {
    fc.lines
        .iter()
        .filter(|(_, c)| **c > 0)
        .map(|(l, _)| *l)
        .collect()
}

fn taken_branches(fc: &FileCoverage) -> BTreeSet<BranchKey> {
    fc.branches
        .iter()
        .filter(|(_, t)| truthy(**t))
        .map(|(k, _)| k.clone())
        .collect()
}

/// Python truthiness of an optional count.
fn truthy(c: Option<Count>) -> bool {
    c.is_some_and(|c| c != 0)
}

type Branches = BTreeMap<BranchKey, Option<Count>>;
type Convert<'a> = &'a dyn Fn(LineNo) -> Option<LineNo>;

fn diff_file(
    pair: &FilePair<'_>,
    base_runs: &[Snapshot],
    patched_runs: &[Snapshot],
    opts: &DiffOptions<'_>,
    symbolizers: SymbolizerLookup<'_>,
    warnings: &mut Vec<ReportWarning>,
) -> Result<FileDelta, DiffError> {
    let lm: &LineMap = &pair.lm;
    let fc_b = get(&base_runs[0], pair.base_path.as_ref());
    let fc_p = get(&patched_runs[0], pair.patched_path.as_ref());
    let state_b = state(fc_b, pair.base_path.as_ref(), opts.base_root);
    let state_p = state(fc_p, pair.patched_path.as_ref(), opts.patched_root);
    let base_path = pair
        .base_path
        .clone()
        .filter(|_| state_b != State::Nonexistent);
    let patched_path = pair
        .patched_path
        .clone()
        .filter(|_| state_p != State::Nonexistent);
    let path = patched_path
        .clone()
        .or_else(|| base_path.clone())
        .expect("one side is always measured");
    let to_new_fn = |l: LineNo| lm.to_new(l);
    let to_old_fn = |l: LineNo| lm.to_old(l);
    let never = |_: LineNo| None;
    let to_new: Convert = if patched_path.is_some() {
        &to_new_fn
    } else {
        &never
    };
    let to_old: Convert = if base_path.is_some() {
        &to_old_fn
    } else {
        &never
    };

    let mut lines_b: Lines = fc_b.map(|f| f.lines.clone()).unwrap_or_default();
    let mut lines_p: Lines = fc_p.map(|f| f.lines.clone()).unwrap_or_default();
    let mut br_b: Branches = fc_b.map(|f| f.branches.clone()).unwrap_or_default();
    let mut br_p: Branches = fc_p.map(|f| f.branches.clone()).unwrap_or_default();
    let mut inferred_side = None;
    // Nothing executable on the measured side means nothing to infer: no flag, no warning.
    if state_b == State::Inferred && (!lines_p.is_empty() || !br_p.is_empty()) {
        inferred_side = Some(Side::Base);
        (lines_b, br_b) = inferred(&lines_p, &br_p, to_old);
    } else if state_p == State::Inferred && (!lines_b.is_empty() || !br_b.is_empty()) {
        inferred_side = Some(Side::Patched);
        (lines_p, br_p) = inferred(&lines_b, &br_b, to_new);
    }
    if let Some(side) = inferred_side {
        let s = side.as_str();
        let msg =
            format!(": not in {s} coverage but present in the {s} tree; counted as never executed");
        warnings.push(warning("inferred_file", cat(&path, &msg)));
    }

    let flaky_b = unstable(base_runs, base_path.as_ref(), hit_lines);
    let flaky_p = unstable(patched_runs, patched_path.as_ref(), hit_lines);
    let (entries, flaky) = diff_lines(&lines_b, &lines_p, to_new, to_old, &flaky_b, &flaky_p);
    let branches = diff_branches(
        &br_b,
        &br_p,
        to_new,
        to_old,
        &unstable(base_runs, base_path.as_ref(), taken_branches),
        &unstable(patched_runs, patched_path.as_ref(), taken_branches),
        inferred_side,
    );
    warn_unmeasured(&path, state_b, state_p, &entries, &branches, warnings);

    let spans = |path: &Option<PyText>, root, fc| -> Result<_, DiffError> {
        match path {
            None => Ok(Some(Vec::new())),
            Some(p) => {
                let sym = symbolizers(p);
                resolve_spans(p, root, fc, sym.as_deref()).map_err(DiffError::Symbolizer)
            }
        }
    };
    let base_spans = spans(&base_path, opts.base_root, fc_b)?;
    let patched_spans = spans(&patched_path, opts.patched_root, fc_p)?;
    let lines_in = |cat: LineCategory| -> BTreeSet<LineNo> {
        entries
            .iter()
            .filter(|e| e.category == cat)
            .filter_map(|e| e.line)
            .collect()
    };
    let sets = LineSets {
        lost: lines_in(LineCategory::Lost),
        gained: lines_in(LineCategory::Gained),
        flaky_base: flaky_b,
        flaky_patched: flaky_p,
    };
    let functions = function_deltas(
        base_spans.as_deref(),
        patched_spans.as_deref(),
        lm,
        &lines_b,
        &lines_p,
        &sets,
    );

    let sha = |root: Option<&Path>, path: &Option<PyText>| {
        let file = match (root, path) {
            (Some(r), Some(p)) if !p.is_empty() => p.to_os_path().map(|p| r.join(p)),
            _ => None,
        };
        sha256_file_or_none(file.as_deref()).map_err(DiffError::Io)
    };
    let sha_b = sha(opts.base_root, &base_path)?;
    let sha_p = sha(opts.patched_root, &patched_path)?;
    let source_changed = match (&sha_b, &sha_p) {
        (Some(b), Some(p)) => b != p,
        _ => {
            base_path.is_none()
                || patched_path.is_none()
                || !lm.deleted_old().is_empty()
                || !lm.added_new().is_empty()
        }
    };

    let n = |cat: LineCategory| entries.iter().filter(|e| e.category == cat).count() as i64;
    let hits = |lines: &Lines| lines.values().filter(|c| **c > 0).count() as i64;
    let stats = Stats {
        lost: n(LineCategory::Lost),
        gained: n(LineCategory::Gained),
        new_hit: n(LineCategory::NewHit),
        new_miss: n(LineCategory::NewMiss),
        deleted_hit: n(LineCategory::DeletedHit),
        deleted_miss: n(LineCategory::DeletedMiss),
        executable_base: lines_b.len() as i64,
        executable_patched: lines_p.len() as i64,
        hit_base: hits(&lines_b),
        hit_patched: hits(&lines_p),
    };
    Ok(FileDelta {
        path,
        base_path,
        sha256: (sha_b, sha_p),
        source_changed,
        inferred_side,
        lines: entries,
        branches,
        functions,
        flaky,
        stats,
    })
}

/// The other side's executable lines and branches, carried over as never executed.
fn inferred(lines: &Lines, branches: &Branches, convert: Convert) -> (Lines, Branches) {
    let out_lines = lines
        .keys()
        .filter_map(|&l| convert(l))
        .map(|n| (n, 0))
        .collect();
    let mut out_branches = Branches::new();
    for k in branches.keys() {
        if let (Some(n), Some(branch)) = (convert(k.line), remap_branch_id(&k.branch, convert)) {
            let key = BranchKey {
                line: n,
                block: k.block.clone(),
                branch,
            };
            out_branches.insert(key, Some(0));
        }
    }
    (out_lines, out_branches)
}

fn diff_lines(
    lines_b: &Lines,
    lines_p: &Lines,
    to_new: Convert,
    to_old: Convert,
    flaky_b: &BTreeSet<LineNo>,
    flaky_p: &BTreeSet<LineNo>,
) -> (Vec<LineDelta>, Vec<FlakyLine>) {
    let mut pairs: BTreeSet<(Option<LineNo>, Option<LineNo>)> = BTreeSet::new();
    for &b in lines_b.keys().chain(flaky_b) {
        pairs.insert((Some(b), to_new(b)));
    }
    for &p in lines_p.keys().chain(flaky_p) {
        pairs.insert((to_old(p), Some(p)));
    }
    let mut entries = Vec::new();
    let mut flaky = Vec::new();
    for (b, p) in pairs {
        let hits = (
            b.and_then(|b| lines_b.get(&b).copied()),
            p.and_then(|p| lines_p.get(&p).copied()),
        );
        let mut sides = Vec::new();
        if b.is_some_and(|b| flaky_b.contains(&b)) {
            sides.push(Side::Base);
        }
        if p.is_some_and(|p| flaky_p.contains(&p)) {
            sides.push(Side::Patched);
        }
        if !sides.is_empty() {
            flaky.push(FlakyLine {
                line: p,
                base_line: b,
                hits,
                sides,
            });
        } else {
            entries.push(LineDelta {
                line: p,
                base_line: b,
                category: line_category(b, p, hits.0, hits.1),
                hits,
            });
        }
    }
    entries.sort_by_key(|e| order(e.line, e.base_line));
    flaky.sort_by_key(|e| order(e.line, e.base_line));
    (entries, flaky)
}

fn line_category(
    b: Option<LineNo>,
    p: Option<LineNo>,
    hb: Option<Count>,
    hp: Option<Count>,
) -> LineCategory {
    if b.is_none() {
        return if truthy(hp) {
            LineCategory::NewHit
        } else {
            LineCategory::NewMiss
        };
    }
    if p.is_none() {
        return if truthy(hb) {
            LineCategory::DeletedHit
        } else {
            LineCategory::DeletedMiss
        };
    }
    let (Some(hb), Some(hp)) = (hb, hp) else {
        return LineCategory::Unmeasured;
    };
    match (hb > 0, hp > 0) {
        (true, true) => LineCategory::KeptHit,
        (true, false) => LineCategory::Lost,
        (false, true) => LineCategory::Gained,
        (false, false) => LineCategory::KeptMiss,
    }
}

static LINE_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bline (\d+)\b").unwrap());
static DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d+").unwrap());
static ND: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A\d\z").unwrap());

/// Value of a Unicode decimal digit, as Python's int() reads it.
fn digit_value(c: char) -> u32 {
    if c.is_ascii_digit() {
        return c as u32 - '0' as u32;
    }
    // Nd digits come in contiguous runs of 0..9; find the run's start.
    let is_nd =
        |cp: u32| char::from_u32(cp).is_some_and(|c| ND.is_match(c.encode_utf8(&mut [0; 4])));
    let mut zero = c as u32;
    while zero > 0 && is_nd(zero - 1) {
        zero -= 1;
    }
    (c as u32 - zero) % 10
}

/// A digit run as a line number; None if it doesn't fit.
fn parse_digits(s: &str) -> Option<LineNo> {
    s.chars().try_fold(0 as LineNo, |acc, c| {
        acc.checked_mul(10)?.checked_add(digit_value(c) as LineNo)
    })
}

/// Map "line N" refs in a branch id (coverage.py "jump to line N"); None if one doesn't map.
fn remap_branch_id(branch: &str, convert: Convert) -> Option<String> {
    let mut out = String::with_capacity(branch.len());
    let mut last = 0;
    for caps in LINE_REF.captures_iter(branch) {
        let m = caps.get(0).expect("whole match");
        let target = convert(parse_digits(&caps[1])?)?;
        out.push_str(&branch[last..m.start()]);
        out.push_str(&format!("line {target}"));
        last = m.end();
    }
    out.push_str(&branch[last..]);
    Some(out)
}

fn diff_branches(
    br_b: &Branches,
    br_p: &Branches,
    to_new: Convert,
    to_old: Convert,
    flaky_b: &BTreeSet<BranchKey>,
    flaky_p: &BTreeSet<BranchKey>,
    inferred_side: Option<Side>,
) -> Vec<BranchDelta> {
    // A one-sided branch on an unchanged line is unmeasured when the other side has no branch
    // data on that line, unless that side was inferred (it has exactly the copied branches).
    let keys_b: BTreeSet<&BranchKey> = br_b.keys().chain(flaky_b).collect();
    let keys_p: BTreeSet<&BranchKey> = br_p.keys().chain(flaky_p).collect();
    let lines_with_b: HashSet<LineNo> = keys_b.iter().map(|k| k.line).collect();
    let lines_with_p: HashSet<LineNo> = keys_p.iter().map(|k| k.line).collect();
    let mut matched: HashSet<BranchKey> = HashSet::new();
    let mut out = Vec::new();
    for kb in keys_b {
        let p = to_new(kb.line);
        let kp = p
            .and_then(|p| {
                let branch = remap_branch_id(&kb.branch, to_new)?;
                Some(BranchKey {
                    line: p,
                    block: kb.block.clone(),
                    branch,
                })
            })
            .filter(|kp| keys_p.contains(kp));
        let category = if let Some(kp) = &kp {
            matched.insert(kp.clone());
            branch_category(kb, kp, br_b, br_p, flaky_b, flaky_p)
        } else if flaky_b.contains(kb) {
            BranchCategory::Flaky
        } else if p.is_some_and(|p| !lines_with_p.contains(&p))
            && inferred_side != Some(Side::Patched)
        {
            BranchCategory::Unmeasured
        } else {
            BranchCategory::Deleted
        };
        let taken = (
            br_b.get(kb).copied().flatten(),
            kp.as_ref().and_then(|kp| br_p.get(kp).copied().flatten()),
        );
        let key = kp.as_ref().unwrap_or(kb);
        out.push(BranchDelta {
            line: p,
            base_line: Some(kb.line),
            block: key.block.clone(),
            branch: key.branch.clone(),
            category,
            taken,
        });
    }
    for kp in keys_p.into_iter().filter(|k| !matched.contains(*k)) {
        let b = to_old(kp.line);
        let category = if flaky_p.contains(kp) {
            BranchCategory::Flaky
        } else if b.is_some_and(|b| !lines_with_b.contains(&b)) && inferred_side != Some(Side::Base)
        {
            BranchCategory::Unmeasured
        } else {
            BranchCategory::New
        };
        out.push(BranchDelta {
            line: Some(kp.line),
            base_line: b,
            block: kp.block.clone(),
            branch: kp.branch.clone(),
            category,
            taken: (None, br_p.get(kp).copied().flatten()),
        });
    }
    out.sort_by(|a, b| {
        order(a.line, a.base_line)
            .cmp(&order(b.line, b.base_line))
            .then_with(|| natural_cmp(&a.block, &b.block))
            .then_with(|| natural_cmp(&a.branch, &b.branch))
            .then_with(|| a.category.as_str().cmp(b.category.as_str()))
    });
    out
}

fn branch_category(
    kb: &BranchKey,
    kp: &BranchKey,
    br_b: &Branches,
    br_p: &Branches,
    flaky_b: &BTreeSet<BranchKey>,
    flaky_p: &BTreeSet<BranchKey>,
) -> BranchCategory {
    if flaky_b.contains(kb) || flaky_p.contains(kp) {
        return BranchCategory::Flaky;
    }
    let hit_p = truthy(br_p.get(kp).copied().flatten());
    if truthy(br_b.get(kb).copied().flatten()) {
        if hit_p {
            BranchCategory::KeptHit
        } else {
            BranchCategory::Lost
        }
    } else if hit_p {
        BranchCategory::Gained
    } else {
        BranchCategory::KeptMiss
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NatPart<'a> {
    Text(&'a str),
    /// Digit values with leading zeros stripped; compared by length, then digits.
    Num(usize, Vec<u32>),
}

fn natural_key(s: &str) -> Vec<NatPart<'_>> {
    let mut out = Vec::new();
    let mut last = 0;
    for m in DIGITS.find_iter(s) {
        out.push(NatPart::Text(&s[last..m.start()]));
        let digits: Vec<u32> = m
            .as_str()
            .chars()
            .map(digit_value)
            .skip_while(|d| *d == 0)
            .collect();
        out.push(NatPart::Num(digits.len(), digits));
        last = m.end();
    }
    out.push(NatPart::Text(&s[last..]));
    out
}

/// Compare with digit runs as numbers ("line 9" < "line 10").
fn natural_cmp(a: &str, b: &str) -> Ordering {
    natural_key(a).cmp(&natural_key(b))
}

/// Patched lines in order, then deleted lines (line None) by base line.
fn order(line: Option<LineNo>, base_line: Option<LineNo>) -> (u8, LineNo, LineNo) {
    match line {
        None => (1, base_line.unwrap_or(0), 0),
        Some(l) => (0, l, base_line.unwrap_or(0)),
    }
}

fn warn_unmeasured(
    path: &PyText,
    state_b: State,
    state_p: State,
    lines: &[LineDelta],
    branches: &[BranchDelta],
    warnings: &mut Vec<ReportWarning>,
) {
    let n_lines = lines
        .iter()
        .filter(|e| e.category == LineCategory::Unmeasured)
        .count();
    let n_branches = branches
        .iter()
        .filter(|e| e.category == BranchCategory::Unmeasured)
        .count();
    if n_lines == 0 && n_branches == 0 {
        return;
    }
    let why = if state_b == State::Unmeasured || state_p == State::Unmeasured {
        let side = if state_b == State::Unmeasured {
            "base"
        } else {
            "patched"
        };
        format!("not in {side} coverage and the {side} tree is unknown")
    } else {
        "measured on one side only".to_string()
    };
    let msg = format!(": {n_lines} line(s), {n_branches} branch(es) unmeasured: {why}");
    warnings.push(warning("unmeasured", cat(path, &msg)));
}

fn diff_outcomes(
    base: Option<&BTreeMap<String, Outcome>>,
    patched: Option<&BTreeMap<String, Outcome>>,
    warnings: &mut Vec<ReportWarning>,
) -> Vec<OutcomePair> {
    let (Some(base), Some(patched)) = (base, patched) else {
        if base.is_some() || patched.is_some() {
            let msg = "test outcomes for one side only; tests[] is empty";
            warnings.push(warning("tests_one_side", msg.to_string().into()));
        }
        return Vec::new();
    };
    let ids: BTreeSet<&String> = base.keys().chain(patched.keys()).collect();
    ids.into_iter()
        .map(|t| OutcomePair {
            id: t.clone(),
            base: base.get(t).copied().unwrap_or(Outcome::NotRun),
            patched: patched.get(t).copied().unwrap_or(Outcome::NotRun),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_compares_digit_runs_as_numbers() {
        assert_eq!(natural_cmp("line 9", "line 10"), Ordering::Less);
        assert_eq!(natural_cmp("007", "7"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "a1"), Ordering::Less);
        assert_eq!(natural_cmp("\u{0663}", "2"), Ordering::Greater); // Arabic-Indic 3
    }

    #[test]
    fn remap_maps_every_line_ref() {
        let shift = |l: LineNo| (l != 5).then_some(l + 1);
        assert_eq!(
            remap_branch_id("jump to line 7", &shift).as_deref(),
            Some("jump to line 8")
        );
        assert_eq!(remap_branch_id("line 5", &shift), None);
        assert_eq!(remap_branch_id("0", &shift).as_deref(), Some("0"));
        assert_eq!(
            remap_branch_id("baseline 7", &shift).as_deref(),
            Some("baseline 7")
        );
    }
}
