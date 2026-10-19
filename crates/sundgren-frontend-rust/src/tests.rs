use super::*;

fn names(src: &str) -> Vec<String> {
    let parsed = parse("lib.rs", src.as_bytes()).unwrap();
    parsed.file.validate(src.as_bytes()).unwrap();
    parsed.file.nodes[1..]
        .iter()
        .map(|n| n.qualified_name.clone().unwrap())
        .collect()
}

fn node<'a>(file: &'a File, qual: &str) -> &'a Node {
    file.nodes
        .iter()
        .find(|n| n.qualified_name.as_deref() == Some(qual))
        .unwrap_or_else(|| panic!("no {qual}"))
}

/// Names checked against rustc 1.97 v0 symbols (see design/ir-mapping.md).
#[test]
fn names_follow_rustc_def_paths() {
    let src = r#"
pub struct S;
pub trait T { fn r(&self) -> u32; fn d(&self) -> u32 { (|| 7)() } }
impl S {
    pub fn m(&self) -> u32 {
        let a = |x: u32| {
            let inner = |y: u32| y + 1;
            inner(x)
        };
        let b = |y: u32| y * 2;
        let fut = async { 3 };
        a(1) + b(2) + (|| 5)()
    }
}
impl T for S { fn r(&self) -> u32 { (|| 1)() } }
pub fn f() -> u32 { fn nested() -> u32 { (|| 4)() } nested() }
pub mod inner { pub fn g() -> u32 { 9 } }
pub async fn af(x: u32) -> u32 { let c = |y: u32| y + x; c(1) }
pub static F: fn() -> u32 = || 5;
pub fn in_fn_impl() -> u32 { struct L; impl L { fn m(&self) -> u32 { (|| 6)() } } L.m() }
"#;
    assert_eq!(
        names(src),
        [
            "T",
            "T::r",
            "T::d",
            "T::d::{closure#0}",
            "<S>",
            "<S>::m",
            "<S>::m::{closure#0}",
            "<S>::m::{closure#0}::{closure#0}",
            "<S>::m::{closure#1}",
            "<S>::m::{closure#2}",
            "<S>::m::{closure#3}",
            "<S as T>",
            "<S as T>::r",
            "<S as T>::r::{closure#0}",
            "f",
            "f::nested",
            "f::nested::{closure#0}",
            "inner",
            "inner::g",
            "af",
            "af::{closure#0}::{closure#0}",
            "F::{closure#0}",
            "in_fn_impl",
            "<in_fn_impl::L>",
            "<in_fn_impl::L>::m",
            "<in_fn_impl::L>::m::{closure#0}",
        ]
    );
}

#[test]
fn spans_include_attributes_and_doc_comments_not_plain_comments() {
    let src = "// plain\n#[inline]\n/// Doc.\npub fn add(a: i32) -> i32 {\n    a\n}\n";
    let parsed = parse("lib.rs", src.as_bytes()).unwrap();
    let add = node(&parsed.file, "add");
    assert!(src[add.span.bytes()].starts_with("#[inline]"));
    assert_eq!(add.span.lines(), 2..=6);
    assert_eq!(add.body_span.unwrap().start.line, 5);
}

#[test]
fn body_rules() {
    let src = "\
fn one() -> i32 { 1 }
fn sig(
    x: u8,
) -> u8 {
    x
}
fn empty() {
}
fn tail(
    x: u8) -> u8 { x }
fn closures() {
    let a = |x: u8| x;
    let b = |x: u8| {
        x
    };
    let c = async {
        1
    };
}
trait T { fn decl(&self); }
";
    let parsed = parse("lib.rs", src.as_bytes()).unwrap();
    parsed.file.validate(src.as_bytes()).unwrap();
    let body = |q: &str| node(&parsed.file, q).body_span.map(|b| b.start.line);
    assert_eq!(body("one"), None);
    assert_eq!(body("sig"), Some(5));
    assert_eq!(body("empty"), Some(8));
    assert_eq!(body("tail"), Some(10));
    assert_eq!(body("closures::{closure#0}"), None);
    assert_eq!(body("closures::{closure#1}"), Some(14));
    assert_eq!(body("closures::{closure#2}"), Some(17));
    assert_eq!(body("T::decl"), None);
}

