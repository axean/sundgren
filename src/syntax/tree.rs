//! Reading code with tree-sitter: one engine for every language. A language is a grammar and a
//! query (`queries/*.scm`) whose captures say what its nodes are:
//!
//! - `@item.KIND`: an item of that kind, named by the `@name` nodes (first to last).
//! - `@binding.KIND`: an item too, except inside a function.
//! - `@scope.function`: a function that isn't an item; what's in it is inside a function.
//! - `@lead`: a comment or attribute that belongs to the item right after it.
//! - `@token.KIND`: one token of that kind: `comment`, `string`, `number` or `directive`.
//!
//! The same rules for every language: items nest by where they are; a value (a variable, a
//! constant, a field, ...) holds no items, what's in it belongs to its parent; a variable in a
//! class, struct, ... is a field.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};

use super::{Item, KINDS, Kind, Language, Token};

/// Items with a body of code: what's in them is inside a function.
const FUNCTIONS: [&str; 2] = ["function", "prototype"];
/// Items whose variables are fields.
const RECORDS: [&str; 6] = ["class", "struct", "union", "interface", "trait", "impl"];
/// Items that are values: they hold no items.
const VALUES: [&str; 7] = [
    "variable", "const", "static", "field", "import", "call", "variant",
];

/// What a capture says about its node.
#[derive(Clone, Copy)]
enum Role {
    Item(&'static str),
    Binding(&'static str),
    Scope,
    Name,
    Lead,
    Token(Kind),
}

struct Grammar {
    language: tree_sitter::Language,
    query: Query,
    /// By capture index.
    roles: Vec<Option<Role>>,
}

const C: &str = include_str!("queries/c.scm");
const ECMASCRIPT: &str = include_str!("queries/ecmascript.scm");

fn grammar(language: Language) -> &'static Grammar {
    static GRAMMARS: [OnceLock<Grammar>; 7] = [const { OnceLock::new() }; 7];
    let (index, ts, query): (usize, tree_sitter::Language, String) = match language {
        Language::Python => (
            0,
            tree_sitter_python::LANGUAGE.into(),
            include_str!("queries/python.scm").into(),
        ),
        Language::Rust => (
            1,
            tree_sitter_rust::LANGUAGE.into(),
            include_str!("queries/rust.scm").into(),
        ),
        Language::C => (2, tree_sitter_c::LANGUAGE.into(), C.into()),
        Language::Cpp => (
            3,
            tree_sitter_cpp::LANGUAGE.into(),
            [C, include_str!("queries/cpp.scm")].concat(),
        ),
        Language::JavaScript => (
            4,
            tree_sitter_javascript::LANGUAGE.into(),
            [ECMASCRIPT, include_str!("queries/javascript.scm")].concat(),
        ),
        Language::TypeScript | Language::Tsx => {
            let tsx = language == Language::Tsx;
            (
                if tsx { 6 } else { 5 },
                if tsx {
                    tree_sitter_typescript::LANGUAGE_TSX.into()
                } else {
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
                },
                [ECMASCRIPT, include_str!("queries/typescript.scm")].concat(),
            )
        }
    };
    GRAMMARS[index].get_or_init(|| {
        let query = Query::new(&ts, &query)
            .unwrap_or_else(|e| panic!("query for {}: {e}", language.name()));
        let roles = query.capture_names().iter().map(|&c| role(c)).collect();
        Grammar {
            language: ts,
            query,
            roles,
        }
    })
}

fn role(capture: &str) -> Option<Role> {
    // The kind as one of `KINDS`, which outlive the query.
    let kind = |kind: &str| {
        *KINDS
            .iter()
            .find(|&&k| k == kind)
            .unwrap_or_else(|| panic!("query: no kind {kind}"))
    };
    let token = |kind: &str| match kind {
        "comment" => Some(Kind::Comment),
        "string" => Some(Kind::String),
        "number" => Some(Kind::Number),
        "directive" => Some(Kind::Directive),
        _ => None,
    };
    match capture.split_once('.') {
        None if capture == "name" => Some(Role::Name),
        None if capture == "lead" => Some(Role::Lead),
        Some(("item", k)) => Some(Role::Item(kind(k))),
        Some(("binding", k)) => Some(Role::Binding(kind(k))),
        Some(("scope", "function")) => Some(Role::Scope),
        Some(("token", kind)) => token(kind).map(Role::Token),
        _ => None,
    }
}

fn parse(grammar: &Grammar, source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&grammar.language).ok()?;
    parser.parse(source, None)
}

