//! Nodes, kinds and stable node ids.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::span::Span;

/// Shared node kinds. Constructs without a shared kind use `Other` with the native kind name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Module,
    Class,
    Function,
    Closure,
    Block,
    If,
    Loop,
    Match,
    Arm,
    Return,
    Call,
    Assign,
    Expr,
    Other(String),
}

impl Kind {
    /// The names of the shared kinds, in declaration order (`Other` excluded).
    pub const SHARED: [&'static str; 13] = [
        "Module", "Class", "Function", "Closure", "Block", "If", "Loop", "Match", "Arm", "Return",
        "Call", "Assign", "Expr",
    ];

    pub fn is_callable(&self) -> bool {
        matches!(self, Kind::Function | Kind::Closure)
    }
}

/// A node id, stable across runs on identical source.
///
/// - The file's root node: the repo-relative path, e.g. `pkg/mod.py`.
/// - A named node: `<path>::<qualified_name>`, with `#<k>` appended for the k-th (k >= 1)
///   repeat of the same qualified name in the file, in node order (e.g. a redefinition).
///
/// Ids are opaque: compare them, don't parse them.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        NodeId(id.into())
    }

    pub fn root(path: &str) -> Self {
        NodeId(path.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Derives the ids of one file's named nodes. Call [`NodeIds::named`] in node order.
#[derive(Clone, Debug)]
pub struct NodeIds {
    path: String,
    seen: HashMap<String, u32>,
}

impl NodeIds {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            seen: HashMap::new(),
        }
    }

    pub fn named(&mut self, qualified_name: &str) -> NodeId {
        let k = self.seen.entry(qualified_name.to_owned()).or_insert(0);
        let id = if *k == 0 {
            format!("{}::{}", self.path, qualified_name)
        } else {
            format!("{}::{}#{}", self.path, qualified_name, k)
        };
        *k += 1;
        NodeId(id)
    }
}

/// One IR node. Children follow their parent in [`crate::File::nodes`] (pre-order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: NodeId,
    /// None only for the file's root node.
    pub parent: Option<NodeId>,
    pub kind: Kind,
    pub span: Span,
    /// Functions and closures only: from the first statement that counts as the body to the
    /// end. None for a one-liner, whose whole span counts (the prototype's rule).
    pub body_span: Option<Span>,
    pub name: Option<String>,
    /// Python: matches `__qualname__`.
    pub qualified_name: Option<String>,
    /// Language-specific payload, opaque to generic code. Keep it shallow: serde_json's
    /// default recursion limit is 128.
    #[serde(default)]
    pub lang: serde_json::Value,
    /// L1 payload; set on every node of a file whose `levels` include "L1".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1: Option<L1>,
}

/// What structural hashing needs from a node, beyond its kind and children.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct L1 {
    /// The node's normalized content that is not in a child node: native kind plus significant
    /// tokens (`BinOp:Add`, `LITERAL:1`). Never comments, whitespace or the node's own binding
    /// name. Equal labels mean equal constructs.
    pub label: String,
    /// Set when the node binds or uses a local binding.
    #[serde(default)]
    pub local: Option<Local>,
}

/// A local binding: a name bound inside a callable (parameter, local variable, nested def).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Local {
    /// The Function or Closure node that binds the name.
    pub scope: NodeId,
    pub name: String,
}

impl NodeId {
    /// Id of an unnamed (L1) node: `<parent id>/<k>`, k the 0-based index among the parent's
    /// children in node order.
    pub fn child(parent: &NodeId, k: usize) -> Self {
        NodeId(format!("{}/{}", parent.0, k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_get_a_counter() {
        let mut ids = NodeIds::new("a.py");
        assert_eq!(ids.named("C.x").as_str(), "a.py::C.x");
        assert_eq!(ids.named("f").as_str(), "a.py::f");
        assert_eq!(ids.named("C.x").as_str(), "a.py::C.x#1");
        assert_eq!(ids.named("C.x").as_str(), "a.py::C.x#2");
        assert_eq!(NodeId::root("a.py").as_str(), "a.py");
    }

    #[test]
    fn kind_json() {
        let kinds = [Kind::Function, Kind::Other("impl".into())];
        let json = serde_json::to_string(&kinds).unwrap();
        assert_eq!(json, r#"["Function",{"Other":"impl"}]"#);
        assert_eq!(serde_json::from_str::<Vec<Kind>>(&json).unwrap(), kinds);
    }
}