#[test]
fn repeats_get_counters_and_spans_are_exact() {
    let src =
        "#[cfg(unix)]\nfn f() {}\n#[cfg(not(unix))]\nfn f() {}\nfn g() { let s = \"é😀\"; }\n";
    let parsed = parse("lib.rs", src.as_bytes()).unwrap();
    parsed.file.validate(src.as_bytes()).unwrap();
    let ids: Vec<&str> = parsed.file.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["lib.rs", "lib.rs::f", "lib.rs::f#1", "lib.rs::g"]);
    let g = &parsed.file.nodes[3];
    assert_eq!(&src[g.span.bytes()], "fn g() { let s = \"é😀\"; }");
    assert_eq!(g.span.end.col, 29); // 28 bytes, 24 characters
}

#[test]
fn broken_code_still_gives_ir() {
    let src = "fn ok() {\n    1;\n}\nfn broken( {\n";
    let parsed = parse("lib.rs", src.as_bytes()).unwrap();
    assert!(!parsed.errors.is_empty());
    parsed.file.validate(src.as_bytes()).unwrap();
    assert_eq!(names(src), ["ok", "broken"]);
}

#[test]
fn bom_crlf_and_non_utf8() {
    let src = "\u{feff}fn f() {\r\n    1;\r\n}\r\n";
    let parsed = parse("lib.rs", src.as_bytes()).unwrap();
    parsed.file.validate(src.as_bytes()).unwrap();
    let f = node(&parsed.file, "f");
    assert_eq!(f.span.start.byte, 3);
    assert_eq!(f.span.lines(), 1..=3);
    assert_eq!(
        parse("x.rs", b"fn f() {}\xff").unwrap_err(),
        FrontendError::NotUtf8
    );
}

fn l1(src: &str) -> File {
    let opts = Options {
        l1: true,
        ..Options::default()
    };
    let file = parse_with("lib.rs", src.as_bytes(), opts).unwrap().file;
    file.validate(src.as_bytes()).unwrap();
    file
}

/// (label, local name) of every node under `qual`, in node order.
fn l1_rows(file: &File, qual: &str) -> Vec<(String, Option<String>)> {
    let root = node(file, qual).id.clone();
    let mut inside = vec![root.clone()];
    let mut rows = Vec::new();
    for n in &file.nodes {
        if n.id == root || n.parent.as_ref().is_some_and(|p| inside.contains(p)) {
            inside.push(n.id.clone());
            let l1 = n.l1.as_ref().unwrap();
            rows.push((l1.label.clone(), l1.local.as_ref().map(|l| l.name.clone())));
        }
    }
    rows
}

#[test]
fn l1_labels_and_locals() {
    let src = "fn add(a: i32, b: i32) -> i32 {\n    let c = a + b; // sum\n    c\n}\n";
    let file = l1(src);
    assert_eq!(file.levels, ["L1"]);
    let rows: Vec<(String, Option<String>)> = l1_rows(&file, "add");
    let row = |label: &str, local: Option<&str>| (label.to_owned(), local.map(str::to_owned));
    assert_eq!(
        rows,
        [
            row("FN:fn", None),
            row("PARAM_LIST", None),
            row("PARAM::", None),
            row("IDENT_PAT", Some("a")),
            row("PATH_TYPE", None),
            row("PATH", None),
            row("PATH_SEGMENT", None),
            row("NAME_REF:i32", None),
            row("PARAM::", None),
            row("IDENT_PAT", Some("b")),
            row("PATH_TYPE", None),
            row("PATH", None),
            row("PATH_SEGMENT", None),
            row("NAME_REF:i32", None),
            row("RET_TYPE:->", None),
            row("PATH_TYPE", None),
            row("PATH", None),
            row("PATH_SEGMENT", None),
            row("NAME_REF:i32", None),
            row("BLOCK_EXPR", None),
            row("STMT_LIST", None),
            row("LET_STMT:let =", None),
            row("IDENT_PAT", Some("c")),
            row("BIN_EXPR:+", None),
            row("PATH_EXPR", None),
            row("PATH", None),
            row("PATH_SEGMENT", None),
            row("NAME_REF", Some("a")),
            row("PATH_EXPR", None),
            row("PATH", None),
            row("PATH_SEGMENT", None),
            row("NAME_REF", Some("b")),
            row("PATH_EXPR", None),
            row("PATH", None),
            row("PATH_SEGMENT", None),
            row("NAME_REF", Some("c")),
        ]
    );
    let let_stmt = file.nodes.iter().find(|n| n.kind == Kind::Assign).unwrap();
    assert_eq!(let_stmt.id.as_str(), "lib.rs::add/2/0/0");
    assert_eq!(&src[let_stmt.span.bytes()], "let c = a + b;");
}

