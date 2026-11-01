//! The branch map behind `sundgren graph`: branches around the tip of a base branch, routed on a
//! grid of cells like a circuit board.
//!
//! - The base's first-parent history is the reference line.
//! - Each other branch ref is a lane: its first-parent history down to where it meets a lane
//!   already placed (its fork). A branch's local and remote refs are twin lanes, next to each
//!   other; commits they share are drawn on both, in the same column.
//! - Columns are the drawn commits in time order, parents first. The frame is a stretch of the
//!   base's history, as wide as `columns` allows.
//! - Which branches: the base, the target and `--pin` always; then, of the branches visible in
//!   the frame, the `branches` most contained in the target (`contain`'s matching).
//! - Lanes go above or below the reference line, wherever the fewest vertical lines have to pass
//!   under them. A lane passing under a vertical line is broken around it (`╴│╶`): a via.
//!
//! The grid is the drawing: [`text`] prints it as Unicode, the extension draws it as SVG.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::contain;
use crate::git::{self, RefKind, Tip};
use crate::{Error, Result};

pub const SCHEMA: &str = "sundgren.graph/2";

/// Cell sides, as in the schema.
pub const UP: u8 = 1;
pub const RIGHT: u8 = 2;
pub const DOWN: u8 = 4;
pub const LEFT: u8 = 8;

/// What to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub base: String,
    /// None: the checked-out branch, unless it is the base.
    pub target: Option<String>,
    /// How many branches besides the base, the target and the pinned ones.
    pub branches: usize,
    /// First-parent commits of the base back from its tip to the newest one in the frame.
    pub offset: usize,
    /// The most commit columns.
    pub columns: usize,
    /// Branch name patterns (`*` matches anything) whose lanes get the protected colour.
    pub protected: Vec<String>,
    pub pins: Vec<String>,
    /// Fuzzy threshold of the ranking, as `contain --fuzzy`.
    pub threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Named {
    pub name: String,
    pub oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Frame {
    pub offset: usize,
    pub older: Option<usize>,
    pub newer: Option<usize>,
    pub columns: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LaneKind {
    Reference,
    Local,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Only {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lane {
    pub name: String,
    pub branch: String,
    pub kind: LaneKind,
    pub row: usize,
    pub family: Option<usize>,
    pub twin: Option<usize>,
    pub protected: bool,
    pub pinned: bool,
    pub merged: bool,
    pub only: Option<Only>,
    pub ahead: usize,
    pub behind: usize,
    pub score: Option<f64>,
    pub fork: Option<String>,
    pub tip: String,
    pub before: Option<usize>,
    pub after: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitOut {
    pub oid: String,
    pub parents: Vec<String>,
    pub author: String,
    pub committed: i64,
    pub subject: String,
    pub refs: Vec<String>,
    pub column: usize,
    pub lanes: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Commit,
    Connector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub sides: u8,
    pub lane: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
}

/// The `sundgren.graph/2` document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MapDocument {
    pub schema: &'static str,
    pub repository: PathBuf,
    pub base: Named,
    pub target: Option<Named>,
    pub threshold: f64,
    pub frame: Frame,
    pub candidates: usize,
    pub lanes: Vec<Lane>,
    pub commits: Vec<CommitOut>,
    pub grid: Grid,
}

/// The repository's commits, indexed.
struct History {
    nodes: Vec<git::Node>,
    index: HashMap<String, usize>,
}

impl History {
    fn new(nodes: Vec<git::Node>) -> Self {
        let index = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.oid.clone(), i))
            .collect();
        History { nodes, index }
    }

    fn get(&self, oid: &str) -> Option<usize> {
        self.index.get(oid).copied()
    }

    fn time(&self, i: usize) -> i64 {
        self.nodes[i].committed
    }

    fn parents(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.nodes[i].parents.iter().filter_map(|p| self.get(p))
    }

    fn first_parent(&self, i: usize) -> Option<usize> {
        self.nodes[i].parents.first().and_then(|p| self.get(p))
    }

    /// Which commits `i` can reach, itself included.
    fn ancestors(&self, i: usize) -> Vec<bool> {
        let mut seen = vec![false; self.nodes.len()];
        let mut todo = vec![i];
        while let Some(c) = todo.pop() {
            if !std::mem::replace(&mut seen[c], true) {
                todo.extend(self.parents(c));
            }
        }
        seen
    }
}

/// A branch: its local ref, its remote ref, or both.
#[derive(Debug, Clone)]
struct Branch {
    name: String,
    local: Option<(String, usize)>,
    remote: Option<(String, usize)>,
}

impl Branch {
    fn refs(&self) -> impl Iterator<Item = (&str, usize, LaneKind)> {
        let local = self
            .local
            .iter()
            .map(|(n, i)| (n.as_str(), *i, LaneKind::Local));
        let remote = self
            .remote
            .iter()
            .map(|(n, i)| (n.as_str(), *i, LaneKind::Remote));
        local.chain(remote)
    }

    fn has(&self, name: &str) -> bool {
        self.refs().any(|(n, _, _)| n == name)
    }

    /// The ref its work is measured on: local if there is one.
    fn main_ref(&self) -> (&str, usize) {
        self.refs()
            .map(|(n, i, _)| (n, i))
            .next()
            .unwrap_or(("", 0))
    }
}

/// Branches from refs: a local branch with its upstream (or `origin/NAME`) as twin, and the
/// remote branches no local one tracks.
fn branches(tips: &[Tip], history: &History) -> Vec<Branch> {
    let commit = |t: &Tip| history.get(&t.oid).map(|i| (t.name.clone(), i));
    let remotes: BTreeMap<&str, &Tip> = tips
        .iter()
        .filter(|t| t.kind == RefKind::Remote)
        .map(|t| (t.name.as_str(), t))
        .collect();
    let mut claimed = HashSet::new();
    let mut out: Vec<Branch> = Vec::new();
    for t in tips.iter().filter(|t| t.kind == RefKind::Branch) {
        let Some(local) = commit(t) else { continue };
        let suffix = format!("/{}", t.name);
        let twin = t
            .upstream
            .as_deref()
            .and_then(|u| remotes.get(u))
            .or_else(|| remotes.get(format!("origin/{}", t.name).as_str()))
            .or_else(|| remotes.values().find(|r| r.name.ends_with(&suffix)))
            .copied();
        if let Some(r) = twin {
            claimed.insert(r.name.clone());
        }
        out.push(Branch {
            name: t.name.clone(),
            local: Some(local),
            remote: twin.and_then(commit),
        });
    }
    for r in remotes.values().filter(|r| !claimed.contains(&r.name)) {
        let Some(remote) = commit(r) else { continue };
        let short = r.name.split_once('/').map_or(r.name.as_str(), |(_, n)| n);
        let name = if out.iter().any(|b| b.name == short) {
            r.name.clone()
        } else {
            short.to_owned()
        };
        out.push(Branch {
            name,
            local: None,
            remote: Some(remote),
        });
    }
    out
}

/// `*` matches any run of characters.
fn glob(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, rest)) => {
            let Some(text) = text.strip_prefix(head) else {
                return false;
            };
            (0..=text.len())
                .filter(|&i| text.is_char_boundary(i))
                .any(|i| glob(rest, &text[i..]))
        }
    }
}

