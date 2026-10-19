//! Function spans, function statuses and the directory tree.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ops::{Add, RangeInclusive};

use serde::Serialize;

use crate::linemap::LineMap;
use crate::model::{Count, FileCoverage, LineNo};
use crate::str_enum;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// Qualified, e.g. "Class.method".
    pub name: String,
    pub start: LineNo,
    pub end: LineNo,
    /// First body line; lines before it (signature) don't count.
    pub body_start: Option<LineNo>,
}

impl Span {
    pub fn body_lines(&self) -> RangeInclusive<LineNo> {
        self.body_start.unwrap_or(self.start)..=self.end
    }
}

/// Spans from lcov FN records, sorted; None when the file has none. A missing end runs to the
/// next function's start - 1, or the last executable line.
///
/// The FN line is the signature, which tools report hit on import or entry, so the body starts
/// one line later. A one-line function has no separate body.
pub fn lcov_spans(fc: Option<&FileCoverage>) -> Option<Vec<Span>> {
    let fc = fc.filter(|fc| !fc.functions.is_empty())?;
    let mut funcs: Vec<_> = fc.functions.iter().collect();
    funcs.sort_by(|a, b| (a.start, &a.name).cmp(&(b.start, &b.name)));
    let last_line = fc.lines.keys().next_back().copied().unwrap_or(0);
    let mut spans: Vec<Span> = funcs
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let end =
                f.end
                    .unwrap_or_else(|| match funcs[i + 1..].iter().find(|g| g.start > f.start) {
                        Some(g) => g.start - 1,
                        None => last_line.max(f.start),
                    });
            let end = end.max(f.start);
            Span {
                name: f.name.clone(),
                start: f.start,
                end,
                body_start: (end > f.start).then_some(f.start + 1),
            }
        })
        .collect();
    spans.sort_by(|a, b| span_key(a).cmp(&span_key(b)));
    Some(spans)
}

str_enum!(FunctionStatus {
    WentDark = "went_dark",
    LitUp = "lit_up",
    PartialLoss = "partial_loss",
    PartialGain = "partial_gain",
    Mixed = "mixed",
    Unchanged = "unchanged",
    Added = "added",
    Removed = "removed",
});

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
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

impl Add for Stats {
    type Output = Stats;

    fn add(self, o: Stats) -> Stats {
        Stats {
            lost: self.lost + o.lost,
            gained: self.gained + o.gained,
            new_hit: self.new_hit + o.new_hit,
            new_miss: self.new_miss + o.new_miss,
            deleted_hit: self.deleted_hit + o.deleted_hit,
            deleted_miss: self.deleted_miss + o.deleted_miss,
            executable_base: self.executable_base + o.executable_base,
            executable_patched: self.executable_patched + o.executable_patched,
            hit_base: self.hit_base + o.hit_base,
            hit_patched: self.hit_patched + o.hit_patched,
        }
    }
}

