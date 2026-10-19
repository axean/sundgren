//! Rust frontend: IR (L0) for one Rust file, parsed with rust-analyzer's lossless parser.
//!
//! Names follow rustc's def paths, relative to the file's module (`inner::f`, `<S>::m`,
//! `<S as T>::r`, `f::{closure#0}`), since that is what coverage tools report. See
//! design/ir-mapping.md.

use std::fmt;

use ra_ap_syntax::ast::{self, HasName};
use ra_ap_syntax::{
    AstNode, AstToken, Edition, NodeOrToken, SourceFile, SyntaxElement, SyntaxKind, SyntaxNode,
    SyntaxToken, WalkEvent,
};
use sundgren_ir::{File, Kind, LineIndex, Newlines, Node, NodeId, NodeIds, SCHEMA, Span};

pub const PRODUCER: &str = "sundgren-frontend-rust/1 ra_ap_syntax 0.0.349";

/// The IR, plus the parser's syntax errors. The IR is built even when there are errors; spans
/// around an error may then not be what rustc would see.
#[derive(Debug)]
pub struct Parsed {
    pub file: File,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    /// Rust source must be UTF-8.
    NotUtf8,
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontendError::NotUtf8 => f.write_str("source is not valid UTF-8"),
        }
    }
}

impl std::error::Error for FrontendError {}

/// Parse with the newest edition.
pub fn parse(path: &str, source: &[u8]) -> Result<Parsed, FrontendError> {
    parse_edition(path, source, Edition::CURRENT)
}

pub fn parse_edition(path: &str, source: &[u8], edition: Edition) -> Result<Parsed, FrontendError> {
    let text = std::str::from_utf8(source).map_err(|_| FrontendError::NotUtf8)?;
    let parse = SourceFile::parse(text, edition);
    let errors = parse
        .errors()
        .iter()
        .map(|e| format!("{:?}: {e}", e.range()))
        .collect();
    let root = parse.tree().syntax().clone();
    let mut b = Builder {
        index: LineIndex::new(source, Newlines::Lf),
        ids: NodeIds::new(path),
        nodes: Vec::new(),
        parents: Vec::new(),
        scopes: Vec::new(),
    };
    b.nodes.push(Node {
        id: NodeId::root(path),
        parent: None,
        kind: Kind::Module,
        span: b.index.span(0, source.len()).expect("in range"),
        body_span: None,
        name: None,
        qualified_name: None,
        lang: serde_json::json!({"syntax": "SOURCE_FILE"}),
    });
    b.parents.push((root.clone(), NodeId::root(path)));
    b.walk(&root);
    Ok(Parsed {
        file: File {
            schema: SCHEMA.into(),
            path: path.into(),
            language: "rust".into(),
            producer: PRODUCER.into(),
            source_sha256: sundgren_ir::sha256_hex(source),
            newlines: Newlines::Lf,
            nodes: b.nodes,
        },
        errors,
    })
}

/// A naming scope: the def path of an item, impl or closure.
struct Scope {
    node: SyntaxNode,
    path: String,
    /// Bodies number their closures (`{closure#N}`); None for scopes that aren't bodies.
    closures: Option<u32>,
}

struct Builder {
    index: LineIndex,
    ids: NodeIds,
    nodes: Vec<Node>,
    /// Enclosing IR nodes, innermost last.
    parents: Vec<(SyntaxNode, NodeId)>,
    scopes: Vec<Scope>,
}

impl Builder {
    fn walk(&mut self, root: &SyntaxNode) {
        for event in root.preorder() {
            match event {
                WalkEvent::Enter(node) => self.enter(node),
                WalkEvent::Leave(node) => {
                    if self.parents.last().is_some_and(|(n, _)| *n == node) {
                        self.parents.pop();
                    }
                    while self.scopes.last().is_some_and(|s| s.node == node) {
                        self.scopes.pop();
                    }
                }
            }
        }
    }

    fn prefix(&self) -> &str {
        self.scopes.last().map_or("", |s| s.path.as_str())
    }

