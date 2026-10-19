//! (Snapshots, Snapshots, LineMaps, outcomes) -> Delta.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::linemap::{LineMap, LineMaps, linemap_for};
use crate::model::{
    BranchKey, Count, FileCoverage, LineNo, Outcome, Outcomes, Snapshot, Warning, warning,
};
use crate::rollup::{
    FunctionDelta, FunctionStatus, LineSets, Lines, Stats, TreeNode, build_tree, function_deltas,
    lcov_spans,
};
use crate::str_enum;

str_enum!(Side {
    Base = "base",
    Patched = "patched"
});

str_enum!(LineCategory {
    KeptHit = "kept_hit",
    KeptMiss = "kept_miss",
    Lost = "lost",
    Gained = "gained",
    Unmeasured = "unmeasured",
    NewHit = "new_hit",
    NewMiss = "new_miss",
    DeletedHit = "deleted_hit",
    DeletedMiss = "deleted_miss",
});

str_enum!(BranchCategory {
    KeptHit = "branch_kept_hit",
    KeptMiss = "branch_kept_miss",
    Lost = "branch_lost",
    Gained = "branch_gained",
    New = "branch_new",
    Deleted = "branch_deleted",
    Flaky = "branch_flaky",
    Unmeasured = "branch_unmeasured",
});

impl LineCategory {
    pub fn is_kept(self) -> bool {
        matches!(self, Self::KeptHit | Self::KeptMiss)
    }
}

impl BranchCategory {
    pub fn is_kept(self) -> bool {
        matches!(self, Self::KeptHit | Self::KeptMiss)
    }
}

/// (base, patched); None = absent side (or '-' for branches).
pub type Pair = (Option<Count>, Option<Count>);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LineDelta {
    /// Patched line; None if deleted.
    pub line: Option<LineNo>,
    /// None if added.
    pub base_line: Option<LineNo>,
    pub category: LineCategory,
    pub hits: Pair,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct BranchDelta {
    pub line: Option<LineNo>,
    pub base_line: Option<LineNo>,
    /// Patched-side ids, base side if there is no patched branch.
    pub block: String,
    pub branch: String,
    pub category: BranchCategory,
    pub taken: Pair,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct FlakyLine {
    pub line: Option<LineNo>,
    pub base_line: Option<LineNo>,
    /// From run 0.
    pub hits: Pair,
    /// Side(s) whose repeats disagreed.
    pub sides: Vec<Side>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDelta {
    /// Patched path, base path if deleted.
    pub path: String,
    /// None for a new file.
    pub base_path: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct OutcomePair {
    pub id: String,
    pub base: Outcome,
    pub patched: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    /// Every non-external file on either side, sorted by path.
    pub files: Vec<FileDelta>,
    /// Every test, sorted by id.
    pub tests: Vec<OutcomePair>,
    pub tree: TreeNode,
    pub external_files: i64,
    /// Deduplicated and sorted.
    pub warnings: Vec<Warning>,
}

/// Inputs besides the snapshots and line maps; the defaults mean "unknown".
#[derive(Default, Clone, Copy)]
pub struct DiffOptions<'a> {
    pub base_root: Option<&'a Path>,
    pub patched_root: Option<&'a Path>,
    pub base_outcomes: Option<&'a Outcomes>,
    pub patched_outcomes: Option<&'a Outcomes>,
}

/// Classify every line, branch and function; [0] of each side is canonical, the rest repeats.
pub fn diff(
    base: &[Snapshot],
    patched: &[Snapshot],
    linemaps: &LineMaps,
    opts: DiffOptions<'_>,
) -> crate::Result<Delta> {
    assert!(
        !base.is_empty() && !patched.is_empty(),
        "a side has no snapshot"
    );
    let mut warnings = Vec::new();
    let mut files = Vec::new();
    for pair in pair_files(&base[0], &patched[0], linemaps, &mut warnings) {
        files.push(diff_file(&pair, base, patched, &opts, &mut warnings)?);
    }
    files.sort_by(|a, b| (&a.path, &a.base_path).cmp(&(&b.path, &b.base_path)));
    let external: HashSet<&str> = [&base[0], &patched[0]]
        .iter()
        .flat_map(|s| s.files.iter())
        .filter(|(_, fc)| fc.external)
        .map(|(p, _)| p.as_str())
        .collect();
    let tests = diff_outcomes(opts.base_outcomes, opts.patched_outcomes, &mut warnings);
    let tree = build_tree(files.iter().map(|f| (f.path.as_str(), f.stats)));
    let warnings: BTreeSet<Warning> = warnings.into_iter().collect();
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
    base_path: Option<String>,
    patched_path: Option<String>,
    lm: Cow<'a, LineMap>,
}

fn internal(snapshot: &Snapshot) -> impl Iterator<Item = &String> {
    snapshot
        .files
        .iter()
        .filter(|(_, fc)| !fc.external)
        .map(|(p, _)| p)
}

/// Patched paths first, via the line map's old path; then unclaimed base paths.
fn pair_files<'a>(
    base: &Snapshot,
    patched: &Snapshot,
    linemaps: &'a LineMaps,
    warnings: &mut Vec<Warning>,
) -> Vec<FilePair<'a>> {
    let mut pairs: Vec<FilePair<'a>> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut claimed: HashSet<String> = HashSet::new();
    for p in internal(patched) {
        let mut lm = linemap_for(linemaps, p);
        if lm.new_path.as_ref() != Some(p) {
            // The patch says p is gone; the snapshot wins.
            lm = Cow::Owned(LineMap::identity(p));
        }
        if let Some(old) = &lm.old_path {
            claimed.insert(old.clone());
        }
        let pair = FilePair {
            base_path: lm.old_path.clone(),
            patched_path: Some(p.clone()),
            lm,
        };
        match index.get(p) {
            Some(&i) => pairs[i] = pair,
            None => {
                index.insert(p.clone(), pairs.len());
                pairs.push(pair);
            }
        }
    }
    for b in internal(base) {
        if claimed.contains(b) {
            continue;
        }
        let mut lm = linemap_for(linemaps, b);
        if lm.old_path.as_ref() != Some(b) {
            lm = Cow::Owned(LineMap::identity(b));
        }
        let key = lm.new_path.clone().unwrap_or_else(|| b.clone());
        if index.contains_key(&key) {
            let msg = format!("{b}: base file collides with patched {key}; ignored");
            warnings.push(warning("unpaired_file", msg));
            continue;
        }
        index.insert(key, pairs.len());
        pairs.push(FilePair {
            base_path: Some(b.clone()),
            patched_path: lm.new_path.clone(),
            lm,
        });
    }
    pairs
}

fn get<'s>(snapshot: &'s Snapshot, path: Option<&String>) -> Option<&'s FileCoverage> {
    snapshot.files.get(path?).filter(|fc| !fc.external)
}

