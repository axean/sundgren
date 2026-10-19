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
pub(crate) mod tests {
    use super::*;
    use crate::model::FunctionRecord;

    pub(crate) const P: &str = "pkg/a.py";
    /// `None` for lines and counts in compact expectations.
    const N: i64 = -1;

    pub(crate) type Br<'a> = &'a [(LineNo, &'a str, &'a str, Option<Count>)];

    pub(crate) fn fc(path: &str, lines: &[(LineNo, Count)], branches: Br) -> FileCoverage {
        FileCoverage {
            path: path.into(),
            lines: lines.iter().copied().collect(),
            branches: branches
                .iter()
                .map(|&(line, block, branch, t)| {
                    let key = BranchKey {
                        line,
                        block: block.into(),
                        branch: branch.into(),
                    };
                    (key, t)
                })
                .collect(),
            ..Default::default()
        }
    }

    pub(crate) fn lines(l: &[(LineNo, Count)]) -> FileCoverage {
        fc(P, l, &[])
    }

    pub(crate) fn snap(files: &[FileCoverage]) -> Snapshot {
        Snapshot {
            files: files.iter().map(|f| (f.path.clone(), f.clone())).collect(),
            test_names: vec![],
        }
    }

    fn lmap(
        old: Option<&str>,
        new: Option<&str>,
        deleted: &[LineNo],
        added: &[LineNo],
    ) -> LineMaps {
        let lm = LineMap {
            old_path: old.map(Into::into),
            new_path: new.map(Into::into),
            old_to_new: BTreeMap::new(),
            deleted: deleted.to_vec(),
            added: added.to_vec(),
        };
        [&lm.old_path, &lm.new_path]
            .into_iter()
            .flatten()
            .map(|p| (p.clone(), lm.clone()))
            .collect()
    }

    fn edit(deleted: &[LineNo], added: &[LineNo]) -> LineMaps {
        lmap(Some(P), Some(P), deleted, added)
    }

    pub(crate) fn d(base: &[FileCoverage], patched: &[FileCoverage], lms: &LineMaps) -> Delta {
        diff(&[snap(base)], &[snap(patched)], lms, DiffOptions::default()).unwrap()
    }

