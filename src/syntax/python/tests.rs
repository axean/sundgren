use super::*;
use crate::syntax::{Language, lex};

/// (depth, kind, name, start, end) of each item, depth first.
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
    walk(&items(source, &lex(Language::Python, source)), 0, &mut out);
    out
}

#[test]
fn functions_and_classes_nest_by_indentation() {
    let source = "\
import os

@dataclass
class A(Base):
    x: int = 1

    def f(self):
        return 1
    # a comment, not code

    @property
    async def g(self):
        def inner():
            pass
        return inner


def h(a,
      b):
    '''docstring
not indented'''
    return (a +
b)
x = 1
";
    assert_eq!(
        outline(source),
        [
            (0, "class", "A".to_owned(), 3, 15),
            (1, "function", "f".to_owned(), 7, 8),
            (1, "function", "g".to_owned(), 11, 15),
            (2, "function", "inner".to_owned(), 13, 14),
            (0, "function", "h".to_owned(), 18, 23),
        ]
    );
}

#[test]
fn one_liners_and_backslashes() {
    let source = "def f(): return 1\ndef g(a, \\\nb):\n    pass\nclass C: pass\n";
    assert_eq!(
        outline(source),
        [
            (0, "function", "f".to_owned(), 1, 1),
            (0, "function", "g".to_owned(), 2, 4),
            (0, "class", "C".to_owned(), 5, 5),
        ]
    );
}

#[test]
fn nothing_to_read() {
    assert!(outline("").is_empty());
    assert!(outline("x = 1\n# def f():\ns = 'def g(): pass'\n").is_empty());
}