    fn join(&self, name: &str) -> String {
        match self.prefix() {
            "" => name.to_owned(),
            p => format!("{p}::{name}"),
        }
    }

    fn enter(&mut self, node: SyntaxNode) {
        use SyntaxKind as K;
        match node.kind() {
            K::FN => {
                let f = ast::Fn::cast(node.clone()).expect("FN");
                let path = self.join(&name_of(f.name()));
                let body_start = f.body().and_then(|b| block_first_token(&b));
                let head = f.fn_token().map(|t| t.text_range().start());
                self.emit(&node, Kind::Function, &path, body_start, head);
                // An async fn's body is a coroutine: rustc names it `{closure#0}`.
                if f.async_token().is_some() {
                    self.push_scope(&node, path.clone(), Some(0));
                    self.push_scope(&node, format!("{path}::{{closure#0}}"), Some(0));
                } else {
                    self.push_scope(&node, path, Some(0));
                }
            }
            K::CLOSURE_EXPR => {
                let c = ast::ClosureExpr::cast(node.clone()).expect("CLOSURE_EXPR");
                let path = self.next_closure();
                let body_start = c.body().and_then(|e| match e {
                    ast::Expr::BlockExpr(b) if is_plain_block(&b) => block_first_token(&b),
                    e => Some(e.syntax().text_range().start()),
                });
                self.emit(&node, Kind::Closure, &path, body_start, None);
                self.push_scope(&node, path, Some(0));
            }
            K::BLOCK_EXPR => {
                let b = ast::BlockExpr::cast(node.clone()).expect("BLOCK_EXPR");
                // async and gen blocks are coroutines and share the closure counter.
                if b.async_token().is_some() || b.gen_token().is_some() {
                    let path = self.next_closure();
                    self.emit(&node, Kind::Closure, &path, block_first_token(&b), None);
                    self.push_scope(&node, path, Some(0));
                }
            }
            K::IMPL => {
                let i = ast::Impl::cast(node.clone()).expect("IMPL");
                let ty = i
                    .self_ty()
                    .map_or_else(|| "_".into(), |t| tight(t.syntax()));
                // rustc prints the type's own path; for a plain local type that is ours.
                let ty = if is_simple_path(&ty) {
                    self.join(&ty)
                } else {
                    ty
                };
                let path = match i.trait_() {
                    Some(t) => format!("<{ty} as {}>", tight(t.syntax())),
                    None => format!("<{ty}>"),
                };
                self.emit(&node, Kind::Other("IMPL".into()), &path, None, None);
                self.push_scope(&node, path, None);
            }
            K::TRAIT | K::MODULE => {
                let name = match ast::Trait::cast(node.clone()) {
                    Some(t) => name_of(t.name()),
                    None => name_of(ast::Module::cast(node.clone()).expect("MODULE").name()),
                };
                let path = self.join(&name);
                let kind = if node.kind() == K::MODULE {
                    Kind::Module
                } else {
                    Kind::Other("TRAIT".into())
                };
                self.emit(&node, kind, &path, None, None);
                self.push_scope(&node, path, None);
            }
            // Bodies that can hold closures but are not L0 nodes.
            K::CONST | K::STATIC => {
                let name = ast::Const::cast(node.clone())
                    .and_then(|c| c.name())
                    .or_else(|| ast::Static::cast(node.clone()).and_then(|s| s.name()));
                let path = self.join(&name_of(name));
                self.push_scope(&node, path, Some(0));
            }
            _ => {}
        }
    }

    fn next_closure(&mut self) -> String {
        match self.scopes.iter_mut().rev().find(|s| s.closures.is_some()) {
            Some(owner) => {
                let n = owner.closures.as_mut().expect("body");
                let path = format!("{}::{{closure#{n}}}", owner.path);
                *n += 1;
                path
            }
            None => "{closure#0}".into(),
        }
    }

    fn push_scope(&mut self, node: &SyntaxNode, path: String, closures: Option<u32>) {
        self.scopes.push(Scope {
            node: node.clone(),
            path,
            closures,
        });
    }

