//! Language-independent analyses over the IR and the coverage overlay (DESIGN.md, "Analyses and
//! suggestions"). They use generic IR fields only; `edits` come from language packs later.

mod duplicates;
mod hash;

pub use duplicates::{DuplicateOptions, duplicate_functions};
pub use hash::{Hashes, structural_hashes};

use sundgren_ir::{FileEdit, NodeId};
use sundgren_join::NodeStatus;

/// How far coverage backs a suggestion. Uncovered code is reported, not hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Confidence {
    /// No involved node is reached by a test.
    Low,
    /// Some involved nodes are reached.
    Medium,
    /// Every involved node is reached.
    High,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }
}

/// Coverage of one involved node, from the join overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeEvidence {
    pub node: NodeId,
    /// None when the node's file wasn't joined (no coverage, or a source mismatch).
    pub status: Option<NodeStatus>,
    pub executable: usize,
    pub hit: usize,
    /// Covering test ids; None without per-test coverage.
    pub tests: Option<Vec<String>>,
}

impl NodeEvidence {
    /// Reached by a test: covering tests when known, else any hit line.
    pub fn reached(&self) -> bool {
        match &self.tests {
            Some(t) => !t.is_empty(),
            None => matches!(self.status, Some(NodeStatus::Covered | NodeStatus::Partial)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// One entry per node of the suggestion, in the same order.
    pub nodes: Vec<NodeEvidence>,
}

impl Evidence {
    pub fn confidence(&self) -> Confidence {
        let reached = self.nodes.iter().filter(|n| n.reached()).count();
        if reached == 0 {
            Confidence::Low
        } else if reached == self.nodes.len() {
            Confidence::High
        } else {
            Confidence::Medium
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// Stable: analysis name and involved node ids.
    pub id: String,
    /// e.g. "duplicate-function".
    pub analysis: String,
    pub nodes: Vec<NodeId>,
    pub message: String,
    pub evidence: Evidence,
    pub confidence: Confidence,
    /// A ready patch, when a language pack can produce one (M4).
    pub edits: Option<Vec<FileEdit>>,
}
