//! `sundgren relate`: how two clones of related repositories relate, from git history alone.
//!
//! Both are read as one ([`crate::join`]). Each side has a base (its default branch). Where they
//! split:
//!
//! - the copy point of each side: walking its base's first-parent chain, the first parent of the
//!   oldest commit the other base can't reach;
//! - the last sync: the merge base of the two bases;
//! - the newest commit both repositories have, on their bases or not.
//!
//! A commit is brought onto a base by one first-parent commit: itself, or the merge that brought
//! it in. That is found for all commits in one walk per base, and gives the merges that synced
//! one base into the other, and which base had each shared commit first.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::join::{self, Join, Source};
use crate::{Result, git};

pub const RELATE_SCHEMA: &str = "sundgren.relate/0";

/// What to compare: the bases (default: each repository's default branch) and the other
/// repository's name (default: its folder's name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub base: Option<String>,
    pub other_base: Option<String>,
    pub name: Option<String>,
}

/// A commit, for the reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Point {
    pub oid: String,
    /// Committer time, unix seconds.
    pub committed: i64,
    pub subject: String,
}

/// Committer times, unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Window {
    pub from: i64,
    pub until: i64,
}

/// Where a side's base left the other's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyPoint {
    /// The last commit of the first-parent chain both bases have before it.
    pub commit: Point,
    /// The oldest first-parent commit the other base doesn't have.
    pub first_own: Point,
    /// When the copy can have been made: from the copy point to the earlier of the other base's
    /// next first-parent commit and the first own commit.
    pub window: Window,
}

/// A first-parent merge on one base that brought in commits of the other base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncMerge {
    pub commit: Point,
    /// The commits of the other base it brought in.
    pub count: usize,
    /// The newest of them.
    pub newest: Point,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Side {
    pub repository: PathBuf,
    /// The folder's name, or `--name` for the other.
    pub name: String,
    pub base: String,
    pub base_oid: String,
    /// Commits reachable from its branches, remote branches, tags and HEAD.
    pub commits: usize,
    /// Commits reachable from its base.
    pub base_commits: usize,
    pub copy_point: Option<CopyPoint>,
    /// The merges into its base that brought in the other base's commits, oldest first.
    pub sync_merges: Vec<SyncMerge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shared {
    pub roots: Vec<String>,
    pub commits: usize,
}

/// The newest commit both repositories have, and the refs it is reachable from on a side whose
/// base doesn't have it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Newest {
    pub commit: Point,
    pub on_this_base: bool,
    pub on_other_base: bool,
    /// Short names in that repository (`feature`, `origin/feature`, `v1`).
    pub this_refs: Vec<String>,
    pub other_refs: Vec<String>,
}

/// The commits both bases have after the copy, by which base had them first: the one whose
/// first-parent commit that brought them in is older.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Direction {
    pub commits: usize,
    pub this_first: usize,
    pub other_first: usize,
    pub same_time: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarkKind {
    CopyPoint,
    LastSync,
    NewestShared,
    SyncMerge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Which {
    This,
    Other,
}

/// A commit to mark on a map: what it is, and for a copy point or sync merge, whose (the base it
/// is on).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mark {
    pub oid: String,
    pub kind: MarkKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<Which>,
    /// For a sync merge: the commits it brought in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelateDocument {
    pub schema: &'static str,
    /// Whether they share any commit. If not, only the sides' counts are filled in.
    pub related: bool,
    pub this: Side,
    pub other: Side,
    pub shared: Shared,
    pub last_sync: Option<Point>,
    pub newest_shared: Option<Newest>,
    /// The side whose copy point is older (an ancestor of the other's, or else earlier): the
    /// copy. None if neither base left the other.
    pub copy: Option<Which>,
    /// None if neither base left the other.
    pub direction: Option<Direction>,
    pub marks: Vec<Mark>,
}

/// Both histories, indexed.
struct Dag {
    nodes: Vec<git::Node>,
    index: HashMap<String, usize>,
    parents: Vec<Vec<usize>>,
}

