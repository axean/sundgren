//! One file's IR, its JSON form and validation against the source.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::node::{Node, NodeId, NodeIds};
use crate::span::{LineIndex, Newlines, Span};

/// Schema id of the JSON form. Bump on any incompatible change.
pub const SCHEMA: &str = "sundgren.ir/1";

/// The IR of one source file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    /// Always [`SCHEMA`].
    pub schema: String,
    /// Repo-relative, `/`-separated.
    pub path: String,
    /// e.g. `python`, `rust`.
    pub language: String,
    /// Frontend and parser that produced this, e.g. `sundgren-python-ast/1 CPython 3.12.13`.
    pub producer: String,
    /// sha256 of the exact bytes the spans refer to, lowercase hex.
    pub source_sha256: String,
    /// How the frontend counts lines.
    pub newlines: Newlines,
    /// Pre-order: the root (the whole file) first, every parent before its children, siblings
    /// in source order.
    pub nodes: Vec<Node>,
    /// Layers present besides L0, e.g. `["L1"]`. Empty: L0 only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub levels: Vec<String>,
}

#[derive(Debug)]
pub enum IrError {
    Json(serde_json::Error),
    Schema(String),
    Invalid(Vec<String>),
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrError::Json(e) => write!(f, "IR JSON: {e}"),
            IrError::Schema(s) => write!(f, "IR schema {s:?}, expected {SCHEMA:?}"),
            IrError::Invalid(errs) => write!(f, "invalid IR: {}", errs.join("; ")),
        }
    }
}

impl std::error::Error for IrError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl File {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("IR serializes")
    }

    /// Parse the JSON form. Checks the schema id, not the content (see [`File::validate`]).
    pub fn from_json(json: &str) -> Result<File, IrError> {
        #[derive(Deserialize)]
        struct Head {
            schema: Option<String>,
        }
        let head: Head = serde_json::from_str(json).map_err(IrError::Json)?;
        match head.schema.as_deref() {
            Some(SCHEMA) => serde_json::from_str(json).map_err(IrError::Json),
            other => Err(IrError::Schema(other.unwrap_or("").to_owned())),
        }
    }

    /// True when the frontend emitted L1 (statement and expression nodes with `l1` payloads).
    pub fn has_l1(&self) -> bool {
        self.levels.iter().any(|l| l == "L1")
    }

    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    /// Check the IR against the bytes it was built from: hash, ids, tree order, and that every
    /// span's line/column agrees with its byte offsets.
    pub fn validate(&self, source: &[u8]) -> Result<(), IrError> {
        let mut errs = Vec::new();
        if self.schema != SCHEMA {
            errs.push(format!("schema {:?}", self.schema));
        }
        if self.source_sha256 != sha256_hex(source) {
            errs.push("source_sha256 does not match the source".into());
        }
        let index = LineIndex::new(source, self.newlines);
        let mut ids = NodeIds::new(&self.path);
        let mut seen: HashSet<&NodeId> = HashSet::new();
        let mut children: HashMap<&NodeId, usize> = HashMap::new();
        let l1 = self.has_l1();
        // Ancestors of the current node, innermost last.
        let mut stack: Vec<&Node> = Vec::new();
        for (i, n) in self.nodes.iter().enumerate() {
            let at = format!("node {i} ({})", n.id);
            if !seen.insert(&n.id) {
                errs.push(format!("{at}: duplicate id"));
            }
            let k = n.parent.as_ref().map(|p| {
                let c = children.entry(p).or_insert(0);
                *c += 1;
                *c - 1
            });
            if l1 && n.l1.is_none() {
                errs.push(format!("{at}: no l1 payload in an L1 file"));
            }
            match (&n.parent, &n.qualified_name) {
                (None, _) if i > 0 => errs.push(format!("{at}: only the first node is a root")),
                (None, _) => {
                    if n.id != NodeId::root(&self.path) {
                        errs.push(format!("{at}: root id must be the path"));
                    }
                    if n.span.bytes() != (0..source.len()) {
                        errs.push(format!("{at}: root span must cover the file"));
                    }
                }
                (Some(_), Some(q)) => {
                    if n.id != ids.named(q) {
                        errs.push(format!("{at}: id does not follow the naming rule"));
                    }
                }
                (Some(p), None) if l1 && n.l1.is_some() => {
                    if n.id != NodeId::child(p, k.expect("has a parent")) {
                        errs.push(format!("{at}: id does not follow the structural rule"));
                    }
                }
                (Some(_), None) => errs.push(format!("{at}: non-root node without a name")),
            }
            if let Some(p) = &n.parent {
                while stack.last().is_some_and(|top| &top.id != p) {
                    stack.pop();
                }
                match stack.last() {
                    None => errs.push(format!("{at}: parent {p} is not an earlier ancestor")),
                    Some(parent) if !parent.span.contains(&n.span) => {
                        errs.push(format!("{at}: span outside its parent"));
                    }
                    Some(_) => {}
                }
            }
            stack.push(n);
            // The root covers the whole file, trailing newline included.
            check_span(&index, &n.span, &at, "span", n.parent.is_some(), &mut errs);
            if let Some(b) = &n.body_span {
                check_span(&index, b, &at, "body_span", true, &mut errs);
                if !n.kind.is_callable() {
                    errs.push(format!("{at}: body_span on a {:?}", n.kind));
                }
                if !n.span.contains(b) {
                    errs.push(format!("{at}: body_span outside span"));
                }
            }
        }
        if self.nodes.is_empty() {
            errs.push("no root node".into());
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(IrError::Invalid(errs))
        }
    }
}

fn check_span(
    index: &LineIndex,
    span: &Span,
    at: &str,
    what: &str,
    no_trailing_newline: bool,
    errs: &mut Vec<String>,
) {
    if index.span(span.start.byte, span.end.byte) != Some(*span) {
        errs.push(format!("{at}: {what} line/col disagree with its bytes"));
    } else if no_trailing_newline && span.end.byte > span.start.byte && span.end.col == 1 {
        errs.push(format!("{at}: {what} ends with a newline"));
    }
}