/// A ref's first-parent history down to the reference line: its commits, newest first, and the
/// commit on the line it left from.
fn walk_to_line(history: &History, on_line: &[bool], tip: usize) -> (Vec<usize>, Option<usize>) {
    let mut own = Vec::new();
    let mut c = Some(tip);
    while let Some(i) = c {
        if on_line[i] {
            return (own, Some(i));
        }
        own.push(i);
        c = history.first_parent(i);
    }
    (own, None)
}

/// The time span a branch's lanes cover: from their fork (or first commit) to their tip.
fn span(history: &History, on_line: &[bool], b: &Branch) -> Option<(i64, i64)> {
    let mut out: Option<(i64, i64)> = None;
    for (_, tip, _) in b.refs() {
        let (own, fork) = walk_to_line(history, on_line, tip);
        let Some(&first) = own.last() else { continue };
        let lo = history.time(fork.unwrap_or(first));
        let hi = own.iter().map(|&i| history.time(i)).max().unwrap_or(lo);
        out = Some(out.map_or((lo, hi), |(a, z)| (a.min(lo), z.max(hi))));
    }
    out
}

/// One lane before rows are known.
struct Draft {
    name: String,
    branch: String,
    kind: LaneKind,
    tip: usize,
    /// Its own commits, newest first (twin-shared ones included).
    own: Vec<usize>,
    fork: Option<usize>,
    unit: usize,
    protected: bool,
    pinned: bool,
    only: Option<Only>,
    score: Option<f64>,
}

/// Everything the routing needs about the frame.
struct Layout {
    drafts: Vec<Draft>,
    /// Drawn commits, in column order.
    columns: Vec<usize>,
    column: HashMap<usize, usize>,
    /// The lane that owns each drawn commit (draws its merges).
    owner: HashMap<usize, usize>,
    oldest: i64,
    newest: i64,
}

impl Layout {
    fn in_frame(&self, history: &History, i: usize) -> bool {
        (self.oldest..=self.newest).contains(&history.time(i))
    }

    fn x(&self, i: usize) -> Option<usize> {
        self.column.get(&i).map(|j| 3 * j + 2)
    }

    fn width(&self) -> usize {
        3 * self.columns.len() + 3
    }
}

/// What stays the same while the frame is fitted.
struct Context<'a> {
    history: &'a History,
    all: &'a [Branch],
    reference: &'a Draft,
    /// The base's first-parent history, newest first, and which commits are on it.
    line: &'a [usize],
    on_line: &'a [bool],
    pinned: &'a HashSet<usize>,
    protected: &'a dyn Fn(&str) -> bool,
}

