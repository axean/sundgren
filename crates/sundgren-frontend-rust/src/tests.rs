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
