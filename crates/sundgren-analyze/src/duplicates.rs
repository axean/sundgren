//! First analysis: duplicate functions (DESIGN.md, "First analysis: duplicate functions").

use std::collections::{BTreeMap, BTreeSet, HashMap};

use sundgren_ir::{File, Kind, NodeId};
use sundgren_join::{NodeStatus, Overlay};

use crate::hash::{Hashes, structural_hashes};
use crate::{Evidence, NodeEvidence, Suggestion};

pub const ANALYSIS: &str = "duplicate-function";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateOptions {
    /// Functions with fewer nodes under them than this are trivial and ignored.
    pub min_nodes: usize,
}

impl Default for DuplicateOptions {
    fn default() -> Self {
        Self { min_nodes: 10 }
    }
}

struct Candidate<'a> {
    id: &'a NodeId,
    hashes: Hashes,
    /// Ancestor ids, innermost first.
    ancestors: Vec<&'a NodeId>,
}

/// Groups of functions with equal `renamed` hash (so equal `exact` hash too) across all L1
/// files. `nodes[0]` of each suggestion is the copy to keep: the one covered by the most tests.
/// A group nested entirely inside the copies of another group is not reported again.
pub fn duplicate_functions(
    files: &[File],
    overlay: &Overlay,
    opts: DuplicateOptions,
) -> Vec<Suggestion> {
    let mut groups: BTreeMap<String, Vec<Candidate<'_>>> = BTreeMap::new();
    for file in files.iter().filter(|f| f.has_l1()) {
        let mut hashes = structural_hashes(file);
        let parents: HashMap<&NodeId, &NodeId> = file
            .nodes
            .iter()
            .filter_map(|n| n.parent.as_ref().map(|p| (&n.id, p)))
            .collect();
        for n in file.nodes.iter().filter(|n| n.kind == Kind::Function) {
            let h = hashes.remove(&n.id).expect("hash per node");
            if h.size - 1 < opts.min_nodes {
                continue;
            }
            let mut ancestors = Vec::new();
            let mut p = parents.get(&n.id);
            while let Some(a) = p {
                ancestors.push(*a);
                p = parents.get(*a);
            }
            groups
                .entry(h.renamed.clone())
                .or_default()
                .push(Candidate {
                    id: &n.id,
                    hashes: h,
                    ancestors,
                });
        }
    }
    groups.retain(|_, g| g.len() >= 2);

    // Which group each candidate is in, to drop groups nested in another group's copies.
    let group_of: HashMap<&NodeId, &str> = groups
        .iter()
        .flat_map(|(k, g)| g.iter().map(move |c| (c.id, k.as_str())))
        .collect();
    let mut out = Vec::new();
    for (key, group) in &groups {
        let nested = group.iter().all(|c| {
            c.ancestors
                .iter()
                .any(|a| group_of.get(a).is_some_and(|k| k != key))
        });
        if !nested {
            out.push(suggest(group, overlay));
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn evidence(id: &NodeId, overlay: &Overlay) -> NodeEvidence {
    let fact = overlay.facts.get(id);
    NodeEvidence {
        node: id.clone(),
        status: fact.map(|f| f.status),
        executable: fact.map_or(0, |f| f.executable.len()),
        hit: fact.map_or(0, |f| f.hit.len()),
        tests: fact.and_then(|f| f.tests.clone()),
    }
}

/// Preference for the kept copy: covering tests, then status, then hit lines.
fn rank(e: &NodeEvidence) -> (usize, u8, usize) {
    let status = match e.status {
        Some(NodeStatus::Covered) => 4,
        Some(NodeStatus::Partial) => 3,
        Some(NodeStatus::Flaky) => 2,
        Some(NodeStatus::Uncovered) => 1,
        Some(NodeStatus::Unmeasured) | None => 0,
    };
    (e.tests.as_ref().map_or(0, Vec::len), status, e.hit)
}

fn suggest(group: &[Candidate<'_>], overlay: &Overlay) -> Suggestion {
    let ids: BTreeSet<&NodeId> = group.iter().map(|c| c.id).collect();
    let mut evs: Vec<NodeEvidence> = ids.iter().map(|id| evidence(id, overlay)).collect();
    // Earliest id wins ties.
    let keep = (1..evs.len()).fold(0, |best, i| {
        if rank(&evs[i]) > rank(&evs[best]) {
            i
        } else {
            best
        }
    });
    let kept = evs.remove(keep);
    evs.insert(0, kept);
    let nodes: Vec<NodeId> = evs.iter().map(|e| e.node.clone()).collect();

    let exact: BTreeSet<&str> = group.iter().map(|c| c.hashes.exact.as_str()).collect();
    let same = if exact.len() == 1 {
        "identical"
    } else {
        "identical up to local names"
    };
    let others: Vec<String> = nodes[1..].iter().map(|n| n.to_string()).collect();
    let mut message = format!(
        "{} functions are {same}. Keep {} and replace {} with calls or re-exports.",
        nodes.len(),
        nodes[0],
        others.join(", ")
    );
    let evidence = Evidence { nodes: evs };
    if !evidence.nodes.iter().any(NodeEvidence::reached) {
        message.push_str(" No copy is covered by any test.");
    }
    let confidence = evidence.confidence();
    let id_list: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
    Suggestion {
        id: format!("{ANALYSIS}:{}", id_list.join(",")),
        analysis: ANALYSIS.into(),
        nodes,
        message,
        evidence,
        confidence,
        edits: None,
    }
}
