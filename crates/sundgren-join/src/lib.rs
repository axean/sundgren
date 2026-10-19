//! Join coverage onto the IR (see DESIGN.md, "Join: coverage onto IR").
//!
//! The result is an overlay, `NodeId -> CoverageFact`; the IR is never modified, so one IR can
//! be joined against many runs. The key is `(file, line)`. lcov function names are only used to
//! attach `FN`/`FNDA` records to function nodes: by qualified name, then by line overlap.

use std::collections::{BTreeMap, BTreeSet};

use sundgren_core::linemap::identity;
use sundgren_core::model::{Count, FileCoverage, LineNo, ReportWarning, Snapshot};
use sundgren_core::rollup::match_spans;
use sundgren_core::symbols::{LcovFnSymbolizer, Span};
use sundgren_ir::{File, Node, NodeId};

/// What the coverage input can resolve. Facts never claim more than this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Granularity {
    /// Whole lines (lcov): two nodes on one line can't be told apart.
    Line,
    /// Line and column spans (e.g. llvm-cov JSON).
    Region,
    /// Line-to-line arcs (coverage.py JSON).
    Arc,
}

impl Granularity {
    pub fn as_str(self) -> &'static str {
        match self {
            Granularity::Line => "line",
            Granularity::Region => "region",
            Granularity::Arc => "arc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeStatus {
    /// Every executable line hit.
    Covered,
    /// Some executable lines hit.
    Partial,
    /// No executable line hit.
    Uncovered,
    /// No executable lines known (or the file has no coverage).
    Unmeasured,
    /// A line's hit/miss differs between repeats.
    Flaky,
}

impl NodeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeStatus::Covered => "covered",
            NodeStatus::Partial => "partial",
            NodeStatus::Uncovered => "uncovered",
            NodeStatus::Unmeasured => "unmeasured",
            NodeStatus::Flaky => "flaky",
        }
    }
}

/// One BRDA record, attached to the innermost node containing its line (no L2 yet).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BranchFact {
    pub line: LineNo,
    pub block: String,
    pub branch: String,
    /// None = '-' (block never reached).
    pub taken: Option<Count>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchedBy {
    Name,
    Overlap,
}

/// The lcov function record matched to a function node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionMatch {
    /// The name as the coverage tool wrote it.
    pub name: String,
    /// FNDA count; None when the record has none.
    pub hits: Option<Count>,
    pub by: MatchedBy,
}

/// Coverage of one node. Line sets cover the node's counted lines: the body for functions and
/// closures (whole span for a one-liner), the whole span otherwise. Nested nodes' lines are
/// included, as in the prototype's function status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageFact {
    pub granularity: Granularity,
    /// Executable lines, from run 0.
    pub executable: BTreeSet<LineNo>,
    pub hit: BTreeSet<LineNo>,
    pub missed: BTreeSet<LineNo>,
    /// Lines whose hit/miss differs between repeats; also in `executable`.
    pub flaky: BTreeSet<LineNo>,
    /// Executable lines that also lie in a node that is neither an ancestor nor a descendant
    /// (two nodes on one line). At line granularity their counts can't be split.
    pub shared: BTreeSet<LineNo>,
    pub branches: Vec<BranchFact>,
    /// Ids of the tests that hit the node; None without per-test coverage.
    pub tests: Option<Vec<String>>,
    pub function: Option<FunctionMatch>,
    pub status: NodeStatus,
}

/// Coverage for one IR file: the runs (run 0 is canonical, the rest repeats) and the sha256 of
/// the source they were measured on.
#[derive(Debug, Clone, Copy)]
pub struct Measured<'a> {
    pub runs: &'a [Snapshot],
    pub source_sha256: &'a str,
    pub granularity: Granularity,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overlay {
    pub facts: BTreeMap<NodeId, CoverageFact>,
    pub warnings: Vec<ReportWarning>,
}