impl Dag {
    fn new(nodes: Vec<git::Node>) -> Self {
        let index: HashMap<String, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.oid.clone(), i))
            .collect();
        let parents = nodes
            .iter()
            .map(|n| {
                n.parents
                    .iter()
                    .filter_map(|p| index.get(p).copied())
                    .collect()
            })
            .collect();
        Dag {
            nodes,
            index,
            parents,
        }
    }

    fn get(&self, oid: &str) -> Option<usize> {
        self.index.get(oid).copied()
    }

    fn time(&self, i: usize) -> i64 {
        self.nodes[i].committed
    }

    fn point(&self, i: usize) -> Point {
        let n = &self.nodes[i];
        Point {
            oid: n.oid.clone(),
            committed: n.committed,
            subject: n.subject.clone(),
        }
    }

    /// Which commits `tips` reach, themselves included.
    fn reach(&self, tips: impl IntoIterator<Item = usize>) -> Vec<bool> {
        let mut seen = vec![false; self.nodes.len()];
        let mut todo: Vec<usize> = tips.into_iter().collect();
        while let Some(c) = todo.pop() {
            if !std::mem::replace(&mut seen[c], true) {
                todo.extend(&self.parents[c]);
            }
        }
        seen
    }

    /// Which commits reach `i`, itself included.
    fn descendants(&self, i: usize) -> Vec<bool> {
        let mut children = vec![Vec::new(); self.nodes.len()];
        for (c, parents) in self.parents.iter().enumerate() {
            for &p in parents {
                children[p].push(c);
            }
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut todo = vec![i];
        while let Some(c) = todo.pop() {
            if !std::mem::replace(&mut seen[c], true) {
                todo.extend(&children[c]);
            }
        }
        seen
    }

    fn first_parent(&self, i: usize) -> Option<usize> {
        self.nodes[i].parents.first().and_then(|p| self.get(p))
    }

    /// The first-parent chain from `tip`, oldest first.
    fn chain(&self, tip: usize) -> Vec<usize> {
        let mut chain = vec![tip];
        while let Some(p) = chain.last().and_then(|&c| self.first_parent(c)) {
            chain.push(p);
        }
        chain.reverse();
        chain
    }

    /// For each commit `chain` (oldest first) reaches, the first-parent commit that brought it
    /// in: in one walk, each commit visited once.
    fn introduced(&self, chain: &[usize]) -> Vec<Option<usize>> {
        let mut by = vec![None; self.nodes.len()];
        for &m in chain {
            let mut todo = vec![m];
            while let Some(c) = todo.pop() {
                if by[c].is_none() {
                    by[c] = Some(m);
                    todo.extend(&self.parents[c]);
                }
            }
        }
        by
    }
}

/// One base, read against the other.
struct Base {
    tip: usize,
    chain: Vec<usize>,
    on: Vec<bool>,
    by: Vec<Option<usize>>,
}

impl Base {
    fn new(dag: &Dag, tip: usize) -> Self {
        let chain = dag.chain(tip);
        let by = dag.introduced(&chain);
        Base {
            tip,
            on: dag.reach([tip]),
            chain,
            by,
        }
    }
}

/// Where `base` left `other`: (copy point, first own commit).
fn copy_point(dag: &Dag, base: &Base, other: &Base) -> Option<(usize, usize)> {
    let first_own = *base.chain.iter().find(|&&c| !other.on[c])?;
    let point = dag.first_parent(first_own)?;
    Some((point, first_own))
}

fn copy_out(dag: &Dag, (point, first_own): (usize, usize), other: &Base) -> CopyPoint {
    let next = other.by[point]
        .and_then(|m| other.chain.iter().position(|&c| c == m))
        .and_then(|at| other.chain.get(at + 1))
        .map(|&c| dag.time(c));
    let own = dag.time(first_own);
    CopyPoint {
        commit: dag.point(point),
        first_own: dag.point(first_own),
        window: Window {
            from: dag.time(point),
            until: next.map_or(own, |t| t.min(own)),
        },
    }
}

