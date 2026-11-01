//! The branch map behind `sundgren graph`: branches around the tip of a base branch, routed on a
//! grid of cells like a circuit board. Time runs down, newest first, like `git log --graph`.
//!
//! - The base's first-parent history is the reference lane.
//! - Each other branch ref is a lane: its first-parent history down to where it meets a lane
//!   already placed (its fork). A branch's local and remote refs are twin lanes, next to each
//!   other, sharing the rows of shared commits; one lane when both point at the same commit.
//! - Rows are the drawn commits in time order. The page is the newest `commits` of them.
//! - Which branches: the base, `--like` and `--pin` always; then, of the branches with commits on
//!   the page, the first `branches` by the `show` mode, less `--hide`.
//! - Lanes go left or right of the reference, wherever the fewest lines have to cross them. A
//!   lane crossed by a line is broken around it (`╵─╷`): a via.
//! - Runs of commits on one lane, with nothing else happening, are folds: one row when folded.
//!
//! The layout is worked out with time running left to right ([`lay_out`], [`route`]) and then
//! turned ([`turn`]). The grid is the drawing: [`text`] prints it as Unicode, the extension draws
//! it as SVG.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::contain;
use crate::git::{self, RefKind, Tip};
use crate::{Error, Result};

pub const SCHEMA: &str = "sundgren.graph/3";

/// Cell sides, as in the schema.
pub const UP: u8 = 1;
pub const RIGHT: u8 = 2;
pub const DOWN: u8 = 4;
pub const LEFT: u8 = 8;

/// How the branches besides the base are chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Show {
    /// Local branches first, then remote ones; newest work first.
    Local,
    /// Newest work first.
    Recent,
    /// Branches with commits by the git `user.email` first; newest work first.
    Mine,
    /// Most contained in this branch first (`contain`'s matching).
    Like(String),
}

