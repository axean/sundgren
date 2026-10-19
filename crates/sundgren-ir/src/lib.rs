//! Source IR (see DESIGN.md, "IR"). Plain data: no language knowledge, no parsing.
//!
//! A frontend turns one source file into a [`File`]: a flat, pre-order list of [`Node`]s with
//! exact [`Span`]s. The JSON form is versioned ([`SCHEMA`]) and described by `ir.schema.json`.

mod edit;
mod file;
mod node;
mod span;

pub use edit::{EditError, TextEdit, apply_edits};
pub use file::{File, IrError, SCHEMA, sha256_hex};
pub use node::{Kind, Node, NodeId, NodeIds};
pub use span::{LineIndex, Newlines, Pos, Span};

/// The checked-in JSON schema for [`File`].
pub const JSON_SCHEMA: &str = include_str!("../ir.schema.json");