/// The merges on `base`'s first-parent chain that brought in commits of `other`'s base, oldest
/// first.
fn sync_merges(dag: &Dag, base: &Base, other: &Base) -> Vec<SyncMerge> {
    let mut found: HashMap<usize, (usize, usize)> = HashMap::new();
    for (c, by) in base.by.iter().enumerate() {
        let Some(m) = *by else { continue };
        if m != c && other.on[c] && !other.on[m] {
            let entry = found.entry(m).or_insert((0, c));
            entry.0 += 1;
            if dag.time(c) > dag.time(entry.1) {
                entry.1 = c;
            }
        }
    }
    base.chain
        .iter()
        .filter_map(|m| {
            let &(count, newest) = found.get(m)?;
            Some(SyncMerge {
                commit: dag.point(*m),
                count,
                newest: dag.point(newest),
            })
        })
        .collect()
}

/// A ref's short name: without `refs/heads/`, `refs/remotes/` or `refs/tags/`.
fn short(full: &str) -> &str {
    ["refs/heads/", "refs/remotes/", "refs/tags/"]
        .iter()
        .find_map(|p| full.strip_prefix(p))
        .unwrap_or(full)
}

/// How `repo` and `other` relate.
pub fn relate_document(repo: &Path, other: &Path, options: &Options) -> Result<RelateDocument> {
    let joined = join::join(repo, other, options.name.as_deref())?;
    joined_document(&joined, options)
}

/// As [`relate_document`], on a join already made.
pub fn joined_document(joined: &Join, options: &Options) -> Result<RelateDocument> {
    let (repo, other) = (joined.this_repo.as_path(), joined.other_repo.as_path());
    let this_base = match &options.base {
        Some(b) => b.clone(),
        None => git::default_branch(repo)?,
    };
    let other_base = match &options.other_base {
        Some(b) => b.clone(),
        None => git::default_branch(other)?,
    };
    let this_oid = git::resolve(repo, &this_base)?;
    let other_oid = git::resolve(other, &other_base)?;
    let dag = Dag::new(git::commits_with(&joined.dir, &[&this_oid, &other_oid])?);
    relate(joined, &dag, (this_base, this_oid), (other_base, other_oid))
}

fn mark(oid: &str, kind: MarkKind, side: Option<Which>, count: Option<usize>) -> Mark {
    Mark {
        oid: oid.to_owned(),
        kind,
        side,
        count,
    }
}

/// The copy point and sync merges of `side`, whose base is `base`, and their marks.
fn fill(dag: &Dag, side: &mut Side, which: Which, (base, other): (&Base, &Base)) -> Vec<Mark> {
    let mut marks = Vec::new();
    if let Some(point) = copy_point(dag, base, other) {
        let out = copy_out(dag, point, other);
        marks.push(mark(
            &out.commit.oid,
            MarkKind::CopyPoint,
            Some(which),
            None,
        ));
        side.copy_point = Some(out);
    }
    side.sync_merges = sync_merges(dag, base, other);
    marks.extend(side.sync_merges.iter().map(|m| {
        mark(
            &m.commit.oid,
            MarkKind::SyncMerge,
            Some(which),
            Some(m.count),
        )
    }));
    marks
}

/// The copy: the older of the two copy points (an ancestor of the other, or else the earlier).
fn fork(dag: &Dag, a: &Base, b: &Base) -> Option<(Which, usize)> {
    let points = [copy_point(dag, a, b), copy_point(dag, b, a)].map(|p| p.map(|(c, _)| c));
    match points {
        [Some(x), Some(y)] => {
            let y_first = dag.reach([x])[y] || (!dag.reach([y])[x] && dag.time(y) < dag.time(x));
            Some(if y_first {
                (Which::Other, y)
            } else {
                (Which::This, x)
            })
        }
        [Some(x), None] => Some((Which::This, x)),
        [None, y] => y.map(|y| (Which::Other, y)),
    }
}

/// Which base had the commits both have after `fork` first.
fn direction(dag: &Dag, a: &Base, b: &Base, fork: usize) -> Direction {
    let before = dag.reach([fork]);
    let mut d = Direction {
        commits: 0,
        this_first: 0,
        other_first: 0,
        same_time: 0,
    };
    for c in (0..dag.nodes.len()).filter(|&c| a.on[c] && b.on[c] && !before[c]) {
        let (Some(x), Some(y)) = (a.by[c], b.by[c]) else {
            continue;
        };
        d.commits += 1;
        match dag.time(x).cmp(&dag.time(y)) {
            std::cmp::Ordering::Less => d.this_first += 1,
            std::cmp::Ordering::Greater => d.other_first += 1,
            std::cmp::Ordering::Equal => d.same_time += 1,
        }
    }
    d
}

