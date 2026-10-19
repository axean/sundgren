//! Structural hashing and duplicate functions on hand-built L1 IR.

use std::collections::BTreeSet;

use sundgren_analyze::{Confidence, DuplicateOptions, duplicate_functions, structural_hashes};
use sundgren_ir::{File, Kind, L1, Local, Newlines, Node, NodeId, Pos, SCHEMA, Span};
use sundgren_join::{CoverageFact, Granularity, NodeStatus, Overlay};

/// A tree spec: kind, label, local (scope qualified name, name), qualified name, children.
struct T {
    kind: Kind,
    label: &'static str,
    local: Option<(&'static str, &'static str)>,
    qual: Option<&'static str>,
    children: Vec<T>,
}

fn t(kind: Kind, label: &'static str, children: Vec<T>) -> T {
    T {
        kind,
        label,
        local: None,
        qual: None,
        children,
    }
}

fn local(kind: Kind, label: &'static str, scope: &'static str, name: &'static str) -> T {
    T {
        kind,
        label,
        local: Some((scope, name)),
        qual: None,
        children: vec![],
    }
}

fn func(qual: &'static str, children: Vec<T>) -> T {
    T {
        kind: Kind::Function,
        label: "FunctionDef",
        local: None,
        qual: Some(qual),
        children,
    }
}

/// `def <qual>(<p>): return <p> + 1 * 2 - 3`, with a few more nodes to pass the threshold.
fn add_one(qual: &'static str, p: &'static str) -> T {
    func(
        qual,
        vec![
            local(Kind::Other("arg".into()), "arg", qual, p),
            t(
                Kind::Return,
                "Return",
                vec![t(
                    Kind::Expr,
                    "BinOp:Sub",
                    vec![
                        t(
                            Kind::Expr,
                            "BinOp:Add",
                            vec![
                                local(Kind::Expr, "Name", qual, p),
                                t(
                                    Kind::Expr,
                                    "BinOp:Mult",
                                    vec![
                                        t(Kind::Expr, "Constant:int:1", vec![]),
                                        t(Kind::Expr, "Constant:int:2", vec![]),
                                    ],
                                ),
                            ],
                        ),
                        t(Kind::Expr, "Constant:int:3", vec![]),
                    ],
                )],
            ),
        ],
    )
}

fn file(path: &str, top: Vec<T>) -> File {
    let pos = Pos {
        byte: 0,
        line: 1,
        col: 1,
    };
    let span = Span {
        start: pos,
        end: pos,
    };
    let root = NodeId::root(path);
    let mut nodes = vec![Node {
        id: root.clone(),
        parent: None,
        kind: Kind::Module,
        span,
        body_span: None,
        name: None,
        qualified_name: None,
        lang: serde_json::Value::Null,
        l1: Some(L1 {
            label: "Module".into(),
            local: None,
        }),
    }];
    fn add(path: &str, nodes: &mut Vec<Node>, parent: &NodeId, k: usize, t: T, span: Span) {
        let id = match t.qual {
            Some(q) => NodeId::new(format!("{path}::{q}")),
            None => NodeId::child(parent, k),
        };
        let local = t.local.map(|(scope, name)| Local {
            scope: NodeId::new(format!("{path}::{scope}")),
            name: name.into(),
        });
        nodes.push(Node {
            id: id.clone(),
            parent: Some(parent.clone()),
            kind: t.kind,
            span,
            body_span: None,
            name: t.qual.map(|q| q.rsplit('.').next().unwrap().into()),
            qualified_name: t.qual.map(Into::into),
            lang: serde_json::Value::Null,
            l1: Some(L1 {
                label: t.label.into(),
                local,
            }),
        });
        for (i, c) in t.children.into_iter().enumerate() {
            add(path, nodes, &id, i, c, span);
        }
    }
    for (i, c) in top.into_iter().enumerate() {
        add(path, &mut nodes, &root, i, c, span);
    }
    File {
        schema: SCHEMA.into(),
        path: path.into(),
        language: "python".into(),
        producer: "hand".into(),
        source_sha256: String::new(),
        newlines: Newlines::Universal,
        nodes,
        levels: vec!["L1".into()],
    }
}

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn exact_and_renamed_hashes() {
    let f = file(
        "m.py",
        vec![add_one("f", "a"), add_one("g", "b"), add_one("h", "a")],
    );
    let h = structural_hashes(&f);
    let (hf, hg, hh) = (&h[&id("m.py::f")], &h[&id("m.py::g")], &h[&id("m.py::h")]);
    assert_eq!(hf.exact, hh.exact, "the function's own name is not hashed");
    assert_ne!(hf.exact, hg.exact, "local names count for exact");
    assert_eq!(hf.renamed, hg.renamed);
    assert_eq!(hf.size, 10);
    // The Name `a` inside f: free in its own subtree, so renaming keeps it.
    let ret_f = &h[&id("m.py::f/1")];
    let ret_g = &h[&id("m.py::g/1")];
    assert_ne!(ret_f.renamed, ret_g.renamed);
    // L0-only files get no hashes.
    let mut l0 = f.clone();
    l0.levels.clear();
    assert!(structural_hashes(&l0).is_empty());
}