/// Lay out the lanes of `units` (indices into `all`) for the frame `(oldest, newest)`.
fn lay_out(
    cx: &Context<'_>,
    units: &[usize],
    (oldest, newest): (i64, i64),
    scores: &HashMap<usize, Option<f64>>,
) -> Layout {
    let Context {
        history,
        all,
        reference,
        line: base_line,
        on_line,
        pinned,
        protected,
    } = *cx;
    // Longer branches take shared commits first, so a branch off a branch hangs off it.
    let mut order: Vec<usize> = units.to_vec();
    order.sort_by_key(|&u| {
        let len = all[u]
            .refs()
            .map(|(_, tip, _)| walk_to_line(history, on_line, tip).0.len())
            .max()
            .unwrap_or(0);
        (Reverse(len), all[u].name.clone())
    });

    let mut owner_of: HashMap<usize, usize> = base_line.iter().map(|&i| (i, 0)).collect();
    let mut drafts = vec![Draft {
        name: reference.name.clone(),
        branch: reference.branch.clone(),
        kind: LaneKind::Reference,
        tip: reference.tip,
        own: reference.own.clone(),
        fork: None,
        unit: usize::MAX,
        protected: reference.protected,
        pinned: true,
        only: None,
        score: None,
    }];
    for &u in &order {
        let b = &all[u];
        let only = match (&b.local, &b.remote) {
            (Some(_), None) => Some(Only::Local),
            (None, Some(_)) => Some(Only::Remote),
            _ => None,
        };
        let mut twin: Option<usize> = None;
        for (name, tip, kind) in b.refs() {
            // The reference's twin stops at the reference; other twins share commits.
            let is_reference_twin = b.name == reference.branch;
            let me = drafts.len();
            let mut own = Vec::new();
            let mut fork = None;
            let mut c = Some(tip);
            while let Some(i) = c {
                match owner_of.get(&i) {
                    Some(&o) if Some(o) != twin || is_reference_twin => {
                        fork = Some(i);
                        break;
                    }
                    Some(_) => {}
                    None => {
                        owner_of.insert(i, me);
                    }
                }
                own.push(i);
                c = history.first_parent(i);
            }
            if own.is_empty() {
                continue;
            }
            twin = Some(me);
            drafts.push(Draft {
                name: name.to_owned(),
                branch: b.name.clone(),
                kind,
                tip,
                own,
                fork,
                unit: u,
                protected: protected(&b.name),
                pinned: pinned.contains(&u),
                only,
                score: scores.get(&u).copied().flatten(),
            });
        }
    }

    // Drawn commits: the frame's part of each lane.
    let mut drawn: Vec<usize> = Vec::new();
    let mut seen = HashSet::new();
    for d in &drafts {
        for &i in &d.own {
            if (oldest..=newest).contains(&history.time(i)) && seen.insert(i) {
                drawn.push(i);
            }
        }
    }
    let columns = time_order(history, &drawn);
    let column = columns.iter().enumerate().map(|(j, &i)| (i, j)).collect();
    let owner = drawn
        .iter()
        .filter_map(|i| owner_of.get(i).map(|&o| (*i, o)))
        .collect();
    Layout {
        drafts,
        columns,
        column,
        owner,
        oldest,
        newest,
    }
}

/// Commits in time order, a parent always before its children.
fn time_order(history: &History, commits: &[usize]) -> Vec<usize> {
    let set: HashSet<usize> = commits.iter().copied().collect();
    let mut waiting: HashMap<usize, usize> = HashMap::new();
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for &c in commits {
        let parents: HashSet<usize> = history.parents(c).filter(|p| set.contains(p)).collect();
        waiting.insert(c, parents.len());
        for p in parents {
            children.entry(p).or_default().push(c);
        }
    }
    let key = |c: usize| Reverse((history.time(c), history.nodes[c].oid.clone()));
    let mut ready: BinaryHeap<_> = commits
        .iter()
        .filter(|c| waiting[c] == 0)
        .map(|&c| (key(c), c))
        .collect();
    let mut out = Vec::with_capacity(commits.len());
    while let Some((_, c)) = ready.pop() {
        out.push(c);
        for &child in children.get(&c).into_iter().flatten() {
            let n = waiting.get_mut(&child).map(|n| {
                *n -= 1;
                *n
            });
            if n == Some(0) {
                ready.push((key(child), child));
            }
        }
    }
    out
}

/// What each drawn lane looks like in the frame.
struct Span {
    /// First and last grid column of its line.
    start: usize,
    end: usize,
    /// It starts at its fork, drawn in the frame, rather than at the left edge.
    from_fork: Option<usize>,
    before: Option<usize>,
    after: bool,
}

/// A merge into a drawn commit from a commit on another lane.
struct Merge {
    from_lane: usize,
    to_lane: usize,
    /// Columns of the merged commit and of the merge commit.
    from_x: usize,
    x: usize,
    /// The merged commit is its lane's last one before the merge: the line can leave the lane
    /// at the merge commit's column. Otherwise it runs along a row of its own.
    direct: bool,
}