str_enum!(
    /// `Dir` sorts before `File`, as "dir" < "file".
    NodeKind { Dir = "dir", File = "file" }
);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub kind: NodeKind,
    pub stats: Stats,
    /// None for a file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
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

    let count =
        |s: &Span, f: &dyn Fn(LineNo) -> bool| s.body_lines().filter(|&l| f(l)).count() as i64;
    let mut out = Vec::new();
    for (sb, sp) in match_spans(&base_spans, &patched_spans, lm) {
        let delta = |s: &Span, status, lost, gained, executable| FunctionDelta {
            name: s.name.clone(),
            start: s.start,
            end: s.end,
            status,
            lost,
            gained,
            executable,
        };
        let Some(sp) = sp else {
            let sb = sb.expect("a pair has at least one span");
            let n = count(sb, &|l| base_lines.contains_key(&l));
            out.push(delta(sb, FunctionStatus::Removed, 0, 0, n));
            continue;
        };
        let executable = count(sp, &|l| patched_lines.contains_key(&l));
        let Some(sb) = sb else {
            out.push(delta(sp, FunctionStatus::Added, 0, 0, executable));
            continue;
        };
        let n_lost = count(sp, &|l| sets.lost.contains(&l));
        let n_gained = count(sp, &|l| sets.gained.contains(&l));
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

/// Pair spans by qualified name (first span per name), then by largest line overlap after
/// mapping. Each pair has at least one side; unmatched spans pair with None.
pub fn match_spans<'a>(
    base: &'a [Span],
    patched: &'a [Span],
    lm: &LineMap,
) -> Vec<(Option<&'a Span>, Option<&'a Span>)> {
    let mut b_rest: Vec<&Span> = base.iter().collect();
    b_rest.sort_by(|a, b| span_key(a).cmp(&span_key(b)));
    let mut p_rest: Vec<&Span> = patched.iter().collect();
    p_rest.sort_by(|a, b| span_key(a).cmp(&span_key(b)));
    let first_by_name = |spans: &[&'a Span]| -> Vec<&'a Span> {
        let mut seen = HashSet::new();
        spans
            .iter()
            .copied()
            .filter(|s| seen.insert(s.name.as_str()))
            .collect()
    };
    let b_named: BTreeMap<&str, &Span> = first_by_name(&b_rest)
        .into_iter()
        .map(|s| (s.name.as_str(), s))
        .collect();

    let mut pairs = Vec::new();
    for sp in first_by_name(&p_rest) {
        if let Some(sb) = b_named.get(sp.name.as_str()) {
            let bi = b_rest.iter().position(|s| *s == *sb).expect("named span");
            let pi = p_rest.iter().position(|s| *s == sp).expect("named span");
            pairs.push((Some(b_rest.remove(bi)), Some(p_rest.remove(pi))));
        }
    }
    for sp in p_rest {
        let mapped: BTreeSet<LineNo> = (sp.start..=sp.end).filter_map(|l| lm.to_old(l)).collect();
        let mut best: Option<usize> = None;
        let mut best_n = 0;
        for (i, sb) in b_rest.iter().enumerate() {
            let n = mapped
                .iter()
                .filter(|&&o| sb.start <= o && o <= sb.end)
                .count();
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
        let body_start = s
            .body_start
            .map(|_| s.body_lines().filter_map(&convert).min().unwrap_or(hi + 1));
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
pub fn build_tree<'a>(files: impl IntoIterator<Item = (&'a str, Stats)>) -> TreeNode {
    let entries = files
        .into_iter()
        .map(|(path, stats)| (path.split('/').collect(), stats))
        .collect();
    dir("", &[], entries)
}

/// Path components and stats of one file.
type Entry<'a> = (Vec<&'a str>, Stats);

fn dir(name: &str, prefix: &[&str], entries: Vec<Entry<'_>>) -> TreeNode {
    let mut children = Vec::new();
    let mut groups: Vec<(&str, Vec<Entry<'_>>)> = Vec::new();
    for (parts, stats) in entries {
        if let [file] = parts[..] {
            let path = [prefix, &[file]].concat().join("/");
            children.push(TreeNode {
                name: file.to_string(),
                path,
                kind: NodeKind::File,
                stats,
                children: None,
            });
        } else {
            let (head, tail) = (parts[0], parts[1..].to_vec());
            match groups.iter_mut().find(|(n, _)| *n == head) {
                Some((_, sub)) => sub.push((tail, stats)),
                None => groups.push((head, vec![(tail, stats)])),
            }
        }
    }
    for (n, sub) in groups {
        children.push(dir(n, &[prefix, &[n]].concat(), sub));
    }
    children.sort_by(|a, b| (&a.name, a.kind).cmp(&(&b.name, b.kind)));
    let stats = children
        .iter()
        .fold(Stats::default(), |acc, c| acc + c.stats);
    TreeNode {
        name: name.to_string(),
        path: prefix.join("/"),
        kind: NodeKind::Dir,
        stats,
        children: Some(children),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FunctionRecord;

    fn span(name: &str, start: LineNo, end: LineNo, body: Option<LineNo>) -> Span {
        Span {
            name: name.into(),
            start,
            end,
            body_start: body,
        }
    }

    fn f() -> Span {
        span("f", 1, 4, Some(2)) // line 1 is the signature
    }

    fn g() -> Span {
        span("g", 6, 7, Some(7))
    }

    fn lm(deleted: &[LineNo], added: &[LineNo]) -> LineMap {
        LineMap {
            deleted: deleted.to_vec(),
            added: added.to_vec(),
            ..LineMap::identity("m.py")
        }
    }

    fn l(pairs: &[(LineNo, Count)]) -> Lines {
        pairs.iter().copied().collect()
    }

    fn set(v: &[LineNo]) -> BTreeSet<LineNo> {
        v.iter().copied().collect()
    }

    fn status(
        base: &[(LineNo, Count)],
        patched: &[(LineNo, Count)],
        lost: &[LineNo],
        gained: &[LineNo],
    ) -> (&'static str, i64, i64, i64) {
        let sets = LineSets {
            lost: set(lost),
            gained: set(gained),
            ..Default::default()
        };
        let out = function_deltas(
            Some(&[f()]),
            Some(&[f()]),
            &lm(&[], &[]),
            &l(base),
            &l(patched),
            &sets,
        );
        let [fd] = &out[..] else { panic!("{out:?}") };
        (fd.status.as_str(), fd.lost, fd.gained, fd.executable)
    }

    #[test]
    fn statuses() {
        // The signature line still runs at import; it doesn't keep f lit.
        let full = [(1, 1), (2, 1), (3, 1), (4, 1)];
        let dark = [(1, 1), (2, 0), (3, 0), (4, 0)];
        assert_eq!(
            status(&full, &dark, &[2, 3, 4], &[]),
            ("went_dark", 3, 0, 3)
        );
        assert_eq!(
            status(
                &[(1, 1), (2, 0), (3, 0)],
                &[(1, 1), (2, 1), (3, 1)],
                &[],
                &[2, 3]
            ),
            ("lit_up", 0, 2, 2)
        );
        assert_eq!(
            status(&[(2, 1), (3, 1)], &[(2, 1), (3, 0)], &[3], &[]),
            ("partial_loss", 1, 0, 2)
        );
        assert_eq!(
            status(&[(2, 1), (3, 0)], &[(2, 1), (3, 1)], &[], &[3]),
            ("partial_gain", 0, 1, 2)
        );
        assert_eq!(
            status(&[(2, 1), (3, 0)], &[(2, 0), (3, 1)], &[2], &[3]),
            ("mixed", 1, 1, 2)
        );
        assert_eq!(
            status(&[(2, 1), (3, 0)], &[(2, 1), (3, 0)], &[], &[]),
            ("unchanged", 0, 0, 2)
        );
    }

    #[test]
    fn added_removed_and_no_spans() {
        let id = lm(&[], &[]);
        let none = LineSets::default();
        let key = |v: Vec<FunctionDelta>| -> Vec<_> {
            v.into_iter()
                .map(|d| (d.name, d.status.as_str(), d.start, d.end, d.executable))
                .collect()
        };
        let added = function_deltas(
            Some(&[]),
            Some(&[g()]),
            &id,
            &l(&[]),
            &l(&[(6, 1), (7, 0)]),
            &none,
        );
        assert_eq!(key(added), [("g".to_string(), "added", 6, 7, 1)]);
        let removed = function_deltas(
            Some(&[g()]),
            Some(&[]),
            &id,
            &l(&[(6, 1), (7, 1)]),
            &l(&[]),
            &none,
        );
        assert_eq!(key(removed), [("g".to_string(), "removed", 6, 7, 1)]);
        assert!(function_deltas(None, None, &id, &l(&[(1, 1)]), &l(&[(1, 0)]), &none).is_empty());
        let sorted = function_deltas(
            Some(&[g(), f()]),
            Some(&[g(), f()]),
            &id,
            &l(&[]),
            &l(&[]),
            &none,
        );
        assert_eq!(
            sorted.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["f", "g"]
        );
    }

    #[test]
    fn flaky_lines_do_not_decide_went_dark() {
        let sets = LineSets {
            lost: set(&[2]),
            flaky_patched: set(&[3]),
            ..Default::default()
        };
        let out = function_deltas(
            Some(&[f()]),
            Some(&[f()]),
            &lm(&[], &[]),
            &l(&[(2, 1), (3, 1)]),
            &l(&[(2, 0), (3, 1)]),
            &sets,
        );
        assert_eq!(out[0].status, FunctionStatus::WentDark);
    }

    #[test]
    fn renamed_function_matches_by_overlap() {
        let sets = LineSets {
            lost: set(&[2, 3]),
            ..Default::default()
        };
        let out = function_deltas(
            Some(&[span("old", 1, 3, Some(2))]),
            Some(&[span("new", 1, 3, Some(2))]),
            &lm(&[1], &[1]),
            &l(&[(1, 1), (2, 1), (3, 1)]),
            &l(&[(1, 1), (2, 0), (3, 0)]),
            &sets,
        );
        let [fd] = &out[..] else { panic!("{out:?}") };
        assert_eq!(
            (fd.name.as_str(), fd.status),
            ("new", FunctionStatus::WentDark)
        );
    }

    #[test]
    fn unknown_side_borrows_spans_through_the_linemap() {
        let sets = LineSets {
            lost: set(&[4, 5]),
            ..Default::default()
        };
        let out = function_deltas(
            Some(&[f()]),
            None,
            &lm(&[], &[1, 2]),
            &l(&[(2, 1), (3, 1)]),
            &l(&[(4, 0), (5, 0)]),
            &sets,
        );
        let [fd] = &out[..] else { panic!("{out:?}") };
        assert_eq!(
            (fd.name.as_str(), fd.status, fd.start, fd.end),
            ("f", FunctionStatus::WentDark, 3, 6)
        );
    }

    // --- lcov FN spans ---

    fn fns(funcs: &[(&str, LineNo, Option<LineNo>)], lines: &[(LineNo, Count)]) -> Vec<Span> {
        let fc = FileCoverage {
            path: "m.rs".into(),
            lines: l(lines),
            functions: funcs
                .iter()
                .map(|&(name, start, end)| FunctionRecord {
                    name: name.into(),
                    start,
                    end,
                    hits: Some(1),
                })
                .collect(),
            ..Default::default()
        };
        lcov_spans(Some(&fc)).unwrap()
    }

    #[test]
    fn lcov_spans_cases() {
        let s = span;
        assert_eq!(
            fns(&[("f", 2, Some(5)), ("g", 7, Some(9))], &[(2, 1), (9, 1)]),
            [s("f", 2, 5, Some(3)), s("g", 7, 9, Some(8))]
        );
        // A missing end runs to the next start, the last one to the last executable line.
        assert_eq!(
            fns(
                &[("g", 8, None), ("f", 3, None)],
                &[(3, 1), (4, 1), (8, 1), (9, 1)]
            ),
            [s("f", 3, 7, Some(4)), s("g", 8, 9, Some(9))]
        );
        assert_eq!(
            fns(&[("f", 3, None)], &[(1, 1), (3, 1), (4, 0), (12, 0)]),
            [s("f", 3, 12, Some(4))]
        );
        assert_eq!(fns(&[("f", 3, None)], &[]), [s("f", 3, 3, None)]);
        assert_eq!(fns(&[("f", 30, None)], &[(1, 1)]), [s("f", 30, 30, None)]);
        // Same start (outer fn and closure): run to the next distinct start.
        assert_eq!(
            fns(
                &[("a", 2, None), ("b", 2, None), ("c", 6, None)],
                &[(2, 1), (7, 1)]
            ),
            [
                s("a", 2, 5, Some(3)),
                s("b", 2, 5, Some(3)),
                s("c", 6, 7, Some(7))
            ]
        );
        assert_eq!(fns(&[("f", 5, Some(3))], &[(5, 1)]), [s("f", 5, 5, None)]);
        assert_eq!(
            fns(
                &[("f", 1, Some(4)), ("g", 6, None), ("h", 10, None)],
                &[(11, 1)]
            ),
            [
                s("f", 1, 4, Some(2)),
                s("g", 6, 9, Some(7)),
                s("h", 10, 11, Some(11))
            ]
        );
        // The FN line is the signature: only the body may go dark.
        let [divide] = &fns(&[("divide", 1, None)], &[(1, 1), (2, 0), (3, 0), (5, 0)])[..] else {
            panic!()
        };
        assert_eq!(divide.body_lines().collect::<Vec<_>>(), [2, 3, 4, 5]);
        assert_eq!(lcov_spans(None), None);
        assert_eq!(lcov_spans(Some(&FileCoverage::default())), None);
    }

    #[test]
    fn body_lines() {
        assert_eq!(span("f", 3, 7, Some(5)).body_lines(), 5..=7);
        assert_eq!(span("f", 3, 7, None).body_lines(), 3..=7);
    }

    // --- tree ---

    fn stats(lost: i64, gained: i64, new_miss: i64, exec_b: i64, exec_p: i64) -> Stats {
        Stats {
            lost,
            gained,
            new_miss,
            executable_base: exec_b,
            executable_patched: exec_p,
            ..Default::default()
        }
    }

    #[test]
    fn stats_add() {
        let a = Stats {
            hit_patched: 2,
            ..stats(1, 0, 0, 3, 0)
        };
        let b = stats(2, 1, 0, 1, 0);
        assert_eq!(
            a + b,
            Stats {
                hit_patched: 2,
                ..stats(3, 1, 0, 4, 0)
            }
        );
    }

    #[test]
    fn tree_sums_and_sorts() {
        let tree = build_tree([
            ("pkg/sub/b.py", stats(1, 0, 0, 2, 0)),
            ("pkg/a.py", stats(0, 2, 0, 0, 5)),
            ("top.py", stats(0, 0, 1, 0, 0)),
            ("pkg/sub/a.py", stats(4, 0, 0, 0, 0)),
        ]);
        assert_eq!(
            (tree.name.as_str(), tree.path.as_str(), tree.kind),
            ("", "", NodeKind::Dir)
        );
        assert_eq!(tree.stats, stats(5, 2, 1, 2, 5));
        let [pkg, top] = &tree.children.as_ref().unwrap()[..] else {
            panic!()
        };
        assert_eq!((pkg.path.as_str(), pkg.kind), ("pkg", NodeKind::Dir));
        assert_eq!((top.path.as_str(), top.kind), ("top.py", NodeKind::File));
        let pkg_children = pkg.children.as_ref().unwrap();
        assert_eq!(
            pkg_children
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["a.py", "sub"]
        );
        let sub = &pkg_children[1];
        assert_eq!((sub.path.as_str(), sub.stats.lost), ("pkg/sub", 5));
        let leaves: Vec<_> = sub
            .children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| (c.name.as_str(), c.path.as_str(), c.children.is_none()))
            .collect();
        assert_eq!(
            leaves,
            [
                ("a.py", "pkg/sub/a.py", true),
                ("b.py", "pkg/sub/b.py", true)
            ]
        );
    }

    #[test]
    fn file_and_dir_may_share_a_name() {
        let tree = build_tree([("a", Stats::default()), ("a/b", Stats::default())]);
        let kinds: Vec<_> = tree
            .children
            .unwrap()
            .iter()
            .map(|c| (c.name.clone(), c.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("a".to_string(), NodeKind::Dir),
                ("a".to_string(), NodeKind::File)
            ]
        );
    }

    #[test]
    fn empty_tree() {
        let tree = build_tree([]);
        let want = TreeNode {
            name: String::new(),
            path: String::new(),
            kind: NodeKind::Dir,
            stats: Stats::default(),
            children: Some(vec![]),
        };
        assert_eq!(tree, want);
        let keys: Vec<String> = serde_json::to_value(Stats::default())
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let mut want = [
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
        want.sort();
        assert_eq!(keys, want);
    }
}
