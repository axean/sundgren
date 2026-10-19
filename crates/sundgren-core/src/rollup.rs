//! Function and directory-tree rollups. Port of `sundgren/core/rollup.py`.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Add;

use crate::linemap::LineMap;
use crate::model::{Count, LineNo};
use crate::pytext::PyText;
use crate::symbols::{Span, spans_by_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionStatus {
    WentDark,
    LitUp,
    PartialLoss,
    PartialGain,
    Mixed,
    Unchanged,
    Added,
    Removed,
}

impl FunctionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FunctionStatus::WentDark => "went_dark",
            FunctionStatus::LitUp => "lit_up",
            FunctionStatus::PartialLoss => "partial_loss",
            FunctionStatus::PartialGain => "partial_gain",
            FunctionStatus::Mixed => "mixed",
            FunctionStatus::Unchanged => "unchanged",
            FunctionStatus::Added => "added",
            FunctionStatus::Removed => "removed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "went_dark" => FunctionStatus::WentDark,
            "lit_up" => FunctionStatus::LitUp,
            "partial_loss" => FunctionStatus::PartialLoss,
            "partial_gain" => FunctionStatus::PartialGain,
            "mixed" => FunctionStatus::Mixed,
            "unchanged" => FunctionStatus::Unchanged,
            "added" => FunctionStatus::Added,
            "removed" => FunctionStatus::Removed,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDelta {
    pub name: String,
    /// Patched-side span, base side for `Removed`.
    pub start: LineNo,
    pub end: LineNo,
    pub status: FunctionStatus,
    pub lost: i64,
    pub gained: i64,
    /// Executable body lines on the reported side.
    pub executable: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Stats {
    pub lost: i64,
    pub gained: i64,
    pub new_hit: i64,
    pub new_miss: i64,
    pub deleted_hit: i64,
    pub deleted_miss: i64,
    pub executable_base: i64,
    pub executable_patched: i64,
    pub hit_base: i64,
    pub hit_patched: i64,
}

impl Stats {
    pub const FIELDS: [&'static str; 10] = [
        "lost",
        "gained",
        "new_hit",
        "new_miss",
        "deleted_hit",
        "deleted_miss",
        "executable_base",
        "executable_patched",
        "hit_base",
        "hit_patched",
    ];

    /// Field values in declaration order.
    pub fn values(&self) -> [i64; 10] {
        [
            self.lost,
            self.gained,
            self.new_hit,
            self.new_miss,
            self.deleted_hit,
            self.deleted_miss,
            self.executable_base,
            self.executable_patched,
            self.hit_base,
            self.hit_patched,
        ]
    }

    pub fn from_values(v: [i64; 10]) -> Self {
        Stats {
            lost: v[0],
            gained: v[1],
            new_hit: v[2],
            new_miss: v[3],
            deleted_hit: v[4],
            deleted_miss: v[5],
            executable_base: v[6],
            executable_patched: v[7],
            hit_base: v[8],
            hit_patched: v[9],
        }
    }
}

impl Add for Stats {
    type Output = Stats;

    fn add(self, other: Stats) -> Stats {
        let (a, b) = (self.values(), other.values());
        Stats::from_values(std::array::from_fn(|i| a[i] + b[i]))
    }
}

/// `Dir` sorts before `File`, as "dir" < "file".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    Dir,
    File,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Dir => "dir",
            NodeKind::File => "file",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TreeNode {
    pub name: PyText,
    pub path: PyText,
    pub kind: NodeKind,
    pub stats: Stats,
    pub children: Vec<TreeNode>,
}

/// Run-0 hit counts per line, one side.
pub type Lines = BTreeMap<LineNo, Count>;

/// Patched lines lost/gained and each side's flaky lines.
#[derive(Debug, Clone, Default)]
pub struct LineSets {
    pub lost: BTreeSet<LineNo>,
    pub gained: BTreeSet<LineNo>,
    pub flaky_base: BTreeSet<LineNo>,
    pub flaky_patched: BTreeSet<LineNo>,
}

/// Per-function status; None spans are borrowed from the other side, empty means no file.
pub fn function_deltas(
    base_spans: Option<&[Span]>,
    patched_spans: Option<&[Span]>,
    lm: &LineMap,
    base_lines: &Lines,
    patched_lines: &Lines,
    sets: &LineSets,
) -> Vec<FunctionDelta> {
    let (base_spans, patched_spans): (Vec<Span>, Vec<Span>) = match (base_spans, patched_spans) {
        (None, None) => return Vec::new(),
        (None, Some(p)) => (map_spans(p, |l| lm.to_old(l)), p.to_vec()),
        (Some(b), None) => (b.to_vec(), map_spans(b, |l| lm.to_new(l))),
        (Some(b), Some(p)) => (b.to_vec(), p.to_vec()),
    };

    let mut out = Vec::new();
    for (sb, sp) in match_spans(&base_spans, &patched_spans, lm) {
        let Some(sp) = sp else {
            let sb = sb.expect("a pair has at least one span");
            let n = count(sb.body_lines(), |l| base_lines.contains_key(&l));
            out.push(delta(sb, FunctionStatus::Removed, 0, 0, n));
            continue;
        };
        let executable = count(sp.body_lines(), |l| patched_lines.contains_key(&l));
        let Some(sb) = sb else {
            out.push(delta(sp, FunctionStatus::Added, 0, 0, executable));
            continue;
        };
        let n_lost = count(sp.body_lines(), |l| sets.lost.contains(&l));
        let n_gained = count(sp.body_lines(), |l| sets.gained.contains(&l));
        let (b_exec, b_hit) = body(sb, base_lines, &sets.flaky_base);
        let (p_exec, p_hit) = body(sp, patched_lines, &sets.flaky_patched);
        let status = if b_hit && p_exec && !p_hit {
            FunctionStatus::WentDark
        } else if p_hit && b_exec && !b_hit {
            FunctionStatus::LitUp
        } else if n_lost > 0 && n_gained > 0 {
            FunctionStatus::Mixed
        } else if n_lost > 0 {
            FunctionStatus::PartialLoss
        } else if n_gained > 0 {
            FunctionStatus::PartialGain
        } else {
            FunctionStatus::Unchanged
        };
        out.push(delta(sp, status, n_lost, n_gained, executable));
    }
    out.sort_by(|a, b| {
        (a.start, a.end, &a.name, a.status.as_str()).cmp(&(
            b.start,
            b.end,
            &b.name,
            b.status.as_str(),
        ))
    });
    out
}

fn delta(s: &Span, status: FunctionStatus, lost: i64, gained: i64, n: i64) -> FunctionDelta {
    FunctionDelta {
        name: s.name.clone(),
        start: s.start,
        end: s.end,
        status,
        lost,
        gained,
        executable: n,
    }
}

fn count(lines: impl Iterator<Item = LineNo>, f: impl Fn(LineNo) -> bool) -> i64 {
    lines.filter(|&l| f(l)).count() as i64
}

/// (any executable, any hit) over non-flaky body lines.
fn body(span: &Span, lines: &Lines, flaky: &BTreeSet<LineNo>) -> (bool, bool) {
    let mut exec = false;
    let mut hit = false;
    for l in span.body_lines() {
        if let Some(&c) = lines.get(&l)
            && !flaky.contains(&l)
        {
            exec = true;
            hit |= c > 0;
        }
    }
    (exec, hit)
}

fn span_key(s: &Span) -> (LineNo, LineNo, &str) {
    (s.start, s.end, &s.name)
}

/// Pair spans by qualified name, then by largest line overlap after mapping.
fn match_spans<'a>(
    base: &'a [Span],
    patched: &'a [Span],
    lm: &LineMap,
) -> Vec<(Option<&'a Span>, Option<&'a Span>)> {
    let mut b_rest: Vec<&Span> = base.iter().collect();
    b_rest.sort_by(|a, b| span_key(a).cmp(&span_key(b)));
    let mut p_rest: Vec<&Span> = patched.iter().collect();
    p_rest.sort_by(|a, b| span_key(a).cmp(&span_key(b)));
    let b_sorted: Vec<Span> = b_rest.iter().map(|s| (*s).clone()).collect();
    let p_sorted: Vec<Span> = p_rest.iter().map(|s| (*s).clone()).collect();
    let b_named: BTreeMap<&str, &Span> = spans_by_name(&b_sorted).into_iter().collect();

    let mut pairs = Vec::new();
    for (name, sp) in spans_by_name(&p_sorted) {
        if let Some(sb) = b_named.get(name) {
            // The first span with this name is also the first equal one, as list.remove finds.
            let bi = b_rest.iter().position(|s| *s == *sb).expect("named span in rest");
            let pi = p_rest.iter().position(|s| *s == sp).expect("named span in rest");
            pairs.push((Some(b_rest.remove(bi)), Some(p_rest.remove(pi))));
        }
    }
    for sp in p_rest {
        let mapped: BTreeSet<LineNo> = (sp.start..=sp.end).filter_map(|l| lm.to_old(l)).collect();
        let mut best: Option<usize> = None;
        let mut best_n = 0;
        for (i, sb) in b_rest.iter().enumerate() {
            let n = mapped.iter().filter(|&&o| sb.start <= o && o <= sb.end).count();
            if n > best_n {
                best = Some(i);
                best_n = n;
            }
        }
        pairs.push((best.map(|i| b_rest.remove(i)), Some(sp)));
    }
    pairs.extend(b_rest.into_iter().map(|sb| (Some(sb), None)));
    pairs
}

/// Carry spans to the other side; spans with no surviving line are dropped.
fn map_spans(spans: &[Span], convert: impl Fn(LineNo) -> Option<LineNo>) -> Vec<Span> {
    let mut out = Vec::new();
    for s in spans {
        let mapped: Vec<LineNo> = (s.start..=s.end).filter_map(&convert).collect();
        let (Some(&lo), Some(&hi)) = (mapped.iter().min(), mapped.iter().max()) else {
            continue;
        };
        let body_start = s.body_start.map(|_| {
            let body = s.body_lines().filter_map(&convert).min();
            body.unwrap_or(hi + 1)
        });
        out.push(Span {
            name: s.name.clone(),
            start: lo,
            end: hi,
            body_start,
        });
    }
    out
}

/// Directory tree over repo-relative posix paths; dir stats are sums of their children.
pub fn build_tree<I: IntoIterator<Item = (PyText, Stats)>>(files: I) -> TreeNode {
    let entries = files
        .into_iter()
        .map(|(path, stats)| (path.split_ascii(b'/').collect(), stats))
        .collect();
    dir(PyText::default(), &[], entries)
}

fn dir(name: PyText, prefix: &[PyText], entries: Vec<(Vec<PyText>, Stats)>) -> TreeNode {
    let mut children = Vec::new();
    let mut groups: Vec<(PyText, Vec<(Vec<PyText>, Stats)>)> = Vec::new();
    for (mut parts, stats) in entries {
        if parts.len() == 1 {
            let mut path: Vec<PyText> = prefix.to_vec();
            path.push(parts[0].clone());
            children.push(TreeNode {
                name: parts.remove(0),
                path: join(&path),
                kind: NodeKind::File,
                stats,
                children: Vec::new(),
            });
        } else {
            let head = parts.remove(0);
            match groups.iter_mut().find(|(n, _)| *n == head) {
                Some((_, sub)) => sub.push((parts, stats)),
                None => groups.push((head, vec![(parts, stats)])),
            }
        }
    }
    for (n, sub) in groups {
        let mut p = prefix.to_vec();
        p.push(n.clone());
        children.push(dir(n, &p, sub));
    }
    children.sort_by(|a, b| (&a.name, a.kind).cmp(&(&b.name, b.kind)));
    let total = children
        .iter()
        .fold(Stats::default(), |acc, c| acc + c.stats);
    TreeNode {
        name,
        path: join(prefix),
        kind: NodeKind::Dir,
        stats: total,
        children,
    }
}

fn join(parts: &[PyText]) -> PyText {
    let mut out = PyText::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str("/");
        }
        out.push_text(p);
    }
    out
}