/// Join every IR file against its coverage. A file whose measured source differs from the IR's
/// is skipped with an `ir_source_mismatch` warning.
pub fn join<'a>(files: impl IntoIterator<Item = (&'a File, Measured<'a>)>) -> Overlay {
    let mut out = Overlay::default();
    for (ir, measured) in files {
        let o = join_file(ir, measured);
        out.facts.extend(o.facts);
        out.warnings.extend(o.warnings);
    }
    out.warnings.sort();
    out.warnings.dedup();
    out
}

/// Join one IR file. Refuses (no facts, one warning) if the sha256s differ.
pub fn join_file(ir: &File, measured: Measured<'_>) -> Overlay {
    if ir.source_sha256 != measured.source_sha256 {
        let message = format!(
            "{}: coverage was measured on other source than the IR was built from; not joined",
            ir.path
        );
        return Overlay {
            facts: BTreeMap::new(),
            warnings: vec![ReportWarning {
                code: "ir_source_mismatch".into(),
                message: message.into(),
            }],
        };
    }
    let fcs: Vec<Option<&FileCoverage>> = measured
        .runs
        .iter()
        .map(|s| s.files.get(&ir.path).filter(|fc| !fc.external))
        .collect();
    let fc = fcs.first().copied().flatten();
    let lines: BTreeMap<LineNo, Count> = fc.map(|f| f.lines.clone()).unwrap_or_default();
    let flaky = unstable_lines(&fcs);

    let counted: Vec<BTreeSet<LineNo>> = ir.nodes.iter().map(counted_lines).collect();
    let spans: Vec<BTreeSet<LineNo>> = ir.nodes.iter().map(|n| lines_of(&n.span)).collect();
    let related = relations(ir);
    let owners = branch_owners(&spans, fc);
    let functions = match_functions(ir, fc);

    let mut facts = BTreeMap::new();
    for (i, node) in ir.nodes.iter().enumerate() {
        let executable: BTreeSet<LineNo> = counted[i]
            .iter()
            .copied()
            .filter(|l| lines.contains_key(l))
            .collect();
        let hit: BTreeSet<LineNo> = executable
            .iter()
            .copied()
            .filter(|l| lines[l] > 0)
            .collect();
        let missed = executable.difference(&hit).copied().collect();
        let node_flaky: BTreeSet<LineNo> = executable.intersection(&flaky).copied().collect();
        let shared = executable
            .iter()
            .copied()
            .filter(|l| {
                (0..ir.nodes.len())
                    .any(|j| j != i && !related[i].contains(&j) && spans[j].contains(l))
            })
            .collect();
        let status = if !node_flaky.is_empty() {
            NodeStatus::Flaky
        } else if executable.is_empty() {
            NodeStatus::Unmeasured
        } else if hit.len() == executable.len() {
            NodeStatus::Covered
        } else if hit.is_empty() {
            NodeStatus::Uncovered
        } else {
            NodeStatus::Partial
        };
        facts.insert(
            node.id.clone(),
            CoverageFact {
                granularity: measured.granularity,
                executable,
                hit,
                missed,
                flaky: node_flaky,
                shared,
                branches: owners.get(&i).cloned().unwrap_or_default(),
                tests: None,
                function: functions.get(&i).cloned(),
                status,
            },
        );
    }
    Overlay {
        facts,
        warnings: Vec::new(),
    }
}

fn line(l: u32) -> LineNo {
    LineNo::from(l)
}

/// Lines a span covers. The root may end at column 1 of the line after the file's last
/// newline; that line is not covered.
fn lines_of(span: &sundgren_ir::Span) -> BTreeSet<LineNo> {
    let mut end = span.end.line;
    if span.end.col == 1 && span.end.byte > span.start.byte && end > span.start.line {
        end -= 1;
    }
    (line(span.start.line)..=line(end)).collect()
}

/// Body lines for callables (the prototype's rule), the whole span otherwise.
fn counted_lines(node: &Node) -> BTreeSet<LineNo> {
    match &node.body_span {
        Some(body) if node.kind.is_callable() => {
            (line(body.start.line)..=*lines_of(&node.span).last().expect("non-empty")).collect()
        }
        _ => lines_of(&node.span),
    }
}