fn spans(history: &History, l: &Layout) -> (Vec<Option<Span>>, Vec<Merge>) {
    let width = l.width();
    let mut merges = Vec::new();
    for (&c, &to_lane) in &l.owner {
        for p in history.parents(c).skip(1) {
            if let (Some(&from_lane), Some(x), Some(from_x)) = (l.owner.get(&p), l.x(c), l.x(p))
                && from_lane != to_lane
            {
                let direct = !l.drafts[from_lane]
                    .own
                    .iter()
                    .filter_map(|&i| l.x(i))
                    .any(|ox| from_x < ox && ox < x);
                merges.push(Merge {
                    from_lane,
                    to_lane,
                    from_x,
                    x,
                    direct,
                });
            }
        }
    }
    merges.sort_by_key(|m| (m.x, m.from_lane, m.to_lane));

    let spans = l
        .drafts
        .iter()
        .enumerate()
        .map(|(k, d)| {
            let xs: Vec<usize> = d.own.iter().filter_map(|&i| l.x(i)).collect();
            let older = d
                .own
                .iter()
                .filter(|&&i| history.time(i) < l.oldest)
                .count();
            let newer = d.own.iter().any(|&i| history.time(i) > l.newest);
            let fork_x = d.fork.and_then(|f| l.x(f));
            // It starts in the frame at its fork, or at its root commit.
            let starts_here =
                fork_x.is_some() || (d.fork.is_none() && older == 0 && !xs.is_empty());
            let visible = !xs.is_empty() || (newer && (fork_x.is_some() || older > 0));
            if !visible {
                // A pinned lane outside the frame is a connector on the side it is on.
                return d.pinned.then(|| {
                    let x = if newer { width - 1 } else { 0 };
                    Span {
                        start: x,
                        end: x,
                        from_fork: None,
                        before: (!newer).then_some(older),
                        after: newer,
                    }
                });
            }
            let start = match fork_x {
                Some(x) => x,
                None if starts_here => xs.iter().copied().min().unwrap_or(0),
                None => 0,
            };
            let merge_end = merges
                .iter()
                .filter(|m| m.from_lane == k && m.direct)
                .map(|m| m.x)
                .max();
            let end = if newer {
                width - 1
            } else {
                xs.iter()
                    .copied()
                    .chain(merge_end)
                    .max()
                    .unwrap_or(start)
                    .max(start)
            };
            Some(Span {
                start,
                end,
                from_fork: fork_x,
                before: (!starts_here).then_some(older),
                after: newer,
            })
        })
        .collect();
    (spans, merges)
}

/// The grid being drawn, keyed by (row, col).
struct Board {
    cells: BTreeMap<(usize, usize), Cell>,
    vias: usize,
    /// Each row's lane; None for a row that only carries merge lines.
    rows: Vec<Option<usize>>,
}

impl Board {
    fn add(&mut self, row: usize, col: usize, sides: u8, lane: usize) -> &mut Cell {
        let cell = self.cells.entry((row, col)).or_insert(Cell {
            row,
            col,
            sides: 0,
            lane,
            node: None,
            commit: None,
        });
        cell.sides |= sides;
        cell
    }

    /// A vertical line at `col` between rows `a` and `b`, ends included.
    fn vertical(&mut self, col: usize, a: usize, b: usize, lane: usize) {
        let (top, bottom) = (a.min(b), a.max(b));
        if top == bottom {
            return;
        }
        self.add(top, col, DOWN, lane);
        self.add(bottom, col, UP, lane);
        for row in top + 1..bottom {
            let through = self
                .cells
                .get(&(row, col))
                .is_some_and(|c| c.node.is_none() && c.sides == LEFT | RIGHT);
            if through {
                // A via: the lane breaks around the vertical line.
                self.vias += 1;
                if let Some(c) = self.cells.get_mut(&(row, col)) {
                    c.sides = UP | DOWN;
                    c.lane = lane;
                }
                if let Some(c) = self.cells.get_mut(&(row, col - 1)) {
                    c.sides &= !RIGHT;
                }
                if let Some(c) = self.cells.get_mut(&(row, col + 1)) {
                    c.sides &= !LEFT;
                }
            } else {
                self.add(row, col, UP | DOWN, lane);
            }
        }
    }
}

/// Merge lines sharing a row: their (from, to) columns.
type Track = Vec<(usize, usize)>;