    /// Add an IR node. `body_start` is where the body's first token is; `head` is the token
    /// whose line makes a one-liner (default: the node's first token).
    fn emit(
        &mut self,
        node: &SyntaxNode,
        kind: Kind,
        path: &str,
        body_start: Option<ra_ap_syntax::TextSize>,
        head: Option<ra_ap_syntax::TextSize>,
    ) {
        let start = span_start(node);
        let end = usize::from(node.text_range().end());
        let span = self.span(start, end);
        let body_span = match (kind.is_callable(), body_start) {
            (true, Some(b)) => {
                let head_line = self.span(head.map_or(start, usize::from), end).start.line;
                let body = self.span(usize::from(b), end);
                (body.start.line != head_line).then_some(body)
            }
            _ => None,
        };
        let name = path.rsplit("::").next().unwrap_or(path).to_owned();
        let id = self.ids.named(path);
        let parent = self.parents.last().map(|(_, id)| id.clone());
        self.nodes.push(Node {
            id: id.clone(),
            parent,
            kind,
            span,
            body_span,
            name: Some(name),
            qualified_name: Some(path.to_owned()),
            lang: serde_json::json!({"syntax": format!("{:?}", node.kind())}),
        });
        self.parents.push((node.clone(), id));
    }

    fn span(&self, start: usize, end: usize) -> Span {
        self.index
            .span(start, end)
            .expect("syntax ranges are in the source")
    }
}

fn name_of(name: Option<ast::Name>) -> String {
    name.map_or_else(|| "_".into(), |n| n.text().to_string())
}

/// The first token that is not whitespace or a plain comment. ra attaches comments right
/// above an item to it; doc comments are part of the item, plain ones are not.
fn span_start(node: &SyntaxNode) -> usize {
    let first = node.children_with_tokens().find(|e| !is_plain_trivia(e));
    let start = first.map_or(node.text_range().start(), |e| e.text_range().start());
    usize::from(start)
}

fn is_plain_trivia(e: &SyntaxElement) -> bool {
    match e {
        NodeOrToken::Token(t) => is_trivia_token(t),
        NodeOrToken::Node(_) => false,
    }
}

fn is_trivia_token(t: &SyntaxToken) -> bool {
    match t.kind() {
        SyntaxKind::WHITESPACE => true,
        SyntaxKind::COMMENT => !ast::Comment::cast(t.clone()).is_some_and(|c| c.is_doc()),
        _ => false,
    }
}

/// A block with no `async`, `gen`, `const`, `unsafe`, `try` or label.
fn is_plain_block(b: &ast::BlockExpr) -> bool {
    b.async_token().is_none()
        && b.gen_token().is_none()
        && b.const_token().is_none()
        && b.unsafe_token().is_none()
        && b.try_block_modifier().is_none()
        && b.label().is_none()
}

/// The first token after `{` that is not whitespace or a comment (the `}` of an empty block).
fn block_first_token(b: &ast::BlockExpr) -> Option<ra_ap_syntax::TextSize> {
    let mut tok = b.stmt_list()?.l_curly_token()?.next_token();
    while let Some(t) = tok {
        if !t.kind().is_trivia() {
            return Some(t.text_range().start());
        }
        tok = t.next_token();
    }
    None
}

/// Source text without comments, whitespace runs collapsed to one space.
fn tight(node: &SyntaxNode) -> String {
    let mut out = String::new();
    let mut gap = false;
    for e in node.descendants_with_tokens() {
        if let NodeOrToken::Token(t) = e {
            if t.kind().is_trivia() {
                gap = !out.is_empty();
            } else {
                if gap {
                    out.push(' ');
                }
                gap = false;
                out.push_str(t.text());
            }
        }
    }
    out
}

/// `S` or `S<T>`: one path segment.
fn is_simple_path(ty: &str) -> bool {
    let head = ty.split('<').next().unwrap_or(ty);
    !head.is_empty() && head.chars().all(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests;
