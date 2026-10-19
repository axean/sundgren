//! Structural hashes over L1 (DESIGN.md, "Structural hashing (L1)").
//!
//! A node's hash covers its subtree: kind, `l1.label`, local names and the children in order.
//! Spans, names and `lang` are ignored, so comments and formatting never matter. The hashed
//! node's own binding name is left out, so functions that differ only in name hash equal.
//!
//! - `exact`: local names as written.
//! - `renamed`: names bound inside the subtree become `$0`, `$1`, … in order of first use;
//!   names bound outside it (captures) stay as written.

use std::collections::{BTreeMap, HashMap};

use sha2::{Digest, Sha256};
use sundgren_ir::{File, Kind, Node, NodeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hashes {
    pub exact: String,
    pub renamed: String,
    /// Nodes in the subtree, the node itself included.
    pub size: usize,
}

/// Hashes for every node of an L1 file; empty for a file without L1.
pub fn structural_hashes(file: &File) -> BTreeMap<NodeId, Hashes> {
    let mut out = BTreeMap::new();
    if !file.has_l1() {
        return out;
    }
    let nodes = &file.nodes;
    let index: HashMap<&NodeId, usize> =
        nodes.iter().enumerate().map(|(i, n)| (&n.id, i)).collect();
    let ends = subtree_ends(nodes, &index);
    let children = child_counts(nodes, &index);
    for (i, node) in nodes.iter().enumerate() {
        let range = i..ends[i];
        let exact = digest(nodes, &children, range.clone(), &index, false);
        let renamed = digest(nodes, &children, range.clone(), &index, true);
        out.insert(
            node.id.clone(),
            Hashes {
                exact,
                renamed,
                size: range.len(),
            },
        );
    }
    out
}

/// One past the last node of each node's subtree (pre-order makes subtrees contiguous).
fn subtree_ends(nodes: &[Node], index: &HashMap<&NodeId, usize>) -> Vec<usize> {
    let mut ends: Vec<usize> = (1..=nodes.len()).collect();
    for i in (0..nodes.len()).rev() {
        if let Some(p) = nodes[i].parent.as_ref().and_then(|p| index.get(p)) {
            ends[*p] = ends[*p].max(ends[i]);
        }
    }
    ends
}

fn child_counts(nodes: &[Node], index: &HashMap<&NodeId, usize>) -> Vec<usize> {
    let mut counts = vec![0; nodes.len()];
    for n in nodes {
        if let Some(p) = n.parent.as_ref().and_then(|p| index.get(p)) {
            counts[*p] += 1;
        }
    }
    counts
}

/// sha256 over the subtree in pre-order: per node its kind, label, name part and child count.
fn digest(
    nodes: &[Node],
    children: &[usize],
    range: std::ops::Range<usize>,
    index: &HashMap<&NodeId, usize>,
    rename: bool,
) -> String {
    let root = range.start;
    let inside = |id: &NodeId| index.get(id).is_some_and(|i| range.contains(i));
    let mut renames: HashMap<(&NodeId, &str), usize> = HashMap::new();
    let mut h = Sha256::new();
    for i in range.clone() {
        let n = &nodes[i];
        let l1 = n.l1.as_ref();
        let name: String = match l1.and_then(|l| l.local.as_ref()) {
            _ if i == root => String::new(),
            None => String::new(),
            Some(local) if rename && inside(&local.scope) => {
                let next = renames.len();
                let k = *renames
                    .entry((&local.scope, local.name.as_str()))
                    .or_insert(next);
                format!("${k}")
            }
            Some(local) => local.name.clone(),
        };
        field(&mut h, &kind_name(&n.kind));
        field(&mut h, l1.map_or("", |l| l.label.as_str()));
        field(&mut h, &name);
        field(&mut h, &children[i].to_string());
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Length-prefixed, so field boundaries can't shift.
fn field(h: &mut Sha256, s: &str) {
    h.update((s.len() as u64).to_le_bytes());
    h.update(s.as_bytes());
}

fn kind_name(kind: &Kind) -> String {
    match kind {
        Kind::Other(k) => format!("Other:{k}"),
        k => format!("{k:?}"),
    }
}
