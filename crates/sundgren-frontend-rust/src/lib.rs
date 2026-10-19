//! Rust frontend: IR (L0, optionally L1) for one Rust file, parsed with rust-analyzer's lossless
//! parser.
//!
//! Names follow rustc's def paths, relative to the file's module (`inner::f`, `<S>::m`,
//! `<S as T>::r`, `f::{closure#0}`), since that is what coverage tools report. See
//! design/ir-mapping.md.

use std::collections::{HashMap, HashSet};
use std::fmt;

use ra_ap_syntax::ast::{self, HasName};
use ra_ap_syntax::{
    AstNode, AstToken, Edition, NodeOrToken, SourceFile, SyntaxElement, SyntaxKind, SyntaxNode,
    SyntaxToken, WalkEvent,
};
use sundgren_ir::{
    File, Kind, L1, LineIndex, Local, Newlines, Node, NodeId, NodeIds, SCHEMA, Span,
};

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

/// What to parse with and what to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub edition: Edition,
    /// Also emit L1: statement and expression nodes with `l1` payloads.
    pub l1: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            edition: Edition::CURRENT,
            l1: false,
        }
    }
}

/// L0 with the newest edition.
pub fn parse(path: &str, source: &[u8]) -> Result<Parsed, FrontendError> {
    parse_with(path, source, Options::default())
}

pub fn parse_edition(path: &str, source: &[u8], edition: Edition) -> Result<Parsed, FrontendError> {
    parse_with(path, source, Options { edition, l1: false })
}

pub fn parse_with(path: &str, source: &[u8], opts: Options) -> Result<Parsed, FrontendError> {
    let edition = opts.edition;
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
        l1: opts.l1,
        bindings: if opts.l1 {
            bindings(&root)
        } else {
            HashMap::new()
        },
        callables: HashMap::new(),
        skip: None,
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
        l1: opts.l1.then(|| L1 {
            label: label(&root, false, false),
            local: None,
        }),
    });
    b.parents.push((root.clone(), NodeId::root(path), 0));
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
            levels: if opts.l1 {
                vec!["L1".into()]
            } else {
                Vec::new()
            },
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
    /// Enclosing IR nodes with their child counts, innermost last.
    parents: Vec<(SyntaxNode, NodeId, usize)>,
    scopes: Vec<Scope>,
    l1: bool,
    /// Names bound inside each callable (params, `let` and other patterns).
    bindings: HashMap<SyntaxNode, HashSet<String>>,
    /// Ids of the callables emitted so far.
    callables: HashMap<SyntaxNode, NodeId>,
    /// A token tree or macro call emitted as one node: its descendants are skipped.
    skip: Option<SyntaxNode>,
}