/// Draw the lanes in `lanes` order, top to bottom. A merge that can't leave its lane at the
/// merge commit's column gets a row next to its lane, on the side of the lane it merges into.
fn route(l: &Layout, spans: &[Option<Span>], merges: &[Merge], lanes: &[usize]) -> Board {
    let width = l.width();
    let pos: HashMap<usize, usize> = lanes.iter().enumerate().map(|(r, &k)| (k, r)).collect();

    // Merge rows: per (lane, side), tracks of merge lines that don't overlap.
    let mut tracks: BTreeMap<(usize, bool), Vec<Track>> = BTreeMap::new();
    let mut track_of: Vec<Option<(bool, usize)>> = vec![None; merges.len()];
    for (n, m) in merges.iter().enumerate() {
        let (Some(&a), Some(&b)) = (pos.get(&m.from_lane), pos.get(&m.to_lane)) else {
            continue;
        };
        if m.direct {
            continue;
        }
        let below = b > a;
        let slot = tracks.entry((m.from_lane, below)).or_default();
        let fits = |t: &Track| t.iter().all(|&(s, e)| m.x < s || e < m.from_x);
        let t = match slot.iter().position(fits) {
            Some(t) => t,
            None => {
                slot.push(Vec::new());
                slot.len() - 1
            }
        };
        slot[t].push((m.from_x, m.x));
        track_of[n] = Some((below, t));
    }
    let mut rows: Vec<Option<usize>> = Vec::new();
    let mut row_of: HashMap<usize, usize> = HashMap::new();
    let mut track_row: HashMap<(usize, bool, usize), usize> = HashMap::new();
    for &k in lanes {
        let above = tracks.get(&(k, false)).map_or(0, Vec::len);
        for t in (0..above).rev() {
            track_row.insert((k, false, t), rows.len());
            rows.push(None);
        }
        row_of.insert(k, rows.len());
        rows.push(Some(k));
        for t in 0..tracks.get(&(k, true)).map_or(0, Vec::len) {
            track_row.insert((k, true, t), rows.len());
            rows.push(None);
        }
    }

    let mut board = Board {
        cells: BTreeMap::new(),
        vias: 0,
        rows,
    };
    // Lines along the lanes and merge rows first, so vertical lines can tell where they cross.
    let along = |board: &mut Board, r: usize, from: usize, to: usize, k: usize| {
        for x in from..=to {
            let mut sides = 0;
            if x > from {
                sides |= LEFT;
            }
            if x < to {
                sides |= RIGHT;
            }
            board.add(r, x, sides, k);
        }
    };
    for (&k, &r) in &row_of {
        let Some(s) = &spans[k] else { continue };
        along(&mut board, r, s.start, s.end, k);
        if s.before.is_some() {
            board.add(r, 0, 0, k).node = Some(NodeKind::Connector);
        }
        if s.after {
            board.add(r, width - 1, 0, k).node = Some(NodeKind::Connector);
        }
        for &i in &l.drafts[k].own {
            if let Some(x) = l.x(i) {
                let cell = board.add(r, x, 0, k);
                cell.node = Some(NodeKind::Commit);
                cell.commit = l.column.get(&i).copied();
            }
        }
    }
    for (m, track) in merges.iter().zip(&track_of) {
        if let Some((below, t)) = track {
            along(
                &mut board,
                track_row[&(m.from_lane, *below, *t)],
                m.from_x,
                m.x,
                m.from_lane,
            );
        }
    }
    // Forks, then merges.
    for (&k, &r) in &row_of {
        let Some(s) = &spans[k] else { continue };
        let (Some(x), Some(f)) = (s.from_fork, l.drafts[k].fork) else {
            continue;
        };
        let Some(&parent) = l.owner.get(&f).and_then(|o| row_of.get(o)) else {
            continue;
        };
        board.vertical(x, parent, r, k);
    }
    for (m, track) in merges.iter().zip(&track_of) {
        let (Some(&a), Some(&b)) = (row_of.get(&m.from_lane), row_of.get(&m.to_lane)) else {
            continue;
        };
        match track {
            None => board.vertical(m.x, a, b, m.from_lane),
            Some((below, t)) => {
                let r = track_row[&(m.from_lane, *below, *t)];
                board.vertical(m.from_x, a, r, m.from_lane);
                board.vertical(m.x, r, b, m.from_lane);
            }
        }
    }
    board
}

/// Rows for the drawn lanes: the reference, and each branch's lanes together above or below it,
/// wherever the fewest vias are needed. Returns lane indices top to bottom.
fn place(l: &Layout, spans: &[Option<Span>], merges: &[Merge]) -> Vec<usize> {
    // Branches as groups of their drawn lanes (local first), latest fork first.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (k, d) in l.drafts.iter().enumerate().skip(1) {
        if spans[k].is_none() {
            continue;
        }
        match groups.iter_mut().find(|g| l.drafts[g[0]].unit == d.unit) {
            Some(g) => g.push(k),
            None => groups.push(vec![k]),
        }
    }
    groups.sort_by_key(|g| {
        Reverse(
            g.iter()
                .filter_map(|&k| spans[k].as_ref().map(|s| s.start))
                .min(),
        )
    });

    let mut above: Vec<Vec<usize>> = Vec::new();
    let mut below: Vec<Vec<usize>> = Vec::new();
    let rows = |above: &[Vec<usize>], below: &[Vec<usize>]| -> Vec<usize> {
        let mut out = Vec::new();
        for g in above.iter().rev() {
            out.extend(g.iter().rev());
        }
        out.push(0);
        for g in below {
            out.extend(g.iter());
        }
        out
    };
    for g in groups {
        let mut best: Option<(usize, bool, usize)> = None;
        for side_below in [true, false] {
            let len = if side_below { below.len() } else { above.len() };
            for at in 0..=len {
                let (mut a, mut b) = (above.clone(), below.clone());
                if side_below {
                    b.insert(at, g.clone());
                } else {
                    a.insert(at, g.clone());
                }
                let vias = route(l, spans, merges, &rows(&a, &b)).vias;
                // Fewest vias; then below; then farthest out.
                let better = match best {
                    None => true,
                    Some((v, _, _)) => vias < v,
                };
                if better {
                    best = Some((vias, side_below, at));
                } else if let Some((v, true, _)) = best
                    && vias == v
                    && side_below
                {
                    best = Some((vias, true, at));
                }
            }
        }
        if let Some((_, side_below, at)) = best {
            if side_below {
                below.insert(at, g);
            } else {
                above.insert(at, g);
            }
        }
    }
    rows(&above, &below)
}