/// An item as found, before nesting.
struct Found {
    kind: &'static str,
    binding: bool,
    scope: bool,
    name: String,
    bytes: (usize, usize),
    start: usize,
    end: usize,
    pattern: usize,
}

pub(super) fn items(language: Language, source: &str) -> Vec<Item> {
    let grammar = grammar(language);
    let Some(tree) = parse(grammar, source) else {
        return Vec::new();
    };
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&grammar.query, tree.root_node(), source.as_bytes());
    let mut leads: HashSet<usize> = HashSet::new();
    // One item per name: the widest match wins, then the first pattern.
    let mut found: HashMap<(usize, usize), Found> = HashMap::new();
    while let Some(m) = matches.next() {
        let mut item: Option<(Node, &'static str, bool, bool)> = None;
        let mut names: Vec<Node> = Vec::new();
        for c in m.captures() {
            match grammar.roles[c.index as usize] {
                Some(Role::Item(kind)) => item = Some((c.node, kind, false, false)),
                Some(Role::Binding(kind)) => item = Some((c.node, kind, true, false)),
                Some(Role::Scope) => item = Some((c.node, "function", true, true)),
                Some(Role::Name) => names.push(c.node),
                Some(Role::Lead) => {
                    leads.insert(c.node.id());
                }
                Some(Role::Token(_)) | None => {}
            }
        }
        let Some((node, kind, binding, scope)) = item else {
            continue;
        };
        let (first, last) = if scope {
            (node, node)
        } else {
            let (Some(first), Some(last)) = (
                names.iter().min_by_key(|n| n.start_byte()),
                names.iter().max_by_key(|n| n.end_byte()),
            ) else {
                continue;
            };
            (*first, *last)
        };
        let new = Found {
            kind,
            binding,
            scope,
            name: if scope {
                String::new()
            } else {
                name(source, first.start_byte(), last.end_byte())
            },
            bytes: (node.start_byte(), node.end_byte()),
            start: node.start_position().row + 1,
            end: end_line(node),
            pattern: m.pattern_index,
        };
        let key = (first.start_byte(), last.end_byte());
        let width = |f: &Found| f.bytes.1 - f.bytes.0;
        let keep_old = found.get(&key).is_some_and(|old| {
            (width(old), std::cmp::Reverse(old.pattern))
                >= (width(&new), std::cmp::Reverse(new.pattern))
        });
        if !keep_old {
            found.insert(key, new);
        }
    }
    let mut found: Vec<Found> = found.into_values().collect();
    for f in &mut found {
        f.start = lead_start(&tree, f, &leads);
    }
    found.sort_by_key(|f| (f.bytes.0, std::cmp::Reverse(f.bytes.1)));
    nest(&mut found.into_iter().peekable(), usize::MAX, None, false)
}