impl Builder {
    fn walk(&mut self, root: &SyntaxNode) {
        for event in root.preorder() {
            match event {
                WalkEvent::Enter(node) if self.skip.is_none() && node != *root => self.enter(node),
                WalkEvent::Enter(_) => {}
                WalkEvent::Leave(node) => {
                    if self.skip.as_ref() == Some(&node) {
                        self.skip = None;
                    }
                    if self.parents.last().is_some_and(|(n, _, _)| *n == node) {
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
                } else {
                    self.emit_l1(&node);
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
                self.emit_l1(&node);
                self.push_scope(&node, path, Some(0));
            }
            // Folded into the parent's label or `local`.
            K::NAME => {}
            _ => self.emit_l1(&node),
        }
    }

    /// Add an unnamed L1 node (when L1 is on).
    fn emit_l1(&mut self, node: &SyntaxNode) {
        // Nodes of only whitespace and comments (error recovery) have no place in L1.
        if !self.l1 || last_significant(node).is_none() {
            return;
        }
        use SyntaxKind as K;
        let kind = match node.kind() {
            K::IF_EXPR => Kind::If,
            K::LOOP_EXPR | K::WHILE_EXPR | K::FOR_EXPR => Kind::Loop,
            K::MATCH_EXPR => Kind::Match,
            K::MATCH_ARM => Kind::Arm,
            K::RETURN_EXPR => Kind::Return,
            K::CALL_EXPR | K::METHOD_CALL_EXPR => Kind::Call,
            K::LET_STMT => Kind::Assign,
            K::BIN_EXPR if is_assignment(node) => Kind::Assign,
            K::BLOCK_EXPR => Kind::Block,
            K::EXPR_STMT => Kind::Expr,
            k => Kind::Other(format!("{k:?}")),
        };
        if matches!(node.kind(), K::MACRO_CALL | K::TOKEN_TREE) {
            self.skip = Some(node.clone());
        }
        let local = self.local(node);
        let (parent, k) = self.next_child();
        let start = span_start(node);
        let span = self.span(start, span_end(node, start));
        let id = NodeId::child(&parent, k);
        self.nodes.push(Node {
            id: id.clone(),
            parent: Some(parent),
            kind,
            span,
            body_span: None,
            name: None,
            qualified_name: None,
            lang: serde_json::json!({"syntax": format!("{:?}", node.kind())}),
            l1: Some(L1 {
                label: label(node, local.is_some(), false),
                local,
            }),
        });
        self.parents.push((node.clone(), id, 0));
    }

    /// The parent id and this child's index among its children.
    fn next_child(&mut self) -> (NodeId, usize) {
        let (_, id, count) = self.parents.last_mut().expect("the root is always there");
        *count += 1;
        (id.clone(), *count - 1)
    }

    /// The local binding a pattern binds or a name use refers to: the innermost callable
    /// that binds the name. Lexical and approximate; shadowing within one callable doesn't
    /// matter since `Local` names only the callable.
    fn local(&self, node: &SyntaxNode) -> Option<Local> {
        let name = match node.kind() {
            SyntaxKind::IDENT_PAT => binding_name(node)?,
            SyntaxKind::NAME_REF if is_local_use(node) => node.text().to_string(),
            _ => return None,
        };
        node.ancestors()
            .skip(1)
            .filter(is_callable)
            .find(|a| {
                self.bindings
                    .get(a)
                    .is_some_and(|names| names.contains(&name))
            })
            .and_then(|a| self.callables.get(&a))
            .map(|scope| Local {
                scope: scope.clone(),
                name,
            })
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
        let end = span_end(node, start);
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
        let (parent, _) = self.next_child();
        if kind.is_callable() {
            self.callables.insert(node.clone(), id.clone());
        }
        self.nodes.push(Node {
            id: id.clone(),
            parent: Some(parent),
            l1: self.l1.then(|| L1 {
                label: label(node, false, true),
                local: None,
            }),
            kind,
            span,
            body_span,
            name: Some(name),
            qualified_name: Some(path.to_owned()),
            lang: serde_json::json!({"syntax": format!("{:?}", node.kind())}),
        });
        self.parents.push((node.clone(), id, 0));
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

/// The end of the last non-trivia token, if any. Error recovery can leave trailing whitespace
/// or empty nodes in a node, and then rowan's `last_token` is not enough.
fn last_significant(node: &SyntaxNode) -> Option<usize> {
    // Reverse document order: the last child is pushed last and popped first.
    let mut stack: Vec<SyntaxElement> = vec![NodeOrToken::Node(node.clone())];
    while let Some(e) = stack.pop() {
        match e {
            NodeOrToken::Token(t) if !t.kind().is_trivia() => {
                return Some(usize::from(t.text_range().end()));
            }
            NodeOrToken::Token(_) => {}
            NodeOrToken::Node(n) => stack.extend(n.children_with_tokens()),
        }
    }
    None
}

fn span_end(node: &SyntaxNode, start: usize) -> usize {
    last_significant(node).map_or(start, |end| end.max(start))
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

/// FN, CLOSURE_EXPR, or an async/gen block: what a `Local` scope can be.
fn is_callable(node: &SyntaxNode) -> bool {
    match node.kind() {
        SyntaxKind::FN | SyntaxKind::CLOSURE_EXPR => true,
        SyntaxKind::BLOCK_EXPR => ast::BlockExpr::cast(node.clone())
            .is_some_and(|b| b.async_token().is_some() || b.gen_token().is_some()),
        _ => false,
    }
}

/// The name an IDENT_PAT binds. Capitalized names are taken as constants, unit structs or
/// variants (`None`), which a parser alone can't tell from bindings.
fn binding_name(pat: &SyntaxNode) -> Option<String> {
    let name = ast::IdentPat::cast(pat.clone())?.name()?.text().to_string();
    let first = name.chars().next()?;
    (first == '_' || first.is_lowercase()).then_some(name)
}

/// A NAME_REF used as a plain variable: `x` in an expression, or a struct field shorthand.
fn is_local_use(name_ref: &SyntaxNode) -> bool {
    let Some(parent) = name_ref.parent() else {
        return false;
    };
    match parent.kind() {
        SyntaxKind::RECORD_EXPR_FIELD => {
            ast::RecordExprField::cast(parent).is_some_and(|f| f.expr().is_none())
        }
        SyntaxKind::PATH_SEGMENT => {
            let single = parent.children().count() == 1;
            let path = parent.parent();
            let unqualified = path
                .as_ref()
                .and_then(|p| ast::Path::cast(p.clone()))
                .is_some_and(|p| p.qualifier().is_none());
            let in_expr = path
                .and_then(|p| p.parent())
                .is_some_and(|e| e.kind() == SyntaxKind::PATH_EXPR);
            single && unqualified && in_expr
        }
        _ => false,
    }
}

/// Names bound in each callable, by a pattern whose innermost callable it is.
fn bindings(root: &SyntaxNode) -> HashMap<SyntaxNode, HashSet<String>> {
    let mut out: HashMap<SyntaxNode, HashSet<String>> = HashMap::new();
    for pat in root
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::IDENT_PAT)
    {
        let Some(name) = binding_name(&pat) else {
            continue;
        };
        if let Some(owner) = pat.ancestors().skip(1).find(is_callable) {
            out.entry(owner).or_default().insert(name);
        }
    }
    out
}

fn is_assignment(node: &SyntaxNode) -> bool {
    ast::BinExpr::cast(node.clone())
        .and_then(|b| b.op_kind())
        .is_some_and(|op| matches!(op, ast::BinaryOp::Assignment { .. }))
}

/// The L1 label: the syntax kind, then the significant tokens directly under the node,
/// space-separated. Delimiters (`( ) [ ] { } , ;`) and trivia are dropped. A NAME child is
/// folded in, except a named node's own name and a local's. A local NAME_REF drops its text.
/// Macro calls and token trees, emitted as one node, list all their tokens.
fn label(node: &SyntaxNode, local: bool, named: bool) -> String {
    let kind = format!("{:?}", node.kind());
    let mut tokens: Vec<String> = Vec::new();
    if matches!(node.kind(), SyntaxKind::MACRO_CALL | SyntaxKind::TOKEN_TREE) {
        tokens.extend(
            node.descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .filter(|t| !t.kind().is_trivia())
                .map(|t| t.text().to_owned()),
        );
    } else {
        for e in node.children_with_tokens() {
            match e {
                NodeOrToken::Token(t) => {
                    let skip = t.kind().is_trivia()
                        || is_delimiter(t.kind())
                        || (local && node.kind() == SyntaxKind::NAME_REF);
                    if !skip {
                        tokens.push(t.text().to_owned());
                    }
                }
                NodeOrToken::Node(n) if n.kind() == SyntaxKind::NAME && !named && !local => {
                    tokens.push(n.text().to_string());
                }
                NodeOrToken::Node(_) => {}
            }
        }
    }
    if tokens.is_empty() {
        kind
    } else {
        format!("{kind}:{}", tokens.join(" "))
    }
}

fn is_delimiter(kind: SyntaxKind) -> bool {
    use SyntaxKind as K;
    matches!(
        kind,
        K::L_PAREN
            | K::R_PAREN
            | K::L_BRACK
            | K::R_BRACK
            | K::L_CURLY
            | K::R_CURLY
            | K::COMMA
            | K::SEMICOLON
    )
}

#[cfg(test)]
mod tests;