/// The short names of `source`'s refs that reach commit `c`.
fn reaching(joined: &Join, dag: &Dag, source: Source, c: usize) -> Vec<String> {
    let below = dag.descendants(c);
    joined
        .refs
        .iter()
        .filter(|r| r.source == source && dag.get(&r.commit).is_some_and(|i| below[i]))
        .map(|r| short(&r.original).to_owned())
        .collect()
}

fn relate(
    joined: &Join,
    dag: &Dag,
    (this_base, this_oid): (String, String),
    (other_base, other_oid): (String, String),
) -> Result<RelateDocument> {
    let tips = |source: Source| -> Vec<usize> {
        joined
            .refs
            .iter()
            .filter(|r| r.source == source)
            .filter_map(|r| dag.get(&r.commit))
            .collect()
    };
    let missing = |oid: &str| crate::Error::failure(format!("commit {oid} not in the history"));
    let a = Base::new(dag, dag.get(&this_oid).ok_or_else(|| missing(&this_oid))?);
    let b = Base::new(dag, dag.get(&other_oid).ok_or_else(|| missing(&other_oid))?);
    let in_this = dag.reach(tips(Source::This).into_iter().chain([a.tip]));
    let in_other = dag.reach(tips(Source::Other).into_iter().chain([b.tip]));
    let shared: Vec<usize> = (0..dag.nodes.len())
        .filter(|&c| in_this[c] && in_other[c])
        .collect();
    let count = |v: &[bool]| v.iter().filter(|&&x| x).count();
    let side = |repository: &Path, name: String, base, oid, all: &[bool], on: &[bool]| Side {
        repository: git::toplevel(repository).unwrap_or_else(|_| repository.to_path_buf()),
        name,
        base,
        base_oid: oid,
        commits: count(all),
        base_commits: count(on),
        copy_point: None,
        sync_merges: Vec::new(),
    };
    let this_name = git::toplevel(&joined.this_repo)
        .ok()
        .and_then(|t| t.file_name().map(|n| join::ref_safe(&n.to_string_lossy())))
        .unwrap_or_else(|| "this".to_owned());
    let mut doc = RelateDocument {
        schema: RELATE_SCHEMA,
        related: !shared.is_empty(),
        this: side(
            &joined.this_repo,
            this_name,
            this_base,
            this_oid,
            &in_this,
            &a.on,
        ),
        other: side(
            &joined.other_repo,
            joined.name.clone(),
            other_base,
            other_oid,
            &in_other,
            &b.on,
        ),
        shared: Shared {
            roots: shared
                .iter()
                .filter(|&&c| dag.nodes[c].parents.is_empty())
                .map(|&c| dag.nodes[c].oid.clone())
                .collect(),
            commits: shared.len(),
        },
        last_sync: None,
        newest_shared: None,
        copy: None,
        direction: None,
        marks: Vec::new(),
    };
    let Some(&newest) = shared.iter().max_by_key(|&&c| dag.time(c)) else {
        return Ok(doc);
    };

    let mut marks = fill(dag, &mut doc.this, Which::This, (&a, &b));
    marks.extend(fill(dag, &mut doc.other, Which::Other, (&b, &a)));
    let last_sync = git::merge_base(&joined.dir, &doc.this.base_oid, &doc.other.base_oid)?
        .and_then(|oid| dag.get(&oid));
    if let Some(c) = last_sync {
        marks.push(mark(&dag.nodes[c].oid, MarkKind::LastSync, None, None));
    }
    marks.push(mark(
        &dag.nodes[newest].oid,
        MarkKind::NewestShared,
        None,
        None,
    ));
    let refs = |source, on: bool| {
        if on {
            Vec::new()
        } else {
            reaching(joined, dag, source, newest)
        }
    };
    doc.newest_shared = Some(Newest {
        commit: dag.point(newest),
        on_this_base: a.on[newest],
        on_other_base: b.on[newest],
        this_refs: refs(Source::This, a.on[newest]),
        other_refs: refs(Source::Other, b.on[newest]),
    });
    doc.last_sync = last_sync.map(|c| dag.point(c));
    let fork = fork(dag, &a, &b);
    doc.copy = fork.map(|(which, _)| which);
    doc.direction = fork.map(|(_, f)| direction(dag, &a, &b, f));
    doc.marks = marks;
    Ok(doc)
}