/// The text of a name: whitespace collapsed, quotes around a whole string dropped.
fn name(source: &str, start: usize, end: usize) -> String {
    let text = source[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let quoted = text.len() >= 2
        && ["\"", "'", "`"]
            .iter()
            .any(|q| text.starts_with(q) && text.ends_with(q));
    if quoted {
        text[1..text.len() - 1].to_owned()
    } else {
        text
    }
}

/// The last line of `node`, 1-based: not the line after it when it ends with a newline.
fn end_line(node: Node) -> usize {
    let end = node.end_position();
    if end.column == 0 && end.row > node.start_position().row {
        end.row
    } else {
        end.row + 1
    }
}

/// The first line of an item, with the comments and attributes right before it.
fn lead_start(tree: &Tree, found: &Found, leads: &HashSet<usize>) -> usize {
    let Some(mut node) = tree
        .root_node()
        .descendant_for_byte_range(found.bytes.0, found.bytes.1)
    else {
        return found.start;
    };
    // From the outermost node that starts on the same line: `export function`, `pub fn`.
    while let Some(parent) = node.parent() {
        if parent.parent().is_none() || parent.start_position().row != node.start_position().row {
            break;
        }
        node = parent;
    }
    let mut start = found.start.min(node.start_position().row + 1);
    while let Some(before) = node.prev_sibling() {
        if !leads.contains(&before.id()) || end_line(before) + 1 < start {
            break;
        }
        start = before.start_position().row + 1;
        node = before;
    }
    start
}

/// The items that start before byte `end`, as children of an item of kind `parent`.
fn nest(
    found: &mut std::iter::Peekable<std::vec::IntoIter<Found>>,
    end: usize,
    parent: Option<&'static str>,
    in_function: bool,
) -> Vec<Item> {
    let mut out = Vec::new();
    while let Some(f) = found.next_if(|f| f.bytes.0 < end) {
        let inside = in_function || f.scope || FUNCTIONS.contains(&f.kind);
        let children = nest(found, f.bytes.1, Some(f.kind), inside);
        if f.scope || f.binding && in_function || VALUES.contains(&f.kind) {
            // Not an item, or one that holds none: what's in it moves up.
            if !(f.scope || f.binding && in_function) {
                out.push(item(&f, parent, Vec::new()));
            }
            out.extend(children);
            continue;
        }
        out.push(item(&f, parent, children));
    }
    out
}

fn item(f: &Found, parent: Option<&str>, children: Vec<Item>) -> Item {
    let field = f.binding && f.kind == "variable" && parent.is_some_and(|p| RECORDS.contains(&p));
    Item {
        kind: if field { "field" } else { f.kind },
        name: f.name.clone(),
        start: f.start,
        end: f.end,
        children,
    }
}

pub(super) fn tokens(language: Language, source: &str) -> Vec<Token> {
    let grammar = grammar(language);
    let Some(tree) = parse(grammar, source) else {
        return Vec::new();
    };
    let mut kinds: HashMap<usize, Kind> = HashMap::new();
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&grammar.query, tree.root_node(), source.as_bytes());
    while let Some((m, index)) = captures.next() {
        let c = m.captures()[*index];
        if let Some(Role::Token(kind)) = grammar.roles[c.index as usize] {
            kinds.entry(c.node.id()).or_insert(kind);
        }
    }
    let mut out = Vec::new();
    let mut walk = tree.walk();
    'tree: loop {
        let node = walk.node();
        let kind = kinds.get(&node.id()).copied();
        let leaf = kind.is_some() || node.child_count() == 0;
        if leaf && node.end_byte() > node.start_byte() {
            out.push(Token {
                kind: kind.unwrap_or_else(|| leaf_kind(&source[node.byte_range()])),
                start: node.start_byte(),
                end: node.end_byte(),
                line: node.start_position().row + 1,
                end_line: end_line(node).max(node.start_position().row + 1),
            });
        }
        if !leaf && walk.goto_first_child() {
            continue;
        }
        while !walk.goto_next_sibling() {
            if !walk.goto_parent() {
                break 'tree;
            }
        }
    }
    out
}

/// A token the query doesn't name: a number, a word, or punctuation.
fn leaf_kind(text: &str) -> Kind {
    if text.starts_with(|c: char| c.is_ascii_digit()) {
        Kind::Number
    } else if text
        .chars()
        .any(|c| c.is_alphanumeric() || c == '_' || c == '$')
    {
        Kind::Ident
    } else {
        Kind::Punct
    }
}
