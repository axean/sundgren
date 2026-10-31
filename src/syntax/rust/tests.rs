use super::*;
use crate::syntax::{Language, lex};

fn outline(source: &str) -> Vec<(usize, &'static str, String, usize, usize)> {
    fn walk(
        items: &[Item],
        depth: usize,
        out: &mut Vec<(usize, &'static str, String, usize, usize)>,
    ) {
        for item in items {
            out.push((depth, item.kind, item.name.clone(), item.start, item.end));
            walk(&item.children, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(&items(source, &lex(Language::Rust, source)), 0, &mut out);
    out
}

fn item(
    depth: usize,
    kind: &'static str,
    name: &str,
    start: usize,
    end: usize,
) -> (usize, &'static str, String, usize, usize) {
    (depth, kind, name.to_owned(), start, end)
}

#[test]
fn items_with_attributes_and_docs() {
    let source = "\
//! Crate docs.
#![allow(dead_code)]
use std::fmt;

/// A thing.
#[derive(Debug)]
pub(crate) struct A<T> where T: Copy {
    x: T,
}

// Not a doc comment.
struct Unit;
struct Pair(u8, u8);

pub const fn make() -> u8 {
    fn helper() {}
    1
}
const TABLE: [u8; 2] = { [1, 2] };
static mut COUNT: usize = 0;
pub type F = fn(u8) -> u8;
macro_rules! twice {
    ($e:expr) => { $e; $e };
}
union U { a: u8 }
";
    assert_eq!(
        outline(source),
        [
            item(0, "struct", "A", 5, 9),
            item(0, "struct", "Unit", 12, 12),
            item(0, "struct", "Pair", 13, 13),
            item(0, "function", "make", 15, 18),
            item(0, "const", "TABLE", 19, 19),
            item(0, "static", "COUNT", 20, 20),
            item(0, "type", "F", 21, 21),
            item(0, "macro", "twice", 22, 24),
            item(0, "union", "U", 25, 25),
        ]
    );
}

#[test]
fn impls_traits_and_modules_have_children() {
    let source = "\
impl<'a, T: Fn() -> u8> fmt::Display for Wrapper<'a, T>
where
    T: Copy,
{
    /// Doc.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = \"fn not_an_item() {}\";
        Ok(())
    }
}

unsafe impl Send for Raw {}

pub trait Shape {
    const SIDES: u8;
    fn area(&self) -> f64;
    fn name(&self) -> &str { \"shape\" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    async unsafe extern \"C\" fn works() {}
}
mod elsewhere;
extern \"C\" {
    fn abs(x: i32) -> i32;
}
thread_local! { static X: u8 = 1; }
";
    assert_eq!(
        outline(source),
        [
            item(0, "impl", "fmt::Display for Wrapper<'a, T>", 1, 10),
            item(1, "function", "fmt", 5, 9),
            item(0, "impl", "Send for Raw", 12, 12),
            item(0, "trait", "Shape", 14, 18),
            item(1, "const", "SIDES", 15, 15),
            item(1, "function", "area", 16, 16),
            item(1, "function", "name", 17, 17),
            item(0, "module", "tests", 20, 26),
            item(1, "function", "works", 24, 25),
            item(0, "module", "elsewhere", 27, 27),
        ]
    );
}

#[test]
fn broken_code_still_reads() {
    let source = "fn open() {\n    if x {\n\nfn next() {}\n";
    let items = outline(source);
    assert_eq!(items[0], item(0, "function", "open", 1, 4));
    assert!(outline("impl").iter().all(|i| i.1 == "impl"));
    assert_eq!(outline("macro_rules! m")[0].2, "m");
}