/// Unix seconds as `YYYY-MM-DD HH:MM` UTC.
pub fn utc(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let minutes = seconds.rem_euclid(86_400) / 60;
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        minutes / 60,
        minutes % 60
    )
}

fn short_oid(oid: &str) -> &str {
    oid.get(..8).unwrap_or(oid)
}

fn line(p: &Point) -> String {
    format!("{} {}  {}", short_oid(&p.oid), utc(p.committed), p.subject)
}

/// Up to `n` names, and how many more.
fn some(names: &[String], n: usize) -> String {
    let mut out = names.iter().take(n).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > n {
        out.push_str(&format!(" and {} more", names.len() - n));
    }
    out
}

/// A short report.
pub fn text(doc: &RelateDocument) -> String {
    let mut out = String::new();
    for s in [&doc.this, &doc.other] {
        out.push_str(&format!(
            "{} ({}): {} commits, {} on {} ({})\n",
            s.name,
            s.repository.display(),
            s.commits,
            s.base_commits,
            s.base,
            short_oid(&s.base_oid)
        ));
    }
    if !doc.related {
        out.push_str("unrelated: no shared commits\n");
        return out;
    }
    let roots = doc.shared.roots.len();
    out.push_str(&format!(
        "shared: {} commits, {roots} root{}\n",
        doc.shared.commits,
        if roots == 1 { "" } else { "s" }
    ));
    for (s, o, which) in [
        (&doc.this, &doc.other, Which::This),
        (&doc.other, &doc.this, Which::Other),
    ] {
        match &s.copy_point {
            Some(c) => {
                out.push_str(&format!(
                    "{}'s {} left {}'s {} at {}\n  first own {}\n",
                    s.name,
                    s.base,
                    o.name,
                    o.base,
                    line(&c.commit),
                    line(&c.first_own),
                ));
                if doc.copy == Some(which) {
                    out.push_str(&format!(
                        "  the copy: made between {} and {}\n",
                        utc(c.window.from),
                        utc(c.window.until)
                    ));
                }
            }
            None => out.push_str(&format!(
                "{}'s {} has nothing {}'s {} lacks\n",
                s.name, s.base, o.name, o.base
            )),
        }
    }
    if let Some(p) = &doc.last_sync {
        out.push_str(&format!("last sync (merge base): {}\n", line(p)));
    }
    if let Some(n) = &doc.newest_shared {
        out.push_str(&format!("newest shared: {}\n", line(&n.commit)));
        for (s, on, refs) in [
            (&doc.this, n.on_this_base, &n.this_refs),
            (&doc.other, n.on_other_base, &n.other_refs),
        ] {
            if !on {
                out.push_str(&format!(
                    "  not on {}'s {}; reaches it through {}\n",
                    s.name,
                    s.base,
                    some(refs, 5)
                ));
            }
        }
    }
    for (s, o) in [(&doc.this, &doc.other), (&doc.other, &doc.this)] {
        if s.sync_merges.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "merges into {}'s {} from {}'s {}:\n",
            s.name, s.base, o.name, o.base
        ));
        for m in &s.sync_merges {
            out.push_str(&format!(
                "  {} {}  +{} up to {}  {}\n",
                short_oid(&m.commit.oid),
                utc(m.commit.committed),
                m.count,
                short_oid(&m.newest.oid),
                m.commit.subject
            ));
        }
    }
    if let Some(d) = &doc.direction {
        out.push_str(&format!(
            "after the copy, {} commits are on both bases: {} first on {}, {} first on {}, {} at the same time\n",
            d.commits, d.this_first, doc.this.name, d.other_first, doc.other.name, d.same_time
        ));
    }
    out
}

#[cfg(test)]
mod tests;