#[test]
fn l1_scopes_closures_macros_and_renamed_clones() {
    let src = "\
fn outer(x: u8) -> u8 {
    let f = move |y: u8| x + y + helper();
    println!(\"{}\", x);
    f(1)
}
fn helper() -> u8 { 1 }
fn same(p: u8) -> u8 {
    let g = move |q: u8| p + q + helper();
    println!(\"{}\", p);
    g(1)
}
";
    let file = l1(src);
    let closure = node(&file, "outer::{closure#0}");
    assert_eq!(closure.l1.as_ref().unwrap().label, "CLOSURE_EXPR:move");
    let locals: Vec<(String, String)> = file
        .nodes
        .iter()
        .filter_map(|n| n.l1.as_ref()?.local.as_ref())
        .filter(|l| l.scope.as_str().starts_with("lib.rs::outer"))
        .map(|l| (l.scope.as_str().to_owned(), l.name.clone()))
        .collect();
    let outer = "lib.rs::outer".to_owned();
    let inner = "lib.rs::outer::{closure#0}".to_owned();
    assert_eq!(
        locals,
        [
            (outer.clone(), "x".into()),
            (outer.clone(), "f".into()),
            (inner.clone(), "y".into()),
            (outer.clone(), "x".into()),
            (inner, "y".into()),
            (outer.clone(), "f".into()),
        ]
    );
    // Non-local names stay in labels; macro calls are one node.
    let rows = l1_rows(&file, "outer");
    assert!(rows.contains(&("NAME_REF:helper".into(), None)));
    assert!(rows.contains(&("MACRO_CALL:println ! ( \"{}\" , x )".into(), None)));
    // Same structure, different local names: equal labels everywhere but the macro text.
    let other = l1_rows(&file, "same");
    let strip = |rows: Vec<(String, Option<String>)>| -> Vec<String> {
        rows.into_iter()
            .map(|(l, _)| l)
            .filter(|l| !l.starts_with("MACRO_CALL"))
            .collect()
    };
    assert_eq!(strip(rows), strip(other));
}

#[test]
fn l1_named_children_keep_named_ids_and_l0_is_unchanged() {
    let src =
        "impl S {\n    fn m(&self) -> u8 {\n        if true { (|| 1)() } else { 2 }\n    }\n}\n";
    let file = l1(src);
    let closure = node(&file, "<S>::m::{closure#0}");
    assert_eq!(closure.id.as_str(), "lib.rs::<S>::m::{closure#0}");
    assert!(
        closure
            .parent
            .as_ref()
            .unwrap()
            .as_str()
            .starts_with("lib.rs::<S>::m/")
    );
    assert!(file.nodes.iter().any(|n| n.kind == Kind::If));
    // L0 off: the same named nodes, no l1 payloads.
    let l0 = parse("lib.rs", src.as_bytes()).unwrap().file;
    assert!(l0.levels.is_empty() && l0.nodes.iter().all(|n| n.l1.is_none()));
    let named = |f: &File| -> Vec<(String, Span, Option<Span>)> {
        f.nodes
            .iter()
            .filter(|n| n.qualified_name.is_some())
            .map(|n| (n.id.to_string(), n.span, n.body_span))
            .collect()
    };
    assert_eq!(named(&file), named(&l0));
}

#[test]
fn l1_spans_survive_error_recovery() {
    // Empty and trailing-whitespace nodes from error recovery (from ra's parser test data).
    for src in [
        "fn f(_: impl) {}",
        "type Generic = dyn<Path>;\n",
        "impl<T:\nuse std;\n",
        "fn ok() {}\nfn broken( {\n",
    ] {
        l1(src);
        let l0 = parse("lib.rs", src.as_bytes()).unwrap().file;
        l0.validate(src.as_bytes()).unwrap();
    }
}