/// Lines hit in some runs but not all; a run without the file hits nothing.
fn unstable_lines(fcs: &[Option<&FileCoverage>]) -> BTreeSet<LineNo> {
    if fcs.len() < 2 {
        return BTreeSet::new();
    }
    let sets: Vec<BTreeSet<LineNo>> = fcs
        .iter()
        .map(|fc| {
            fc.map(|f| {
                f.lines
                    .iter()
                    .filter(|(_, c)| **c > 0)
                    .map(|(l, _)| *l)
                    .collect()
            })
            .unwrap_or_default()
        })
        .collect();
    let union: BTreeSet<LineNo> = sets.iter().flatten().copied().collect();
    union
        .into_iter()
        .filter(|l| !sets.iter().all(|s| s.contains(l)))
        .collect()
}

/// For each node, the indices of its ancestors and descendants.
fn relations(ir: &File) -> Vec<BTreeSet<usize>> {
    let index: BTreeMap<&NodeId, usize> = ir
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (&n.id, i))
        .collect();
    let mut related = vec![BTreeSet::new(); ir.nodes.len()];
    for (i, n) in ir.nodes.iter().enumerate() {
        let mut p = n.parent.as_ref().and_then(|p| index.get(p).copied());
        while let Some(a) = p {
            related[i].insert(a);
            related[a].insert(i);
            p = ir.nodes[a]
                .parent
                .as_ref()
                .and_then(|p| index.get(p).copied());
        }
    }
    related
}

/// Each branch goes to the innermost node (latest in pre-order) whose span covers its line.
fn branch_owners(
    spans: &[BTreeSet<LineNo>],
    fc: Option<&FileCoverage>,
) -> BTreeMap<usize, Vec<BranchFact>> {
    let mut out: BTreeMap<usize, Vec<BranchFact>> = BTreeMap::new();
    let Some(fc) = fc else {
        return out;
    };
    for (k, taken) in &fc.branches {
        let owner = (0..spans.len()).rev().find(|&i| spans[i].contains(&k.line));
        if let Some(i) = owner {
            out.entry(i).or_default().push(BranchFact {
                line: k.line,
                block: k.block.clone(),
                branch: k.branch.clone(),
                taken: *taken,
            });
        }
    }
    out
}

/// Match lcov FN records to callable nodes: qualified name first, then largest line overlap.
fn match_functions(ir: &File, fc: Option<&FileCoverage>) -> BTreeMap<usize, FunctionMatch> {
    let mut out = BTreeMap::new();
    let Some(fc) = fc.filter(|f| !f.functions.is_empty()) else {
        return out;
    };
    let mut nodes = Vec::new();
    let mut node_spans = Vec::new();
    for (i, n) in ir.nodes.iter().enumerate() {
        if let (true, Some(q)) = (n.kind.is_callable(), &n.qualified_name) {
            let lines = lines_of(&n.span);
            node_spans.push(Span {
                name: q.clone(),
                start: *lines.first().expect("non-empty"),
                end: *lines.last().expect("non-empty"),
                body_start: n.body_span.map(|b| line(b.start.line)),
            });
            nodes.push(i);
        }
    }
    let lcov_spans = LcovFnSymbolizer.spans(fc);
    let lm = identity(ir.path.as_str().into());
    for (sl, sn) in match_spans(&lcov_spans, &node_spans, &lm) {
        let (Some(sl), Some(sn)) = (sl, sn) else {
            continue;
        };
        let i = node_spans
            .iter()
            .position(|s| std::ptr::eq(s, sn))
            .expect("span from node_spans");
        let hits = fc
            .functions
            .iter()
            .find(|f| f.name == sl.name)
            .and_then(|f| f.hits);
        let by = if sl.name == sn.name {
            MatchedBy::Name
        } else {
            MatchedBy::Overlap
        };
        out.insert(
            nodes[i],
            FunctionMatch {
                name: sl.name.clone(),
                hits,
                by,
            },
        );
    }
    out
}