    fn one<'a>(delta: &'a Delta, path: &str) -> &'a FileDelta {
        let mut it = delta.files.iter().filter(|f| f.path == path);
        let f = it.next().expect("file in delta");
        assert!(it.next().is_none());
        f
    }

    fn o(x: Option<i64>) -> i64 {
        x.unwrap_or(N)
    }

    fn c(x: Option<Count>) -> i64 {
        x.map_or(N, |x| x as i64)
    }

    type Cat = (i64, i64, &'static str, (i64, i64));

    /// (line, base_line, category, hits) in report order; N stands for None.
    fn cats(f: &FileDelta) -> Vec<Cat> {
        let pair = |p: Pair| (c(p.0), c(p.1));
        f.lines
            .iter()
            .map(|e| (o(e.line), o(e.base_line), e.category.as_str(), pair(e.hits)))
            .collect()
    }

    type BCat = (i64, i64, String, &'static str, (i64, i64));

    fn bcats(f: &FileDelta) -> Vec<BCat> {
        f.branches
            .iter()
            .map(|b| {
                let taken = (c(b.taken.0), c(b.taken.1));
                (
                    o(b.line),
                    o(b.base_line),
                    b.branch.clone(),
                    b.category.as_str(),
                    taken,
                )
            })
            .collect()
    }

    fn bc(line: i64, base: i64, br: &str, cat: &'static str, t: (i64, i64)) -> BCat {
        (line, base, br.to_string(), cat, t)
    }

    fn codes(delta: &Delta) -> Vec<&str> {
        delta.warnings.iter().map(|w| w.code.as_str()).collect()
    }

    fn tree(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in files {
            let path = dir.path().join(f);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x\n").unwrap();
        }
        dir
    }

    fn with_roots(
        base: &[Snapshot],
        patched: &[Snapshot],
        lms: &LineMaps,
        base_root: Option<&Path>,
        patched_root: Option<&Path>,
    ) -> Delta {
        let opts = DiffOptions {
            base_root,
            patched_root,
            ..Default::default()
        };
        diff(base, patched, lms, opts).unwrap()
    }

    // --- line classification ---

    #[test]
    fn simple_categories() {
        let cases: [(Count, Count, &str); 4] = [
            (2, 5, "kept_hit"),
            (0, 0, "kept_miss"),
            (3, 0, "lost"),
            (0, 2, "gained"),
        ];
        for (b, p, cat) in cases {
            let delta = d(&[lines(&[(1, b)])], &[lines(&[(1, p)])], &LineMaps::new());
            let f = one(&delta, P);
            assert_eq!(cats(f), [(1, 1, cat, (b as i64, p as i64))]);
            assert_eq!(f.changed(), !cat.starts_with("kept"));
        }
        let delta = d(&[lines(&[(1, 3)])], &[lines(&[(1, 0)])], &LineMaps::new());
        assert_eq!(one(&delta, P).stats.lost, 1);
    }

    #[test]
    fn absent_on_one_side_is_unmeasured_with_warning() {
        let delta = d(
            &[lines(&[(1, 1), (2, 1)])],
            &[lines(&[(1, 1), (3, 0)])],
            &LineMaps::new(),
        );
        assert_eq!(
            cats(one(&delta, P)),
            [
                (1, 1, "kept_hit", (1, 1)),
                (2, 2, "unmeasured", (1, N)),
                (3, 3, "unmeasured", (N, 0))
            ]
        );
        assert_eq!(codes(&delta), ["unmeasured"]);
        assert!(delta.warnings[0].message.starts_with("pkg/a.py: 2 line(s)"));
    }

    #[test]
    fn added_lines_are_new_hit_and_new_miss() {
        let delta = d(
            &[lines(&[(1, 1), (2, 1)])],
            &[lines(&[(1, 1), (2, 1), (3, 0), (4, 1)])],
            &edit(&[], &[2, 3]),
        );
        let f = one(&delta, P);
        assert_eq!(
            cats(f),
            [
                (1, 1, "kept_hit", (1, 1)),
                (2, N, "new_hit", (N, 1)),
                (3, N, "new_miss", (N, 0)),
                (4, 2, "kept_hit", (1, 1))
            ]
        );
        assert_eq!((f.stats.new_hit, f.stats.new_miss), (1, 1));
    }

    #[test]
    fn deleted_lines_sort_last() {
        let delta = d(
            &[lines(&[(1, 1), (2, 1), (3, 0), (4, 0)])],
            &[lines(&[(1, 1), (2, 0)])],
            &edit(&[2, 3], &[]),
        );
        assert_eq!(
            cats(one(&delta, P)),
            [
                (1, 1, "kept_hit", (1, 1)),
                (2, 4, "kept_miss", (0, 0)),
                (N, 2, "deleted_hit", (1, N)),
                (N, 3, "deleted_miss", (0, N))
            ]
        );
    }

    #[test]
    fn changed_line_is_deleted_plus_added() {
        let delta = d(
            &[lines(&[(1, 1), (2, 1)])],
            &[lines(&[(1, 1), (2, 0)])],
            &edit(&[2], &[2]),
        );
        assert_eq!(
            cats(one(&delta, P)),
            [
                (1, 1, "kept_hit", (1, 1)),
                (2, N, "new_miss", (N, 0)),
                (N, 2, "deleted_hit", (1, N))
            ]
        );
    }

    #[test]
    fn shifted_lines_are_not_lost_or_gained() {
        let delta = d(
            &[lines(&[(1, 1), (2, 0)])],
            &[lines(&[(3, 1), (4, 0)])],
            &edit(&[], &[1, 2]),
        );
        assert_eq!(
            cats(one(&delta, P)),
            [(3, 1, "kept_hit", (1, 1)), (4, 2, "kept_miss", (0, 0))]
        );
    }

    // --- branches ---

    #[test]
    fn branch_lost_gained_kept() {
        let base = fc(
            P,
            &[(1, 1)],
            &[
                (1, "0", "0", Some(2)),
                (1, "0", "1", Some(0)),
                (1, "0", "2", Some(1)),
            ],
        );
        let patched = fc(
            P,
            &[(1, 1)],
            &[
                (1, "0", "0", Some(0)),
                (1, "0", "1", Some(4)),
                (1, "0", "2", Some(1)),
            ],
        );
        let delta = d(&[base], &[patched], &LineMaps::new());
        assert_eq!(
            bcats(one(&delta, P)),
            [
                bc(1, 1, "0", "branch_lost", (2, 0)),
                bc(1, 1, "1", "branch_gained", (0, 4)),
                bc(1, 1, "2", "branch_kept_hit", (1, 1))
            ]
        );
    }

    #[test]
    fn branch_dash_is_not_taken() {
        let base = fc(
            P,
            &[(1, 1), (2, 1)],
            &[(1, "0", "0", Some(1)), (2, "0", "0", None)],
        );
        let patched = fc(
            P,
            &[(1, 1), (2, 1)],
            &[(1, "0", "0", None), (2, "0", "0", None)],
        );
        let delta = d(&[base], &[patched], &LineMaps::new());
        assert_eq!(
            bcats(one(&delta, P)),
            [
                bc(1, 1, "0", "branch_lost", (1, N)),
                bc(2, 2, "0", "branch_kept_miss", (N, N))
            ]
        );
    }

    #[test]
    fn branch_on_added_and_deleted_lines() {
        let base = fc(P, &[(1, 1), (2, 1)], &[(2, "0", "0", Some(1))]);
        let patched = fc(P, &[(1, 1), (2, 1)], &[(2, "0", "0", Some(0))]);
        let delta = d(&[base], &[patched], &edit(&[2], &[2]));
        assert_eq!(
            bcats(one(&delta, P)),
            [
                bc(2, N, "0", "branch_new", (N, 0)),
                bc(N, 2, "0", "branch_deleted", (1, N))
            ]
        );
    }

    #[test]
    fn coverage_py_branch_ids_follow_shifted_targets() {
        let base = fc(
            P,
            &[(6, 1), (7, 1), (8, 1)],
            &[
                (6, "0", "jump to line 7", Some(1)),
                (6, "0", "jump to line 8", Some(1)),
            ],
        );
        let patched = fc(
            P,
            &[(8, 1), (9, 1), (10, 1)],
            &[
                (8, "0", "jump to line 9", Some(1)),
                (8, "0", "jump to line 10", Some(1)),
            ],
        );
        let delta = d(&[base], &[patched], &edit(&[], &[1, 2]));
        let f = one(&delta, P);
        // Natural order: line 9 before line 10.
        assert_eq!(
            bcats(f),
            [
                bc(8, 6, "jump to line 9", "branch_kept_hit", (1, 1)),
                bc(8, 6, "jump to line 10", "branch_kept_hit", (1, 1))
            ]
        );
        assert!(!f.changed());
    }

    #[test]
    fn branch_to_changed_target_is_deleted_plus_new() {
        let ids = [
            "jump to line 7",
            "jump to line 8",
            "return from function 'f'",
        ];
        let br: Vec<_> = ids.iter().map(|i| (6, "0", *i, Some(1))).collect();
        let base = fc(P, &[(6, 1), (7, 1), (8, 1)], &br);
        let patched = fc(P, &[(6, 1), (7, 0), (8, 1)], &br);
        let delta = d(&[base], &[patched], &edit(&[7], &[7]));
        assert_eq!(
            bcats(one(&delta, P)),
            [
                bc(6, 6, "jump to line 7", "branch_deleted", (1, N)),
                bc(6, 6, "jump to line 7", "branch_new", (N, 1)),
                bc(6, 6, "jump to line 8", "branch_kept_hit", (1, 1)),
                bc(6, 6, "return from function 'f'", "branch_kept_hit", (1, 1))
            ]
        );
    }

    #[test]
    fn branch_without_counterpart_line_data_is_unmeasured() {
        let base = fc(P, &[(1, 1)], &[(1, "0", "0", Some(1))]);
        let delta = d(&[base], &[lines(&[(1, 1)])], &LineMaps::new());
        assert_eq!(
            bcats(one(&delta, P)),
            [bc(1, 1, "0", "branch_unmeasured", (1, N))]
        );
        assert_eq!(codes(&delta), ["unmeasured"]);
    }

    // --- files measured on one side only ---

    #[test]
    fn inferred_base_when_file_exists_in_base_tree() {
        let root = tree(&[P]);
        let patched = fc(P, &[(1, 1), (2, 0)], &[(1, "0", "0", Some(1))]);
        let delta = with_roots(
            &[snap(&[])],
            &[snap(&[patched])],
            &LineMaps::new(),
            Some(root.path()),
            None,
        );
        let f = one(&delta, P);
        assert_eq!(
            (f.inferred_side, f.base_path.as_deref()),
            (Some(Side::Base), Some(P))
        );
        assert_eq!(
            cats(f),
            [(1, 1, "gained", (0, 1)), (2, 2, "kept_miss", (0, 0))]
        );
        assert_eq!(bcats(f), [bc(1, 1, "0", "branch_gained", (0, 1))]);
        assert_eq!(codes(&delta), ["inferred_file"]);
        assert_eq!((f.stats.executable_base, f.stats.hit_base), (2, 0));
    }

    #[test]
    fn inferred_patched_maps_lines_through_the_patch() {
        let root = tree(&[P]);
        let base = fc(P, &[(1, 1), (2, 0)], &[]);
        let delta = with_roots(
            &[snap(&[base])],
            &[snap(&[])],
            &LineMaps::new(),
            None,
            Some(root.path()),
        );
        let f = one(&delta, P);
        assert_eq!(f.inferred_side, Some(Side::Patched));
        assert_eq!(
            cats(f),
            [(1, 1, "lost", (1, 0)), (2, 2, "kept_miss", (0, 0))]
        );

        let base = fc(P, &[(1, 1), (2, 1)], &[(1, "0", "jump to line 2", Some(1))]);
        let delta = with_roots(
            &[snap(&[base])],
            &[snap(&[])],
            &edit(&[], &[1]),
            None,
            Some(root.path()),
        );
        let f = one(&delta, P);
        assert_eq!(cats(f), [(2, 1, "lost", (1, 0)), (3, 2, "lost", (1, 0))]);
        assert_eq!(
            bcats(f),
            [bc(2, 1, "jump to line 3", "branch_lost", (1, 0))]
        );
    }

    #[test]
    fn nothing_executable_means_nothing_inferred() {
        let init = "pkg/__init__.py";
        let root = tree(&[init]);
        let delta = with_roots(
            &[snap(&[])],
            &[snap(&[fc(init, &[], &[])])],
            &LineMaps::new(),
            Some(root.path()),
            None,
        );
        let f = one(&delta, init);
        assert_eq!(
            (f.inferred_side, f.base_path.as_deref()),
            (None, Some(init))
        );
        assert!(f.lines.is_empty() && delta.warnings.is_empty());
    }

    #[test]
    fn inferred_side_has_no_counterpart_for_branches_to_new_lines() {
        let root = tree(&[P]);
        let br = [
            (1, "0", "jump to line 2", Some(1)),
            (1, "0", "jump to line 3", Some(0)),
        ];
        let patched = fc(P, &[(1, 1), (2, 1), (3, 0)], &br);
        let delta = with_roots(
            &[snap(&[])],
            &[snap(&[patched])],
            &edit(&[], &[2, 3]),
            Some(root.path()),
            None,
        );
        let got: Vec<_> = bcats(one(&delta, P)).into_iter().map(|b| b.3).collect();
        assert_eq!(got, ["branch_new", "branch_new"]);
        assert_eq!(codes(&delta), ["inferred_file"]);
    }

    #[test]
    fn missing_from_tree_means_new_or_deleted_file() {
        let empty = tree(&[]);
        let delta = with_roots(
            &[snap(&[])],
            &[snap(&[lines(&[(1, 1), (2, 0)])])],
            &LineMaps::new(),
            Some(empty.path()),
            None,
        );
        let f = one(&delta, P);
        assert_eq!((f.base_path.as_deref(), f.inferred_side), (None, None));
        assert_eq!(
            cats(f),
            [(1, N, "new_hit", (N, 1)), (2, N, "new_miss", (N, 0))]
        );
        assert!(delta.warnings.is_empty());

        let delta = with_roots(
            &[snap(&[lines(&[(1, 1)])])],
            &[snap(&[])],
            &LineMaps::new(),
            None,
            Some(empty.path()),
        );
        let f = one(&delta, P);
        assert_eq!((f.path.as_str(), f.base_path.as_deref()), (P, Some(P)));
        assert_eq!(cats(f), [(N, 1, "deleted_hit", (1, N))]);
        assert!(f.source_changed);
    }

    #[test]
    fn root_unknown_uses_patch_for_new_and_deleted_files() {
        let delta = d(&[], &[lines(&[(1, 0)])], &lmap(None, Some(P), &[], &[]));
        let f = one(&delta, P);
        assert_eq!(f.base_path, None);
        assert_eq!(cats(f), [(1, N, "new_miss", (N, 0))]);
        assert!(delta.warnings.is_empty());

        let delta = d(&[lines(&[(1, 1)])], &[], &lmap(Some(P), None, &[], &[]));
        assert_eq!(cats(one(&delta, P)), [(N, 1, "deleted_hit", (1, N))]);
        assert!(delta.warnings.is_empty());
    }

    #[test]
    fn root_unknown_without_patch_info_is_unmeasured() {
        let delta = d(&[], &[lines(&[(1, 1), (2, 0)])], &LineMaps::new());
        let f = one(&delta, P);
        assert_eq!(f.inferred_side, None);
        assert!(
            f.lines
                .iter()
                .all(|e| e.category == LineCategory::Unmeasured)
        );
        assert_eq!(codes(&delta), ["unmeasured"]);
        let msg = &delta.warnings[0].message;
        assert!(
            msg.contains("not in base coverage and the base tree is unknown"),
            "{msg}"
        );

        let delta = d(&[fc("pkg/__init__.py", &[], &[])], &[], &LineMaps::new());
        assert!(one(&delta, "pkg/__init__.py").lines.is_empty());
        assert!(delta.warnings.is_empty());
    }

    // --- flakiness ---

    #[test]
    fn flaky_base_line_is_excluded_from_lost() {
        let base = [
            snap(&[lines(&[(1, 1), (2, 1)])]),
            snap(&[lines(&[(1, 1), (2, 0)])]),
        ];
        let patched = [snap(&[lines(&[(1, 0), (2, 0)])])];
        let delta = diff(&base, &patched, &LineMaps::new(), DiffOptions::default()).unwrap();
        let f = one(&delta, P);
        assert_eq!(cats(f), [(1, 1, "lost", (1, 0))]);
        let fl = &f.flaky[0];
        assert_eq!(
            (fl.line, fl.base_line, fl.hits),
            (Some(2), Some(2), (Some(1), Some(0)))
        );
        assert_eq!(fl.sides, [Side::Base]);
        assert_eq!((f.stats.lost, f.stats.hit_base), (1, 2));
    }

    #[test]
    fn flaky_patched_line_and_missing_file_in_a_repeat() {
        let base = [snap(&[lines(&[(1, 0)])])];
        let patched = [snap(&[lines(&[(1, 1)])]), snap(&[])];
        let delta = diff(&base, &patched, &LineMaps::new(), DiffOptions::default()).unwrap();
        let f = one(&delta, P);
        assert!(f.lines.is_empty() && f.changed());
        assert_eq!(
            (f.flaky[0].line, &f.flaky[0].sides[..]),
            (Some(1), &[Side::Patched][..])
        );
    }

    #[test]
    fn flaky_branch() {
        let br = |a, b| {
            fc(
                P,
                &[(1, 1)],
                &[(1, "0", "0", Some(a)), (1, "0", "1", Some(b))],
            )
        };
        let base = [snap(&[br(1, 1)]), snap(&[br(0, 1)])];
        let delta = diff(
            &base,
            &[snap(&[br(0, 0)])],
            &LineMaps::new(),
            DiffOptions::default(),
        )
        .unwrap();
        let got: Vec<_> = one(&delta, P)
            .branches
            .iter()
            .map(|b| (b.branch.as_str(), b.category.as_str()))
            .collect();
        assert_eq!(got, [("0", "branch_flaky"), ("1", "branch_lost")]);
    }

    // --- pairing, renames, external files ---

    #[test]
    fn rename_pairs_old_and_new_path() {
        let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[], &[]);
        let delta = d(
            &[fc("pkg/old.py", &[(1, 1), (2, 1)], &[])],
            &[fc("pkg/new.py", &[(1, 1), (2, 0)], &[])],
            &lms,
        );
        let [f] = &delta.files[..] else { panic!() };
        assert_eq!(
            (f.path.as_str(), f.base_path.as_deref()),
            ("pkg/new.py", Some("pkg/old.py"))
        );
        assert_eq!(
            cats(f),
            [(1, 1, "kept_hit", (1, 1)), (2, 2, "lost", (1, 0))]
        );
        assert!(!f.source_changed);
    }

    #[test]
    fn rename_with_edits() {
        let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[2], &[2, 3]);
        let delta = d(
            &[fc("pkg/old.py", &[(1, 1), (2, 1), (3, 0)], &[])],
            &[fc("pkg/new.py", &[(1, 1), (2, 0), (3, 1), (4, 1)], &[])],
            &lms,
        );
        let [f] = &delta.files[..] else { panic!() };
        assert_eq!(
            cats(f),
            [
                (1, 1, "kept_hit", (1, 1)),
                (2, N, "new_miss", (N, 0)),
                (3, N, "new_hit", (N, 1)),
                (4, 3, "gained", (0, 1)),
                (N, 2, "deleted_hit", (1, N))
            ]
        );
        assert!(f.source_changed);
    }

    #[test]
    fn rename_with_only_base_measured() {
        let root = tree(&["pkg/new.py"]);
        let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[], &[]);
        let base = [snap(&[fc("pkg/old.py", &[(1, 1)], &[])])];
        let delta = with_roots(&base, &[snap(&[])], &lms, None, Some(root.path()));
        let [f] = &delta.files[..] else { panic!() };
        let got = (f.path.as_str(), f.base_path.as_deref(), f.inferred_side);
        assert_eq!(got, ("pkg/new.py", Some("pkg/old.py"), Some(Side::Patched)));
        assert_eq!(cats(f), [(1, 1, "lost", (1, 0))]);
    }

    #[test]
    fn patch_says_deleted_but_snapshot_has_it() {
        let delta = d(
            &[lines(&[(1, 1)])],
            &[lines(&[(1, 0)])],
            &lmap(Some(P), None, &[], &[]),
        );
        assert_eq!(cats(one(&delta, P)), [(1, 1, "lost", (1, 0))]);
    }

    #[test]
    fn colliding_base_file_is_ignored_with_warning() {
        let lms = lmap(Some("pkg/old.py"), Some("pkg/new.py"), &[], &[]);
        let delta = d(
            &[
                fc("pkg/old.py", &[(1, 1)], &[]),
                fc("pkg/new.py", &[(1, 1)], &[]),
            ],
            &[fc("pkg/new.py", &[(1, 1)], &[])],
            &lms,
        );
        let got: Vec<_> = delta
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.base_path.as_deref()))
            .collect();
        assert_eq!(got, [("pkg/new.py", Some("pkg/old.py"))]);
        assert_eq!(codes(&delta), ["unpaired_file"]);
    }

    #[test]
    fn external_files_are_counted_not_listed() {
        let mut ext = fc("/usr/lib/python3/os.py", &[(1, 1)], &[]);
        ext.external = true;
        let files = [ext, lines(&[(1, 1)])];
        let delta = d(&files, &files, &LineMaps::new());
        assert_eq!(delta.external_files, 1);
        assert_eq!(
            delta
                .files
                .iter()
                .map(|f| f.path.as_str())
                .collect::<Vec<_>>(),
            [P]
        );
        let children = delta.tree.children.as_ref().unwrap();
        assert_eq!(
            children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["pkg"]
        );
    }

    #[test]
    fn files_sorted_and_tree_covers_unchanged_files() {
        let files = ["b.py", "a/z.py", "a/c.py"].map(|p| fc(p, &[(1, 1)], &[]));
        let delta = d(&files, &files, &LineMaps::new());
        let paths: Vec<_> = delta.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["a/c.py", "a/z.py", "b.py"]);
        let children = delta.tree.children.as_ref().unwrap();
        assert_eq!(
            children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["a", "b.py"]
        );
        assert_eq!(delta.tree.stats.executable_base, 3);
    }

    // --- sha256 and source_changed ---

    #[test]
    fn sha256_from_roots() {
        let (b, p) = (tree(&[P]), tree(&[P]));
        let run = || {
            let s = [snap(&[lines(&[(1, 1)])])];
            with_roots(&s, &s, &LineMaps::new(), Some(b.path()), Some(p.path()))
        };
        let delta = run();
        let f = one(&delta, P);
        let digest = crate::sha256_file(&b.path().join(P)).unwrap();
        assert_eq!(f.sha256, (Some(digest.clone()), Some(digest)));
        assert!(!f.source_changed);
        std::fs::write(p.path().join(P), "changed\n").unwrap();
        assert!(one(&run(), P).source_changed);
    }

    #[test]
    fn source_changed_from_linemap_when_roots_unknown() {
        let delta = d(&[lines(&[(1, 1)])], &[lines(&[(2, 1)])], &edit(&[], &[1]));
        let f = one(&delta, P);
        assert_eq!(f.sha256, (None, None));
        assert!(f.source_changed);
    }

    // --- test outcomes ---

    #[test]
    fn outcomes() {
        let o = |items: &[(&str, Outcome)]| -> Outcomes {
            items.iter().map(|(k, v)| (k.to_string(), *v)).collect()
        };
        let base = o(&[("t::a", Outcome::Passed), ("t::b", Outcome::Passed)]);
        let patched = o(&[("t::a", Outcome::Skipped), ("t::c", Outcome::Failed)]);
        let opts = DiffOptions {
            base_outcomes: Some(&base),
            patched_outcomes: Some(&patched),
            ..Default::default()
        };
        let delta = diff(&[snap(&[])], &[snap(&[])], &LineMaps::new(), opts).unwrap();
        let got: Vec<_> = delta
            .tests
            .iter()
            .map(|t| (t.id.as_str(), t.base.as_str(), t.patched.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("t::a", "passed", "skipped"),
                ("t::b", "passed", "not_run"),
                ("t::c", "not_run", "failed")
            ]
        );

        assert!(d(&[], &[], &LineMaps::new()).tests.is_empty());
        let opts = DiffOptions {
            base_outcomes: Some(&base),
            ..Default::default()
        };
        let delta = diff(&[snap(&[])], &[snap(&[])], &LineMaps::new(), opts).unwrap();
        assert!(delta.tests.is_empty());
        assert_eq!(codes(&delta), ["tests_one_side"]);
    }

    // --- helpers ---

    #[test]
    fn natural_order_and_remap() {
        assert_eq!(natural_cmp("line 9", "line 10"), Ordering::Less);
        assert_eq!(natural_cmp("007", "7"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "a1"), Ordering::Less);
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

    // --- real coverage.py output ---

    #[test]
    fn real_skip_divide() {
        let fix = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let read = |p: &str| std::fs::read_to_string(fix.join(p)).unwrap();
        let run = "shed_py_runs/skip_divide";
        let patch = crate::patch::parse_patch(&read("patches/skip_divide.patch")).unwrap();
        let base_out = crate::junit::parse_junit(&read(&format!("{run}/base.xml"))).unwrap();
        let patched_out = crate::junit::parse_junit(&read(&format!("{run}/patched.xml"))).unwrap();
        let shed = fix.join("shed_py");
        let opts = DiffOptions {
            base_root: Some(&shed),
            patched_root: Some(&shed),
            base_outcomes: Some(&base_out),
            patched_outcomes: Some(&patched_out),
        };
        let base = crate::lcov::parse(&read(&format!("{run}/base.lcov"))).unwrap();
        let patched = crate::lcov::parse(&read(&format!("{run}/patched.lcov"))).unwrap();
        let delta = diff(
            &[base],
            &[patched],
            &crate::linemap::build_linemaps(&patch),
            opts,
        )
        .unwrap();
        let changed_tests: Vec<_> = delta
            .tests
            .iter()
            .filter(|t| t.base != t.patched)
            .map(|t| (t.id.as_str(), t.base.as_str(), t.patched.as_str()))
            .collect();
        assert_eq!(
            changed_tests,
            [("tests.test_calc::test_divide", "passed", "skipped")]
        );
        let calc = "shed/calc.py";
        let changed: Vec<_> = delta
            .files
            .iter()
            .filter(|f| f.changed())
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!(changed, [calc]);
        let f = one(&delta, calc);
        assert!(!f.source_changed);
        assert_eq!(
            f.sha256.0,
            Some(crate::sha256_file(&shed.join(calc)).unwrap())
        );
        let lost: Vec<_> = cats(f)
            .into_iter()
            .filter(|e| !e.2.starts_with("kept_"))
            .collect();
        assert_eq!(lost, [6, 7, 8].map(|l| (l, l, "lost", (1, 0))));
        let br: Vec<_> = bcats(f)
            .into_iter()
            .filter(|b| b.3 != "branch_kept_hit")
            .collect();
        assert_eq!(
            br,
            [
                bc(6, 6, "jump to line 7", "branch_lost", (1, N)),
                bc(6, 6, "jump to line 8", "branch_lost", (1, N))
            ]
        );
        let funcs: Vec<_> = f
            .functions
            .iter()
            .map(|g| (g.name.as_str(), g.status.as_str(), g.start, g.end, g.lost))
            .collect();
        assert_eq!(
            funcs,
            [
                ("add", "unchanged", 2, 3, 0),
                ("divide", "went_dark", 5, 8, 3),
                ("classify", "unchanged", 10, 15, 0)
            ]
        );
        assert!(delta.warnings.is_empty());
    }

    #[test]
    fn functions_from_lcov_fn_records() {
        let fns = vec![
            FunctionRecord {
                name: "f".into(),
                start: 1,
                end: Some(4),
                hits: Some(1),
            },
            FunctionRecord {
                name: "g".into(),
                start: 6,
                end: None,
                hits: Some(1),
            },
        ];
        let mut base = lines(&[(1, 1), (2, 1), (3, 1), (4, 1), (6, 1), (7, 1)]);
        let mut patched = lines(&[(1, 1), (2, 1), (3, 0), (4, 1), (6, 1), (7, 0)]);
        base.functions = fns.clone();
        patched.functions = fns;
        let delta = d(&[base], &[patched], &LineMaps::new());
        let got: Vec<_> = delta.files[0]
            .functions
            .iter()
            .map(|g| (g.name.as_str(), g.status.as_str(), g.start, g.end, g.lost))
            .collect();
        // The FN line is the signature: g's body is line 7 only, so g went dark.
        assert_eq!(
            got,
            [("f", "partial_loss", 1, 4, 1), ("g", "went_dark", 6, 7, 1)]
        );
    }
}