fn state(fc: Option<&FileCoverage>, path: Option<&String>, root: Option<&Path>) -> State {
    match (fc, path, root) {
        (Some(_), _, _) => State::Measured,
        (None, None, _) => State::Nonexistent,
        (None, Some(_), None) => State::Unmeasured,
        (None, Some(p), Some(root)) if root.join(p).is_file() => State::Inferred,
        _ => State::Nonexistent,
    }
}

/// Items "on" in some repeats but not all; a run without the file has nothing on.
fn unstable<T: Ord + Clone>(
    runs: &[Snapshot],
    path: Option<&String>,
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

fn truthy(c: Option<Count>) -> bool {
    c.is_some_and(|c| c != 0)
}

type Branches = BTreeMap<BranchKey, Option<Count>>;
type Convert<'a> = &'a dyn Fn(LineNo) -> Option<LineNo>;

fn sha(root: Option<&Path>, path: &Option<String>) -> crate::Result<Option<String>> {
    match (root, path) {
        (Some(r), Some(p)) if !p.is_empty() && r.join(p).is_file() => {
            Ok(Some(crate::sha256_file(&r.join(p))?))
        }
        _ => Ok(None),
    }
}

fn diff_file(
    pair: &FilePair<'_>,
    base_runs: &[Snapshot],
    patched_runs: &[Snapshot],
    opts: &DiffOptions<'_>,
    warnings: &mut Vec<Warning>,
) -> crate::Result<FileDelta> {
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
        let msg = format!(
            "{path}: not in {s} coverage but present in the {s} tree; counted as never executed"
        );
        warnings.push(warning("inferred_file", msg));
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

    let spans = |path: &Option<String>, fc| match path {
        None => Some(Vec::new()),
        Some(_) => lcov_spans(fc),
    };
    let base_spans = spans(&base_path, fc_b);
    let patched_spans = spans(&patched_path, fc_p);
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

    let sha_b = sha(opts.base_root, &base_path)?;
    let sha_p = sha(opts.patched_root, &patched_path)?;
    let source_changed = match (&sha_b, &sha_p) {
        (Some(b), Some(p)) => b != p,
        _ => {
            base_path.is_none()
                || patched_path.is_none()
                || !lm.deleted.is_empty()
                || !lm.added.is_empty()
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
                category: line_category(b, p, hits),
                hits,
            });
        }
    }
    entries.sort_by_key(|e| order(e.line, e.base_line));
    flaky.sort_by_key(|e| order(e.line, e.base_line));
    (entries, flaky)
}