/// What to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub base: String,
    pub show: Show,
    /// How many branches besides the base and the pinned ones.
    pub branches: usize,
    /// The most commits on the page.
    pub commits: usize,
    /// Branch name patterns (`*` matches anything) whose lanes get the protected colour.
    pub protected: Vec<String>,
    /// Branches always shown.
    pub pins: Vec<String>,
    /// Branches never shown (besides the base and the `like` branch); unknown names are ignored.
    pub hides: Vec<String>,
    /// Fuzzy threshold of the ranking, as `contain --fuzzy`.
    pub threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Named {
    pub name: String,
    pub oid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Local,
    Recent,
    Mine,
    Like,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowOut {
    pub mode: Mode,
    pub like: Option<Named>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Page {
    pub commits: usize,
    pub more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fold {
    pub lane: usize,
    pub rows: [usize; 2],
    pub commits: Vec<usize>,
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
    pub column: usize,
    pub label_row: usize,
    pub family: Option<usize>,
    pub twin: Option<usize>,
    pub remote: Option<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitOut {
    pub oid: String,
    pub parents: Vec<String>,
    pub author: String,
    pub committed: i64,
    pub subject: String,
    pub body: String,
    pub refs: Vec<String>,
    pub row: usize,
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
    pub show: ShowOut,
    pub threshold: f64,
    pub page: Page,
    /// The branches that can be shown besides the base and the `like` branch, in `show` order.
    pub branches: Vec<String>,
    pub lanes: Vec<Lane>,
    pub commits: Vec<CommitOut>,
    pub folds: Vec<Fold>,
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
    /// A local lane's remote ref at the same commit: drawn as this one lane.
    remote: Option<String>,
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
    /// Commits are two cells apart, so a via always has a free cell on each side.
    fn x(&self, i: usize) -> Option<usize> {
        self.column.get(&i).map(|j| 2 * j + 2)
    }

    fn width(&self) -> usize {
        2 * self.columns.len() + 3
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
        remote: reference.remote.clone(),
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
        // In sync: the local lane stands for both.
        let synced = match (&b.local, &b.remote) {
            (Some((_, l)), Some((r, rt))) if l == rt => Some(r.clone()),
            _ => None,
        };
        for (name, tip, kind) in b.refs() {
            if kind == LaneKind::Remote && synced.is_some() {
                continue;
            }
            // Shared history is drawn once, on the lane that came first (the local one): a twin's
            // lane holds only what the other doesn't have.
            let me = drafts.len();
            let mut own = Vec::new();
            let mut fork = None;
            let mut c = Some(tip);
            while let Some(i) = c {
                if owner_of.contains_key(&i) {
                    fork = Some(i);
                    break;
                }
                owner_of.insert(i, me);
                own.push(i);
                c = history.first_parent(i);
            }
            if own.is_empty() {
                continue;
            }
            drafts.push(Draft {
                name: name.to_owned(),
                branch: b.name.clone(),
                kind,
                tip,
                own,
                fork,
                unit: u,
                remote: if kind == LaneKind::Local {
                    synced.clone()
                } else {
                    None
                },
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
    /// Each lane row's lane (they are two grid rows apart); None for a row that only carries
    /// merge lines.
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
    // Lanes and merge rows are two grid rows apart: lines between them have room to turn.
    for &k in lanes {
        let above = tracks.get(&(k, false)).map_or(0, Vec::len);
        for t in (0..above).rev() {
            track_row.insert((k, false, t), 2 * rows.len());
            rows.push(None);
        }
        row_of.insert(k, 2 * rows.len());
        rows.push(Some(k));
        for t in 0..tracks.get(&(k, true)).map_or(0, Vec::len) {
            track_row.insert((k, true, t), 2 * rows.len());
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

/// Colour families the renderers have; past them, family counts on (a second round).
pub const FAMILIES: usize = 12;

/// A family for `branch`: its own (from a hash of the name) if free, else the next free one.
fn free_family(branch: &str, taken: &HashMap<String, usize>) -> usize {
    // FNV-1a: stable across runs and platforms.
    let hash = branch.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    let own = (hash % FAMILIES as u64) as usize;
    (0..)
        .map(|i| (own + i) % FAMILIES + i / FAMILIES * FAMILIES)
        .find(|f| !taken.values().any(|t| t == f))
        .unwrap_or(own)
}

/// The rank of a branch in mode `like`: how much of its work is found in the other's.
fn score(repo: &Path, base: &str, branch: &Branch, like: &Branch, threshold: f64) -> Option<f64> {
    let (a, _) = branch.main_ref();
    let (b, _) = like.main_ref();
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
    // The base's remote twin, when it points at the base tip, is the reference lane too.
    let base_remote = base_branch.and_then(|u| {
        let b = &all[u];
        b.refs()
            .find(|&(name, tip, _)| name != options.base && tip == base_tip)
            .map(|(name, _, _)| name.to_owned())
    });

    // The reference lane: the base's first-parent history.
    let mut line = vec![base_tip];
    while let Some(p) = history.first_parent(line[line.len() - 1]) {
        line.push(p);
    }
    let mut on_line = vec![false; history.nodes.len()];
    for &i in &line {
        on_line[i] = true;
    }
    let base_anc = history.ancestors(base_tip);

    let like = match &options.show {
        Show::Like(name) => {
            let oid = git::resolve(&repository, name)?;
            let unit = all.iter().position(|b| b.has(name) || b.name == *name);
            Some((
                Named {
                    name: name.clone(),
                    oid,
                },
                unit,
            ))
        }
        _ => None,
    };
    let like_unit = like.as_ref().and_then(|(_, u)| *u);

    let mut pinned: HashSet<usize> = HashSet::new();
    pinned.extend(base_branch);
    pinned.extend(like_unit);
    // Pinned by hand: listed with the others, but always shown.
    let mut picked: HashSet<usize> = HashSet::new();
    for p in &options.pins {
        let u = all
            .iter()
            .position(|b| b.has(p) || b.name == *p)
            .ok_or_else(|| Error::usage(format!("--pin {p}: no such branch")))?;
        if !pinned.contains(&u) {
            picked.insert(u);
        }
    }
    let hidden: HashSet<usize> = options
        .hides
        .iter()
        .filter_map(|h| all.iter().position(|b| b.has(h) || b.name == *h))
        .filter(|u| !pinned.contains(u))
        .collect();
    // Drawn whatever the page shows: a connector when it has nothing on it.
    let always: HashSet<usize> = pinned.union(&picked).copied().collect();
    let protected = |name: &str| options.protected.iter().any(|p| glob(p, name));

    // Per branch: the span of its lanes, and whether it has commits by the git user.
    let spans_of: Vec<Option<(i64, i64)>> =
        all.iter().map(|b| span(&history, &on_line, b)).collect();
    let me = match options.show {
        Show::Mine => git::user_email(&repository)?,
        _ => None,
    };
    let mine: Vec<bool> = all
        .iter()
        .map(|b| {
            me.as_ref().is_some_and(|email| {
                b.refs().any(|(_, tip, _)| {
                    let (own, _) = walk_to_line(&history, &on_line, tip);
                    own.iter().any(|&i| history.nodes[i].email == *email)
                })
            })
        })
        .collect();
    let mut scores: HashMap<usize, Option<f64>> = HashMap::new();
    let reference = Draft {
        name: options.base.clone(),
        branch: base_name.clone(),
        kind: LaneKind::Reference,
        tip: base_tip,
        own: line.clone(),
        fork: None,
        unit: usize::MAX,
        remote: base_remote,
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
        pinned: &always,
        protected: &protected,
    };

    // The page: the base's newest `commits` first-parent commits, and every commit of the drawn
    // lanes since the oldest of them.
    let commits = options.commits.max(1);
    let oldest = history.time(line[(commits - 1).min(line.len() - 1)]);
    let newest = i64::MAX;
    let visible: Vec<usize> = (0..all.len())
        .filter(|u| !pinned.contains(u))
        .filter(|&u| {
            picked.contains(&u) || spans_of[u].is_some_and(|(lo, hi)| lo <= newest && hi >= oldest)
        })
        .collect();
    if let Some(lu) = like_unit {
        for &u in &visible {
            scores.insert(
                u,
                score(
                    &repository,
                    &options.base,
                    &all[u],
                    &all[lu],
                    options.threshold,
                ),
            );
        }
    }
    let newest_work = |u: usize| spans_of[u].map_or(0, |s| s.1);
    let mut ranked = visible.clone();
    ranked.sort_by(|&a, &b| {
        let first = match options.show {
            Show::Local => all[b].local.is_some().cmp(&all[a].local.is_some()),
            Show::Recent => std::cmp::Ordering::Equal,
            Show::Mine => mine[b].cmp(&mine[a]),
            // Highest score first, unscored last.
            Show::Like(_) => {
                let sa = scores.get(&a).copied().flatten();
                let sb = scores.get(&b).copied().flatten();
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            }
        };
        first
            .then(newest_work(b).cmp(&newest_work(a)))
            .then(all[a].name.cmp(&all[b].name))
    });
    // The first `branches` by rank and the picked ones, less the hidden: hiding one doesn't
    // bring in the next.
    let mut units: Vec<usize> = pinned.iter().copied().collect();
    units.sort_unstable();
    units.extend(
        ranked
            .iter()
            .enumerate()
            .filter(|&(rank, u)| rank < options.branches || picked.contains(u))
            .map(|(_, &u)| u)
            .filter(|u| !hidden.contains(u)),
    );
    let layout = lay_out(&cx, &units, (oldest, newest), &scores);
    let branch_names: Vec<String> = ranked.iter().map(|&u| all[u].name.clone()).collect();
    let more = layout
        .drafts
        .iter()
        .any(|d| d.own.iter().any(|&i| history.time(i) < layout.oldest));

    let (spans, merges) = spans(&history, &layout);
    let lane_order = place(&layout, &spans, &merges);
    let board = route(&layout, &spans, &merges, &lane_order);
    let Turned {
        cells: turned_cells,
        width: grid_width,
        height: grid_height,
        top,
        left,
        kept,
    } = turn(&board, layout.width());

    // Lanes: the reference first, then left to right.
    let lane_column: HashMap<usize, usize> = board
        .rows
        .iter()
        .enumerate()
        .filter_map(|(r, k)| k.map(|k| (k, 2 * r - left)))
        .collect();
    let mut order: Vec<usize> = lane_column.keys().copied().collect();
    order.sort_by_key(|&k| (k != 0, lane_column[&k]));
    let new_index: HashMap<usize, usize> = order.iter().enumerate().map(|(n, &k)| (k, n)).collect();

    let refs_at: HashMap<&str, Vec<String>> = tips.iter().fold(HashMap::new(), |mut m, t| {
        m.entry(t.oid.as_str()).or_default().push(t.name.clone());
        m
    });
    let commit_row = |i: usize| {
        layout
            .x(i)
            .and_then(|x| kept.get(top - x).copied().flatten())
    };
    let commits_out: Vec<CommitOut> = {
        let mut out: Vec<(usize, CommitOut)> = layout
            .columns
            .iter()
            .filter_map(|&i| {
                let row = commit_row(i)?;
                let n = &history.nodes[i];
                let mut on: Vec<usize> = order
                    .iter()
                    .filter(|&&k| layout.drafts[k].own.contains(&i))
                    .map(|k| new_index[k])
                    .collect();
                on.sort_unstable();
                Some((
                    row,
                    CommitOut {
                        oid: n.oid.clone(),
                        parents: n.parents.clone(),
                        author: n.author.clone(),
                        committed: n.committed,
                        subject: n.subject.clone(),
                        body: n.body.clone(),
                        refs: refs_at.get(n.oid.as_str()).cloned().unwrap_or_default(),
                        row,
                        lanes: on,
                    },
                ))
            })
            .collect();
        out.sort_by_key(|(row, _)| *row);
        out.into_iter().map(|(_, c)| c).collect()
    };
    let index_of_row: HashMap<usize, usize> = commits_out
        .iter()
        .enumerate()
        .map(|(n, c)| (c.row, n))
        .collect();
    let cells: Vec<Cell> = turned_cells
        .into_iter()
        .map(|mut c| {
            c.lane = new_index[&c.lane];
            if c.node == Some(NodeKind::Commit) {
                c.commit = index_of_row.get(&c.row).copied();
            }
            c
        })
        .collect();

    // Colour families: from the branch name, so a branch keeps its colour between loads; the
    // next free one if taken. Protected lanes have none.
    let mut families: HashMap<String, usize> = HashMap::new();
    let mut lanes = Vec::new();
    for &k in &order {
        let d = &layout.drafts[k];
        let s = spans[k].as_ref();
        let family = if d.protected {
            None
        } else if let Some(&f) = families.get(&d.branch) {
            Some(f)
        } else {
            let f = free_family(&d.branch, &families);
            families.insert(d.branch.clone(), f);
            Some(f)
        };
        let anc = history.ancestors(d.tip);
        let ahead = (0..anc.len()).filter(|&i| anc[i] && !base_anc[i]).count();
        let behind = (0..anc.len()).filter(|&i| base_anc[i] && !anc[i]).count();
        let twin = order
            .iter()
            .find(|&&j| j != k && layout.drafts[j].branch == d.branch);
        let column = lane_column[&k];
        let me = new_index[&k];
        let label_row = d
            .own
            .iter()
            .find_map(|&i| commit_row(i))
            .or_else(|| {
                cells
                    .iter()
                    .find(|c| c.lane == me && c.node == Some(NodeKind::Connector))
                    .map(|c| c.row)
            })
            .unwrap_or(0);
        lanes.push(Lane {
            name: d.name.clone(),
            branch: d.branch.clone(),
            kind: d.kind,
            column,
            label_row,
            family,
            twin: twin.map(|j| new_index[j]),
            remote: d.remote.clone(),
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
        });
    }
    let folds = folds(&cells, &commits_out, &lanes);

    Ok(MapDocument {
        schema: SCHEMA,
        repository,
        base: Named {
            name: options.base.clone(),
            oid: base_oid,
        },
        show: ShowOut {
            mode: match options.show {
                Show::Local => Mode::Local,
                Show::Recent => Mode::Recent,
                Show::Mine => Mode::Mine,
                Show::Like(_) => Mode::Like,
            },
            like: like.map(|(n, _)| n),
        },
        threshold: options.threshold,
        page: Page { commits, more },
        branches: branch_names,
        lanes,
        commits: commits_out,
        folds,
        grid: Grid {
            width: grid_width,
            height: grid_height,
            cells,
        },
    })
}

/// The grid turned so time runs down: what was column `x` (time) is row `top - x`, what was row
/// `r` (lane) is column `r - left`. Empty rows at the top and columns at the left are dropped.
struct Turned {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
    top: usize,
    left: usize,
    /// Rows left after dropping spacer rows that only carry straight lines: the new row of each
    /// old one (`top - x`), if kept.
    kept: Vec<Option<usize>>,
}

fn turn(board: &Board, width: usize) -> Turned {
    // Left (older) goes down, right (newer) up; up (the lane before) goes left, down right.
    let sides = |s: u8| {
        let mut out = 0;
        for (from, to) in [(LEFT, DOWN), (RIGHT, UP), (UP, LEFT), (DOWN, RIGHT)] {
            if s & from != 0 {
                out |= to;
            }
        }
        out
    };
    let cells: Vec<&Cell> = board.cells.values().collect();
    let max_x = cells.iter().map(|c| c.col).max().unwrap_or(0);
    let min_r = cells.iter().map(|c| c.row).min().unwrap_or(0);
    let top = max_x.min(width - 1);
    let turned: Vec<Cell> = cells
        .iter()
        .map(|c| Cell {
            row: top - c.col,
            col: c.row - min_r,
            sides: sides(c.sides),
            lane: c.lane,
            node: c.node,
            commit: c.commit,
        })
        .collect();
    // A row with no node and only straight lines down says nothing: drop it.
    let rows = turned.iter().map(|c| c.row + 1).max().unwrap_or(0);
    let mut busy = vec![false; rows];
    for c in &turned {
        if c.node.is_some() || c.sides != UP | DOWN {
            busy[c.row] = true;
        }
    }
    let mut kept = vec![None; rows];
    let mut height = 0;
    for (r, &b) in busy.iter().enumerate() {
        if b {
            kept[r] = Some(height);
            height += 1;
        }
    }
    let wide = turned.iter().map(|c| c.col + 1).max().unwrap_or(0);
    let mut cells: Vec<Cell> = turned
        .into_iter()
        .filter_map(|mut c| {
            c.row = kept[c.row]?;
            Some(c)
        })
        .collect();
    cells.sort_by_key(|c| (c.row, c.col));
    Turned {
        cells,
        width: wide,
        height,
        top,
        left: min_r,
        kept,
    }
}

/// Runs of three or more commits on one lane, one after the other, with nothing else in their
/// rows: other lanes only pass straight down, and no ref points into the run (but at its first
/// commit).
fn folds(cells: &[Cell], commits: &[CommitOut], lanes: &[Lane]) -> Vec<Fold> {
    const LEAST: usize = 3;
    let at: HashMap<(usize, usize), &Cell> = cells.iter().map(|c| ((c.row, c.col), c)).collect();
    let mut by_row: BTreeMap<usize, Vec<&Cell>> = BTreeMap::new();
    for c in cells {
        by_row.entry(c.row).or_default().push(c);
    }
    let lane_names: HashSet<&str> = lanes.iter().map(|l| l.name.as_str()).collect();
    // Only straight lines down, but in `column`.
    let quiet = |row: usize, column: usize| {
        by_row
            .get(&row)
            .into_iter()
            .flatten()
            .all(|c| c.col == column || (c.node.is_none() && c.sides == UP | DOWN))
    };
    let plain = |row: usize, column: usize, n: usize| {
        at.get(&(row, column)).is_some_and(|c| {
            c.node == Some(NodeKind::Commit)
                && c.sides == UP | DOWN
                && commits[n].lanes.len() == 1
                && commits[n]
                    .refs
                    .iter()
                    .all(|r| !lane_names.contains(r.as_str()))
                && commits[n].refs.is_empty()
        })
    };
    let mut out = Vec::new();
    for (k, lane) in lanes.iter().enumerate() {
        let column = lane.column;
        let mine: Vec<usize> = (0..commits.len())
            .filter(|&n| commits[n].lanes.contains(&k))
            .filter(|&n| {
                at.get(&(commits[n].row, column))
                    .is_some_and(|c| c.node.is_some())
            })
            .collect();
        let mut run: Vec<usize> = Vec::new();
        let mut close = |run: &mut Vec<usize>| {
            if run.len() >= LEAST {
                out.push(Fold {
                    lane: k,
                    rows: [commits[run[0]].row, commits[run[run.len() - 1]].row],
                    commits: run.clone(),
                });
            }
            run.clear();
        };
        for &n in &mine {
            let row = commits[n].row;
            let extends = run.last().is_some_and(|&last| {
                let prev = commits[last].row;
                n == last + 1 && (prev + 1..=row).all(|r| quiet(r, column)) && plain(row, column, n)
            });
            if !extends {
                close(&mut run);
            }
            run.push(n);
        }
        close(&mut run);
    }
    out.sort_by_key(|f| f.rows[0]);
    out
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
fn state(lane: &Lane, like: Option<&Named>) -> String {
    let mut parts = Vec::new();
    if lane.kind == LaneKind::Reference {
        parts.push("base".to_owned());
    } else {
        parts.push(format!("↑{} ↓{}", lane.ahead, lane.behind));
    }
    if let Some(remote) = &lane.remote {
        parts.push(format!("= {remote}"));
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
    if let (Some(s), Some(like)) = (lane.score, like) {
        parts.push(format!("{:.0}% in {}", s * 100.0, like.name));
    }
    match lane.before {
        Some(0) => parts.push("forked earlier".to_owned()),
        Some(n) => parts.push(format!("{n} older")),
        None => {}
    }
    parts.join(" · ")
}

/// The map as Unicode: a header, then the grid row by row, folds folded. Commit rows go on with
/// the lane labels (at each lane's newest commit) or the other refs there, and the subject.
pub fn text(doc: &MapDocument) -> String {
    let mut out = String::new();
    let mode = match doc.show.mode {
        Mode::Local => "local first".to_owned(),
        Mode::Recent => "recent".to_owned(),
        Mode::Mine => "mine".to_owned(),
        Mode::Like => format!(
            "like {}",
            doc.show.like.as_ref().map_or("?", |l| l.name.as_str())
        ),
    };
    let shown = doc
        .lanes
        .iter()
        .filter(|l| doc.branches.contains(&l.branch))
        .map(|l| l.branch.as_str())
        .collect::<HashSet<_>>()
        .len();
    out.push_str(&format!(
        "base {} · show {mode} · {shown} of {} branches · {} commits{}\n",
        doc.base.name,
        doc.branches.len(),
        doc.commits.len(),
        if doc.page.more { ", more older" } else { "" },
    ));

    let width = doc.grid.width;
    let mut grid = vec![vec![' '; width]; doc.grid.height];
    for c in &doc.grid.cells {
        grid[c.row][c.col] = glyph(c, doc.lanes[c.lane].kind == LaneKind::Reference);
    }
    let hidden: HashSet<usize> = doc
        .folds
        .iter()
        .flat_map(|f| f.rows[0] + 1..=f.rows[1])
        .collect();
    let folded: HashMap<usize, usize> = doc
        .folds
        .iter()
        .map(|f| (f.rows[0], f.commits.len()))
        .collect();
    let commit_at: HashMap<usize, &CommitOut> = doc.commits.iter().map(|c| (c.row, c)).collect();
    let lane_names: HashSet<&str> = doc
        .lanes
        .iter()
        .flat_map(|l| std::iter::once(l.name.as_str()).chain(l.remote.as_deref()))
        .collect();

    // The label column: lane labels where a lane's label goes, else other refs.
    let labels: HashMap<usize, String> = (0..doc.grid.height)
        .filter_map(|r| {
            let lanes: Vec<String> = doc
                .lanes
                .iter()
                .filter(|l| l.label_row == r)
                .map(|l| format!("{} {}", l.name, state(l, doc.show.like.as_ref())))
                .collect();
            let refs: Vec<&str> = commit_at.get(&r).map_or_else(Vec::new, |c| {
                c.refs
                    .iter()
                    .map(String::as_str)
                    .filter(|n| !lane_names.contains(n))
                    .collect()
            });
            let mut parts = lanes;
            if !refs.is_empty() {
                parts.push(format!("({})", refs.join(", ")));
            }
            (!parts.is_empty()).then(|| (r, parts.join("; ")))
        })
        .collect();
    let label_width = labels
        .values()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);

    for (r, row) in grid.iter().enumerate() {
        if hidden.contains(&r) {
            continue;
        }
        let mut line: String = row.iter().collect();
        if let Some(c) = commit_at.get(&r) {
            let label = labels.get(&r).map_or("", String::as_str);
            let subject = match folded.get(&r) {
                Some(n) => format!("×{n} {}", c.subject),
                None => c.subject.clone(),
            };
            line.push_str(&format!("  {label:<label_width$}  {subject}"));
        } else if let Some(label) = labels.get(&r) {
            line.push_str(&format!("  {label}"));
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests;