/// The rank of a branch: how much of its work is found in the target's.
fn score(repo: &Path, base: &str, branch: &Branch, target: &Branch, threshold: f64) -> Option<f64> {
    let (a, _) = branch.main_ref();
    let (b, _) = target.main_ref();
    contain::containment_document(repo, base, a, b, contain::Options::new(threshold))
        .ok()
        .and_then(|doc| doc.a_in_b.containment)
}

/// Build the map.
pub fn map_document(repo: &Path, options: &Options) -> Result<MapDocument> {
    let repository = git::toplevel(repo)?;
    let history = History::new(git::all_commits(&repository)?);
    let tips = git::tips(&repository)?;
    let all = branches(&tips, &history);

    let base_oid = git::resolve(&repository, &options.base)?;
    let base_tip = history
        .get(&base_oid)
        .ok_or_else(|| Error::usage(format!("{} is not on a branch or tag", options.base)))?;
    let base_branch = all.iter().position(|b| b.has(&options.base));
    let base_name = base_branch.map_or(options.base.clone(), |u| all[u].name.clone());

    // The reference line: the base's first-parent history.
    let mut line = vec![base_tip];
    while let Some(p) = history.first_parent(line[line.len() - 1]) {
        line.push(p);
    }
    let mut on_line = vec![false; history.nodes.len()];
    for &i in &line {
        on_line[i] = true;
    }
    let base_anc = history.ancestors(base_tip);

    let target_name = match &options.target {
        Some(t) => Some(t.clone()),
        None => git::current_branch(&repository)?,
    }
    .filter(|t| !base_branch.is_some_and(|u| all[u].has(t)) && *t != options.base);
    let target = match &target_name {
        None => None,
        Some(t) => {
            let oid = git::resolve(&repository, t)?;
            let unit = all.iter().position(|b| b.has(t));
            Some((
                Named {
                    name: t.clone(),
                    oid,
                },
                unit,
            ))
        }
    };
    let target_unit = target.as_ref().and_then(|(_, u)| *u);

    let mut pinned: HashSet<usize> = HashSet::new();
    pinned.extend(base_branch);
    pinned.extend(target_unit);
    for p in &options.pins {
        let u = all
            .iter()
            .position(|b| b.has(p) || b.name == *p)
            .ok_or_else(|| Error::usage(format!("--pin {p}: no such branch")))?;
        pinned.insert(u);
    }
    let protected = |name: &str| options.protected.iter().any(|p| glob(p, name));

    let offset = options.offset.min(line.len() - 1);
    let newest = if offset == 0 {
        i64::MAX
    } else {
        history.time(line[offset])
    };
    let spans_of: Vec<Option<(i64, i64)>> =
        all.iter().map(|b| span(&history, &on_line, b)).collect();
    let mut scores: HashMap<usize, Option<f64>> = HashMap::new();
    let reference = Draft {
        name: options.base.clone(),
        branch: base_name.clone(),
        kind: LaneKind::Reference,
        tip: base_tip,
        own: line.clone(),
        fork: None,
        unit: usize::MAX,
        protected: protected(&base_name),
        pinned: true,
        only: None,
        score: None,
    };

    let cx = Context {
        history: &history,
        all: &all,
        reference: &reference,
        line: &line,
        on_line: &on_line,
        pinned: &pinned,
        protected: &protected,
    };
    let columns = options.columns.max(1);
    let mut k = columns.saturating_sub(1).min(line.len() - 1 - offset);
    let mut forced_oldest: Option<i64> = None;
    let (layout, candidates) = loop {
        let oldest = forced_oldest.unwrap_or_else(|| history.time(line[offset + k]));
        let window = (oldest, newest);
        let visible: Vec<usize> = (0..all.len())
            .filter(|u| !pinned.contains(u))
            .filter(|&u| spans_of[u].is_some_and(|(lo, hi)| lo <= newest && hi >= oldest))
            .collect();
        if let Some(tu) = target_unit {
            for &u in &visible {
                scores.entry(u).or_insert_with(|| {
                    score(
                        &repository,
                        &options.base,
                        &all[u],
                        &all[tu],
                        options.threshold,
                    )
                });
            }
        }
        let mut ranked = visible.clone();
        ranked.sort_by(|&a, &b| {
            let sa = scores.get(&a).copied().flatten();
            let sb = scores.get(&b).copied().flatten();
            let ta = spans_of[a].map_or(0, |s| s.1);
            let tb = spans_of[b].map_or(0, |s| s.1);
            // Highest score first, unscored last; then the newest tip.
            sb.partial_cmp(&sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(tb.cmp(&ta))
                .then(all[a].name.cmp(&all[b].name))
        });
        let mut units: Vec<usize> = pinned.iter().copied().collect();
        units.sort_unstable();
        units.extend(ranked.iter().take(options.branches));
        let layout = lay_out(&cx, &units, window, &scores);
        let n = layout.columns.len();
        if n <= columns || forced_oldest.is_some() {
            break (layout, visible.len());
        }
        if k == 0 {
            // Even one base commit is too many: keep the newest columns.
            forced_oldest = Some(history.time(layout.columns[n - columns]));
            continue;
        }
        k = (k * columns / n).min(k - 1);
    };

    let (spans, merges) = spans(&history, &layout);
    let lane_order = place(&layout, &spans, &merges);
    let board = route(&layout, &spans, &merges, &lane_order);
    let rows = board.rows.clone();

    // Lanes in row order, the reference first.
    let mut order: Vec<usize> = vec![0];
    order.extend(lane_order.iter().copied().filter(|&k| k != 0));
    let new_index: HashMap<usize, usize> = order.iter().enumerate().map(|(n, &k)| (k, n)).collect();
    let row_of: HashMap<usize, usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(r, k)| k.map(|k| (k, r)))
        .collect();

    // Colour families: reference, then units by first appearance in `order`, protected ones none.
    let mut families: HashMap<String, usize> = HashMap::new();
    let mut lanes = Vec::new();
    for &k in &order {
        let d = &layout.drafts[k];
        let s = spans[k].as_ref();
        let family = if d.protected {
            None
        } else {
            let next = families.len();
            Some(*families.entry(d.branch.clone()).or_insert(next))
        };
        let anc = history.ancestors(d.tip);
        let ahead = (0..anc.len()).filter(|&i| anc[i] && !base_anc[i]).count();
        let behind = (0..anc.len()).filter(|&i| base_anc[i] && !anc[i]).count();
        let twin = order
            .iter()
            .find(|&&j| j != k && layout.drafts[j].branch == d.branch);
        lanes.push(Lane {
            name: d.name.clone(),
            branch: d.branch.clone(),
            kind: d.kind,
            row: row_of[&k],
            family,
            twin: twin.map(|j| new_index[j]),
            protected: d.protected,
            pinned: d.pinned,
            merged: k != 0 && base_anc[d.tip],
            only: d.only,
            ahead,
            behind,
            score: d.score,
            fork: d.fork.map(|f| history.nodes[f].oid.clone()),
            tip: history.nodes[d.tip].oid.clone(),
            before: s.and_then(|s| s.before),
            after: s.is_some_and(|s| s.after),
        });
    }

    let refs_at: HashMap<&str, Vec<String>> = tips.iter().fold(HashMap::new(), |mut m, t| {
        m.entry(t.oid.as_str()).or_default().push(t.name.clone());
        m
    });
    let commits = layout
        .columns
        .iter()
        .enumerate()
        .map(|(j, &i)| {
            let n = &history.nodes[i];
            let mut on: Vec<usize> = order
                .iter()
                .filter(|&&k| layout.drafts[k].own.contains(&i) && spans[k].is_some())
                .map(|k| new_index[k])
                .collect();
            on.sort_unstable();
            CommitOut {
                oid: n.oid.clone(),
                parents: n.parents.clone(),
                author: n.author.clone(),
                committed: n.committed,
                subject: n.subject.clone(),
                refs: refs_at.get(n.oid.as_str()).cloned().unwrap_or_default(),
                column: 3 * j + 2,
                lanes: on,
            }
        })
        .collect();
    let cells = board
        .cells
        .into_values()
        .map(|mut c| {
            c.lane = new_index[&c.lane];
            c
        })
        .collect();

    let base_k = line.len() - 1 - offset;
    let shown = line[offset..]
        .iter()
        .take_while(|&&i| layout.in_frame(&history, i))
        .count();
    let step = (shown * 3 / 4).max(1);
    Ok(MapDocument {
        schema: SCHEMA,
        repository,
        base: Named {
            name: options.base.clone(),
            oid: base_oid,
        },
        target: target.map(|(t, _)| t),
        threshold: options.threshold,
        frame: Frame {
            offset,
            older: (shown <= base_k).then_some(offset + step.min(base_k)),
            newer: (offset > 0).then(|| offset.saturating_sub(step)),
            columns,
        },
        candidates,
        lanes,
        commits,
        grid: Grid {
            width: layout.width(),
            height: rows.len(),
            cells,
        },
    })
}