fn line_category(b: Option<LineNo>, p: Option<LineNo>, (hb, hp): Pair) -> LineCategory {
    match (b, p, hb, hp) {
        (None, ..) if truthy(hp) => LineCategory::NewHit,
        (None, ..) => LineCategory::NewMiss,
        (_, None, ..) if truthy(hb) => LineCategory::DeletedHit,
        (_, None, ..) => LineCategory::DeletedMiss,
        (.., Some(hb), Some(hp)) => match (hb > 0, hp > 0) {
            (true, true) => LineCategory::KeptHit,
            (true, false) => LineCategory::Lost,
            (false, true) => LineCategory::Gained,
            (false, false) => LineCategory::KeptMiss,
        },
        _ => LineCategory::Unmeasured,
    }
}

static LINE_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bline ([0-9]+)\b").unwrap());
static DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]+").unwrap());

/// Map "line N" refs in a branch id (coverage.py "jump to line N"); None if one doesn't map.
fn remap_branch_id(branch: &str, convert: Convert) -> Option<String> {
    let mut out = String::with_capacity(branch.len());
    let mut last = 0;
    for caps in LINE_REF.captures_iter(branch) {
        let m = caps.get(0).expect("whole match");
        let target = convert(caps[1].parse().ok()?)?;
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
    let taken = |br: &Branches, k: &BranchKey| br.get(k).copied().flatten();
    let mut matched: HashSet<BranchKey> = HashSet::new();
    let mut out = Vec::new();
    for kb in keys_b {
        let p = to_new(kb.line);
        let kp = p
            .and_then(|p| {
                Some(BranchKey {
                    line: p,
                    block: kb.block.clone(),
                    branch: remap_branch_id(&kb.branch, to_new)?,
                })
            })
            .filter(|kp| keys_p.contains(kp));
        let category = if let Some(kp) = &kp {
            matched.insert(kp.clone());
            if flaky_b.contains(kb) || flaky_p.contains(kp) {
                BranchCategory::Flaky
            } else {
                match (truthy(taken(br_b, kb)), truthy(taken(br_p, kp))) {
                    (true, true) => BranchCategory::KeptHit,
                    (true, false) => BranchCategory::Lost,
                    (false, true) => BranchCategory::Gained,
                    (false, false) => BranchCategory::KeptMiss,
                }
            }
        } else if flaky_b.contains(kb) {
            BranchCategory::Flaky
        } else if p.is_some_and(|p| !lines_with_p.contains(&p))
            && inferred_side != Some(Side::Patched)
        {
            BranchCategory::Unmeasured
        } else {
            BranchCategory::Deleted
        };
        let key = kp.as_ref().unwrap_or(kb);
        out.push(BranchDelta {
            line: p,
            base_line: Some(kb.line),
            block: key.block.clone(),
            branch: key.branch.clone(),
            category,
            taken: (taken(br_b, kb), kp.as_ref().and_then(|kp| taken(br_p, kp))),
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
            taken: (None, taken(br_p, kp)),
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NatPart<'a> {
    Text(&'a str),
    /// Digits with leading zeros stripped; compared by length, then digits.
    Num(usize, &'a str),
}

fn natural_key(s: &str) -> Vec<NatPart<'_>> {
    let mut out = Vec::new();
    let mut last = 0;
    for m in DIGITS.find_iter(s) {
        out.push(NatPart::Text(&s[last..m.start()]));
        let digits = m.as_str().trim_start_matches('0');
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
    path: &str,
    state_b: State,
    state_p: State,
    lines: &[LineDelta],
    branches: &[BranchDelta],
    warnings: &mut Vec<Warning>,
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
    let msg = format!("{path}: {n_lines} line(s), {n_branches} branch(es) unmeasured: {why}");
    warnings.push(warning("unmeasured", msg));
}

fn diff_outcomes(
    base: Option<&Outcomes>,
    patched: Option<&Outcomes>,
    warnings: &mut Vec<Warning>,
) -> Vec<OutcomePair> {
    let (Some(base), Some(patched)) = (base, patched) else {
        if base.is_some() || patched.is_some() {
            let msg = "test outcomes for one side only; tests[] is empty";
            warnings.push(warning("tests_one_side", msg));
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
pub(crate) mod tests;