#[test]
fn labels_and_shape_matter() {
    let mut other = add_one("g", "a");
    other.children[1].children[0].label = "BinOp:Add";
    let f = file("m.py", vec![add_one("f", "a"), other]);
    let h = structural_hashes(&f);
    assert_ne!(h[&id("m.py::f")].renamed, h[&id("m.py::g")].renamed);
}

#[test]
fn renaming_follows_first_use_order() {
    // def f(a, b): return a - b   vs   def g(a, b): return b - a
    let body = |qual: &'static str, x: &'static str, y: &'static str| {
        func(
            qual,
            vec![
                local(Kind::Other("arg".into()), "arg", qual, "a"),
                local(Kind::Other("arg".into()), "arg", qual, "b"),
                t(
                    Kind::Return,
                    "Return",
                    vec![t(
                        Kind::Expr,
                        "BinOp:Sub",
                        vec![
                            local(Kind::Expr, "Name", qual, x),
                            local(Kind::Expr, "Name", qual, y),
                        ],
                    )],
                ),
            ],
        )
    };
    let f = file(
        "m.py",
        vec![
            body("f", "a", "b"),
            body("g", "b", "a"),
            body("h", "a", "b"),
        ],
    );
    let h = structural_hashes(&f);
    assert_ne!(h[&id("m.py::f")].renamed, h[&id("m.py::g")].renamed);
    assert_eq!(h[&id("m.py::f")].renamed, h[&id("m.py::h")].renamed);
}

fn fact(status: NodeStatus, hit: usize) -> CoverageFact {
    CoverageFact {
        granularity: Granularity::Line,
        executable: (1..=3).collect(),
        hit: (1..=hit as i64).collect(),
        missed: BTreeSet::new(),
        flaky: BTreeSet::new(),
        shared: BTreeSet::new(),
        branches: vec![],
        tests: None,
        function: None,
        status,
    }
}

#[test]
fn duplicates_group_prefer_the_covered_copy_and_skip_trivial_ones() {
    let a = file("a.py", vec![add_one("f", "x"), add_one("tiny", "x")]);
    let b = file("b.py", vec![add_one("g", "y")]);
    let mut overlay = Overlay::default();
    overlay
        .facts
        .insert(id("a.py::f"), fact(NodeStatus::Uncovered, 0));
    overlay
        .facts
        .insert(id("b.py::g"), fact(NodeStatus::Covered, 3));
    let files = [a, b];
    let out = duplicate_functions(&files, &overlay, DuplicateOptions { min_nodes: 10 });
    assert!(out.is_empty(), "9 nodes under each function is below 10");
    let out = duplicate_functions(&files, &overlay, DuplicateOptions { min_nodes: 9 });
    // f and tiny are exact copies in a.py; g differs only in its local name.
    assert_eq!(out.len(), 1);
    let s = &out[0];
    assert_eq!(s.id, "duplicate-function:a.py::f,a.py::tiny,b.py::g");
    assert_eq!(s.nodes, [id("b.py::g"), id("a.py::f"), id("a.py::tiny")]);
    assert_eq!(s.analysis, "duplicate-function");
    assert!(
        s.message.contains("identical up to local names"),
        "{}",
        s.message
    );
    assert!(s.message.starts_with("3 functions"));
    assert_eq!(s.confidence, Confidence::Medium);
    assert_eq!(s.evidence.nodes[2].status, None);
    assert!(s.edits.is_none());
}

#[test]
fn no_covered_copy_is_reported() {
    let f = file("m.py", vec![add_one("f", "a"), add_one("g", "a")]);
    let out = duplicate_functions(&[f], &Overlay::default(), DuplicateOptions { min_nodes: 1 });
    assert_eq!(out.len(), 1);
    assert!(out[0].message.contains("identical."), "{}", out[0].message);
    assert!(out[0].message.ends_with("No copy is covered by any test."));
    assert_eq!(out[0].confidence, Confidence::Low);
    assert_eq!(out[0].nodes[0], id("m.py::f"), "ties keep the earliest id");
}

#[test]
fn copies_nested_in_reported_copies_are_not_repeated() {
    let outer = |q: &'static str, inner: &'static str| func(q, vec![add_one(inner, "a")]);
    let f = file(
        "m.py",
        vec![outer("f", "f.<locals>.i"), outer("g", "g.<locals>.i")],
    );
    let out = duplicate_functions(&[f], &Overlay::default(), DuplicateOptions { min_nodes: 1 });
    let ids: Vec<&str> = out.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["duplicate-function:m.py::f,m.py::g"]);
}

#[test]
fn touched_lists_each_node_span() {
    let f = file("m.py", vec![add_one("f", "a"), add_one("g", "a")]);
    let out = duplicate_functions(
        std::slice::from_ref(&f),
        &Overlay::default(),
        DuplicateOptions { min_nodes: 1 },
    );
    let t = out[0].touched(&[f]);
    assert_eq!(t, [("m.py".into(), 1, 1), ("m.py".into(), 1, 1)]);
}