/// The glyph for a cell.
fn glyph(cell: &Cell, reference: bool) -> char {
    match cell.node {
        Some(NodeKind::Commit) => return if reference { '●' } else { '○' },
        Some(NodeKind::Connector) => return '⋯',
        None => {}
    }
    match cell.sides {
        s if s == LEFT | RIGHT => '─',
        s if s == UP | DOWN => '│',
        s if s == RIGHT | DOWN => '╭',
        s if s == LEFT | DOWN => '╮',
        s if s == UP | RIGHT => '╰',
        s if s == UP | LEFT => '╯',
        s if s == UP | RIGHT | DOWN => '├',
        s if s == UP | LEFT | DOWN => '┤',
        s if s == LEFT | RIGHT | DOWN => '┬',
        s if s == UP | LEFT | RIGHT => '┴',
        s if s == UP | RIGHT | DOWN | LEFT => '┼',
        LEFT => '╴',
        RIGHT => '╶',
        UP => '╵',
        DOWN => '╷',
        _ => ' ',
    }
}

/// What a lane's label says after its name.
fn state(lane: &Lane) -> String {
    let mut parts = Vec::new();
    if lane.kind == LaneKind::Reference {
        parts.push("base".to_owned());
    } else {
        parts.push(format!("↑{} ↓{}", lane.ahead, lane.behind));
    }
    if lane.protected {
        parts.push("protected".to_owned());
    }
    if lane.merged {
        parts.push("merged".to_owned());
    }
    match lane.only {
        Some(Only::Local) => parts.push("local only".to_owned()),
        Some(Only::Remote) => parts.push("remote only".to_owned()),
        None => {}
    }
    if let Some(s) = lane.score {
        parts.push(format!("{:.0}% in target", s * 100.0));
    }
    if let Some(n) = lane.before {
        parts.push(format!("{n} earlier"));
    }
    if lane.after {
        parts.push("more later".to_owned());
    }
    parts.join(" · ")
}

/// The map as Unicode: a header, a row of markers for refs on the reference line that are not
/// lanes, the grid, and a label per lane.
pub fn text(doc: &MapDocument) -> String {
    let mut out = String::new();
    let target = doc
        .target
        .as_ref()
        .map_or("none".to_owned(), |t| t.name.clone());
    out.push_str(&format!(
        "base {} · target {} · {} of {} branches in the frame\n",
        doc.base.name,
        target,
        doc.lanes
            .iter()
            .filter(|l| l.kind != LaneKind::Reference && !l.pinned)
            .map(|l| l.branch.as_str())
            .collect::<HashSet<_>>()
            .len(),
        doc.candidates,
    ));

    let width = doc.grid.width;
    let mut grid = vec![vec![' '; width]; doc.grid.height];
    for c in &doc.grid.cells {
        grid[c.row][c.col] = glyph(c, doc.lanes[c.lane].kind == LaneKind::Reference);
    }

    // Markers: refs on reference-line commits that are not drawn lanes. A marker that doesn't
    // fit next to the others goes on a row above them.
    let lane_names: HashSet<&str> = doc.lanes.iter().map(|l| l.name.as_str()).collect();
    let mut marker_rows: Vec<Vec<char>> = Vec::new();
    for c in &doc.commits {
        if !c.lanes.contains(&0) {
            continue;
        }
        let names: Vec<&str> = c
            .refs
            .iter()
            .map(String::as_str)
            .filter(|n| !lane_names.contains(n))
            .collect();
        if names.is_empty() {
            continue;
        }
        let label: Vec<char> = std::iter::once('▼')
            .chain(names.join(",").chars())
            .collect();
        let end = c.column + label.len();
        let free =
            |row: &Vec<char>| (c.column..=end).all(|x| row.get(x).is_none_or(|&ch| ch == ' '));
        let at = match marker_rows.iter().position(free) {
            Some(i) => i,
            None => {
                marker_rows.push(Vec::new());
                marker_rows.len() - 1
            }
        };
        let row = &mut marker_rows[at];
        if row.len() < end {
            row.resize(end, ' ');
        }
        row[c.column..end].copy_from_slice(&label);
    }
    for row in marker_rows.iter().rev() {
        out.push_str(row.iter().collect::<String>().trim_end());
        out.push('\n');
    }

    let name_width = doc
        .lanes
        .iter()
        .map(|l| l.name.chars().count())
        .max()
        .unwrap_or(0);
    for (r, row) in grid.iter().enumerate() {
        let line: String = row.iter().collect();
        match doc.lanes.iter().find(|l| l.row == r) {
            Some(lane) => {
                out.push_str(&format!(
                    "{line}  {:<name_width$}  {}",
                    lane.name,
                    state(lane)
                ));
            }
            None => out.push_str(&line),
        }
        let trimmed = out.trim_end_matches(' ').len();
        out.truncate(trimmed);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests;
