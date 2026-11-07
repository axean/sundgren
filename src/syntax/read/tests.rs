use crate::syntax::{Item, Language, items};

/// (depth, kind, name, start, end) of each item, depth first.
fn outline(language: Language, source: &str) -> Vec<(usize, &'static str, String, usize, usize)> {
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
    walk(&items(language, source), 0, &mut out);
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

/// (kind, name) of each item, depth first.
fn names(language: Language, source: &str) -> Vec<(&'static str, String)> {
    outline(language, source)
        .into_iter()
        .map(|(_, kind, name, _, _)| (kind, name))
        .collect()
}

fn has(language: Language, source: &str, kind: &str, name: &str) -> bool {
    names(language, source)
        .iter()
        .any(|(k, n)| *k == kind && n == name)
}

// Every language, the same rules ----------------------------------------------------------------

#[test]
fn imports_in_every_language() {
    let cases = [
        (
            Language::Python,
            "import os\nfrom a.b import c\n",
            ["os", "a.b"],
        ),
        (
            Language::Rust,
            "use std::fmt;\nuse a::{b, c};\n",
            ["std::fmt", "a::{b, c}"],
        ),
        (
            Language::C,
            "#include <stdio.h>\n#include \"x.h\"\n",
            ["<stdio.h>", "\"x.h\""],
        ),
        (
            Language::Cpp,
            "#include <vector>\nusing namespace std;\n",
            ["<vector>", "namespace std"],
        ),
        (
            Language::TypeScript,
            "import * as x from \"vscode\";\nimport { a } from './a.js';\n",
            ["vscode", "./a.js"],
        ),
        (
            Language::JavaScript,
            "import a from 'a';\nexport { b } from \"./b.js\";\n",
            ["a", "./b.js"],
        ),
    ];
    for (language, source, expected) in cases {
        for name in expected {
            assert!(
                has(language, source, "import", name),
                "{language:?}: {name}"
            );
        }
    }
}

#[test]
fn definitions_inside_functions_in_every_language() {
    let cases = [
        (
            Language::Python,
            "def outer():\n    if x:\n        def inner():\n            pass\n",
        ),
        (
            Language::Rust,
            "fn outer() {\n    if x {\n        fn inner() {}\n    }\n}\n",
        ),
        (
            Language::TypeScript,
            "function outer() {\n  if (x) {\n    function inner() {}\n  }\n}\n",
        ),
        (
            Language::Cpp,
            "void outer() {\n  if (x) {\n    struct inner { int a; };\n  }\n}\n",
        ),
    ];
    for (language, source) in cases {
        let found = outline(language, source);
        assert_eq!(found[0].2, "outer", "{language:?}");
        assert_eq!(
            (found[1].0, found[1].2.as_str()),
            (1, "inner"),
            "{language:?}"
        );
    }
    // But not bindings or calls: they are the function's code.
    assert_eq!(
        outline(Language::Python, "def f():\n    x = 1\n    g()\n").len(),
        1
    );
    assert_eq!(
        outline(
            Language::TypeScript,
            "function f() {\n  const x = 1;\n  g();\n}\n"
        )
        .len(),
        1
    );
}

#[test]
fn calls_at_the_top_are_items_in_every_language() {
    let cases = [
        (Language::Python, "main()\n", "main"),
        (
            Language::Rust,
            "lazy_static! { static ref X: u8 = 1; }\n",
            "lazy_static!",
        ),
        (Language::C, "EXPORT_SYMBOL(add);\n", "EXPORT_SYMBOL"),
        (Language::Cpp, "TEST(Suite, Name);\n", "TEST"),
        (
            Language::JavaScript,
            "describe(\"x\", () => {\n  it(\"y\", () => {});\n});\n",
            "describe",
        ),
        (Language::TypeScript, "app.listen(3000);\n", "app.listen"),
    ];
    for (language, source, name) in cases {
        assert!(has(language, source, "call", name), "{language:?}");
    }
}

#[test]
fn functions_as_values_in_every_language() {
    assert!(has(
        Language::Python,
        "handler = lambda x: x\n",
        "function",
        "handler"
    ));
    assert!(has(
        Language::TypeScript,
        "const f = (a: number): number => a;\n",
        "function",
        "f"
    ));
    assert!(has(
        Language::JavaScript,
        "const g = async function () {};\n",
        "function",
        "g"
    ));
    assert!(has(
        Language::JavaScript,
        "let h = x => x + 1\n",
        "function",
        "h"
    ));
}

#[test]
fn functions_without_bodies_are_prototypes_in_every_language() {
    assert!(has(
        Language::Rust,
        "trait T { fn f(&self); }\n",
        "prototype",
        "f"
    ));
    assert!(has(
        Language::Rust,
        "extern \"C\" { fn abs(x: i32) -> i32; }\n",
        "prototype",
        "abs"
    ));
    assert!(has(Language::C, "int f(int a);\n", "prototype", "f"));
    assert!(has(
        Language::TypeScript,
        "interface I { m(): void; }\n",
        "prototype",
        "m"
    ));
    assert!(has(
        Language::TypeScript,
        "declare function f(): void;\n",
        "prototype",
        "f"
    ));
}

#[test]
fn control_blocks_are_read_through_in_every_language() {
    assert!(has(
        Language::Python,
        "if TYPE_CHECKING:\n    import os\n",
        "import",
        "os"
    ));
    assert!(has(
        Language::Python,
        "try:\n    import a\nexcept ImportError:\n    a = None\n",
        "variable",
        "a"
    ));
    assert!(has(
        Language::C,
        "extern \"C\" {\nint f(void);\n}\n",
        "prototype",
        "f"
    ));
    assert!(has(
        Language::Rust,
        "#[cfg(x)]\nmod m { fn f() {} }\n",
        "function",
        "f"
    ));
}

#[test]
fn every_item_lists_what_it_calls() {
    let found = items(
        Language::Python,
        "def f(x):\n    a = g(x)\n    self.h()\n    os.path.join(a)\n    def inner():\n        hidden()\n",
    );
    assert_eq!(found[0].calls, ["g", "h", "os.path.join"]);
    assert_eq!(found[0].children[0].calls, ["hidden"]);
    let found = items(
        Language::Rust,
        "fn f() { let v = vec![1]; Foo::new(); g(1); }\n",
    );
    assert_eq!(found[0].calls, ["Foo::new", "g", "vec!"]);
    let found = items(
        Language::C,
        "int f(void) { if (x) return g(1); return p->h(); }\n",
    );
    assert_eq!(found[0].calls, ["g", "p->h"]);
    // A call item's callee is its name, not one of its calls.
    let found = items(
        Language::TypeScript,
        "describe(\"a\", () => { it(\"b\", () => {}); });\n",
    );
    assert_eq!(found[0].calls, ["it"]);
}

// Python --------------------------------------------------------------------------------------

#[test]
fn python() {
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
        outline(Language::Python, source),
        [
            item(0, "import", "os", 1, 1),
            item(0, "class", "A", 3, 15),
            item(1, "field", "x", 5, 5),
            item(1, "function", "f", 7, 8),
            item(1, "function", "g", 11, 15),
            item(2, "function", "inner", 13, 14),
            item(0, "function", "h", 18, 23),
            item(0, "variable", "x", 24, 24),
        ]
    );
}

#[test]
fn python_one_liners_and_backslashes() {
    let source = "def f(): return 1\ndef g(a, \\\nb):\n    pass\nclass C: pass\n";
    assert_eq!(
        outline(Language::Python, source),
        [
            item(0, "function", "f", 1, 1),
            item(0, "function", "g", 2, 4),
            item(0, "class", "C", 5, 5),
        ]
    );
}

#[test]
fn python_names_that_are_not_definitions() {
    // `type` is a definition only as `type X = ...`; strings are not code.
    assert_eq!(
        names(
            Language::Python,
            "type = 3\ns = 'def g(): pass'\n# def f():\n"
        ),
        [
            ("variable", "type".to_owned()),
            ("variable", "s".to_owned())
        ]
    );
    assert!(has(
        Language::Python,
        "type Point = tuple[int, int]\n",
        "type",
        "Point"
    ));
}

#[test]
fn docstrings_keep_their_blocks() {
    let source = "class A:\n    \"\"\"Only a docstring.\"\"\"\n\nx = 1\n";
    assert_eq!(
        outline(Language::Python, source)[0],
        item(0, "class", "A", 1, 2)
    );
}

// Rust ----------------------------------------------------------------------------------------

#[test]
fn rust() {
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
enum E { A, B(u8), C { x: u8 } }
";
    assert_eq!(
        outline(Language::Rust, source),
        [
            item(0, "import", "std::fmt", 3, 3),
            item(0, "struct", "A", 5, 9),
            item(1, "field", "x", 8, 8),
            item(0, "struct", "Unit", 12, 12),
            item(0, "struct", "Pair", 13, 13),
            item(0, "function", "make", 15, 18),
            item(1, "function", "helper", 16, 16),
            item(0, "const", "TABLE", 19, 19),
            item(0, "static", "COUNT", 20, 20),
            item(0, "type", "F", 21, 21),
            item(0, "macro", "twice", 22, 24),
            item(0, "union", "U", 25, 25),
            item(1, "field", "a", 25, 25),
            item(0, "enum", "E", 26, 26),
            item(1, "variant", "A", 26, 26),
            item(1, "variant", "B", 26, 26),
            item(1, "variant", "C", 26, 26),
        ]
    );
}

#[test]
fn rust_impls_traits_and_modules() {
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
        outline(Language::Rust, source),
        [
            item(0, "impl", "fmt::Display for Wrapper<'a, T>", 1, 10),
            item(1, "function", "fmt", 5, 9),
            item(0, "impl", "Send for Raw", 12, 12),
            item(0, "trait", "Shape", 14, 18),
            item(1, "const", "SIDES", 15, 15),
            item(1, "prototype", "area", 16, 16),
            item(1, "function", "name", 17, 17),
            item(0, "module", "tests", 20, 26),
            item(1, "import", "super::*", 22, 22),
            item(1, "function", "works", 24, 25),
            item(0, "module", "elsewhere", 27, 27),
            item(0, "prototype", "abs", 29, 29),
            item(0, "call", "thread_local!", 31, 31),
        ]
    );
}

#[test]
fn broken_code_still_reads() {
    let source = "fn open() {\n    if x {\n\nfn next() {}\n";
    assert_eq!(
        outline(Language::Rust, source),
        [
            item(0, "function", "open", 1, 4),
            item(1, "function", "next", 4, 4)
        ]
    );
    assert!(outline(Language::Rust, "impl").is_empty());
    assert_eq!(names(Language::Rust, "macro_rules! m")[0].1, "m");
    assert_eq!(
        names(Language::C, "void f(void) {")[0],
        ("function", "f".to_owned())
    );
}

// C and C++ -----------------------------------------------------------------------------------

#[test]
fn c() {
    let source = "\
#include <stdio.h>
#define MAX(a, b) \\
    ((a) > (b) ? (a) : (b))
#  define  LIMIT 10
#ifdef __cplusplus
extern \"C\" {
#endif

struct point { int x, y; };
typedef struct node_s {
    struct node_s *next;
} node;
typedef enum { RED, GREEN } color;
union { int i; float f; } u;
int table[] = { 1, 2, 3 };
static const char *names[2] = { \"a\", \"b\" };
int prototype(int a);

__attribute__((noreturn))
static void fail(const char *why)
{
    exit(1);
}

struct point *
make_point(int x, int y) { return 0; }
int (*pick(int which))(int) { return 0; }
typedef int (*handler_t)(int);

#ifdef __cplusplus
}
#endif
";
    assert_eq!(
        outline(Language::C, source),
        [
            item(0, "import", "<stdio.h>", 1, 1),
            item(0, "macro", "MAX", 2, 3),
            item(0, "macro", "LIMIT", 4, 4),
            item(0, "struct", "point", 9, 9),
            item(1, "field", "x", 9, 9),
            item(0, "struct", "node", 10, 12),
            item(1, "field", "next", 11, 11),
            item(0, "enum", "color", 13, 13),
            item(1, "variant", "RED", 13, 13),
            item(1, "variant", "GREEN", 13, 13),
            item(0, "union", "u", 14, 14),
            item(1, "field", "i", 14, 14),
            item(1, "field", "f", 14, 14),
            item(0, "variable", "table", 15, 15),
            item(0, "variable", "names", 16, 16),
            item(0, "prototype", "prototype", 17, 17),
            item(0, "function", "fail", 19, 23),
            item(0, "function", "make_point", 25, 26),
            item(0, "function", "pick", 27, 27),
            item(0, "type", "handler_t", 28, 28),
        ]
    );
}

#[test]
fn c_reads_the_first_branch_of_each_if() {
    let source =
        "#ifdef FAST\nint add(int a) { return a; }\n#else\nint add(int a) { return -a; }\n#endif\n";
    assert_eq!(
        outline(Language::C, source),
        [item(0, "function", "add", 2, 2)]
    );
}

#[test]
fn c_macros_around_names() {
    let source = "GIT_INLINE(int) lookup(const char *name) { return 0; }\n\
                  void *\n__NTH (memcpy (void *dest, const void *src, size_t n))\n{ return dest; }\n";
    assert_eq!(
        names(Language::C, source),
        [
            ("function", "lookup".to_owned()),
            ("function", "memcpy".to_owned())
        ]
    );
}

#[test]
fn c_things_that_are_not_items() {
    assert_eq!(
        names(
            Language::C,
            "#undef X\n#definitely\n#define\nint x;\n{ y; }\n"
        ),
        [("variable", "x".to_owned())]
    );
}

#[test]
fn cpp() {
    let source = "\
#pragma once
namespace sundgren::read {
template <typename T>
class Buffer : public Base<T> {
public:
    explicit Buffer(int size) : size_(size), data_(size) {}
    ~Buffer() override;
    T& operator[](std::size_t i) { return data_[i]; }
    bool operator==(const Buffer& other) const;
private:
    int size_;
};
enum class Mode : int { Fast, Slow };
using Bytes = std::vector<unsigned char>;
int Buffer<int>::count() const noexcept { return size_; }
}  // namespace sundgren::read
const char* raw = R\"(a \"quoted\" { brace)\";
";
    assert_eq!(
        outline(Language::Cpp, source),
        [
            item(0, "module", "sundgren::read", 2, 16),
            item(1, "class", "Buffer", 3, 12),
            item(2, "function", "Buffer", 5, 6),
            item(2, "prototype", "~Buffer", 7, 7),
            item(2, "function", "operator[]", 8, 8),
            item(2, "prototype", "operator==", 9, 9),
            item(2, "field", "size_", 10, 11),
            item(1, "enum", "Mode", 13, 13),
            item(2, "variant", "Fast", 13, 13),
            item(2, "variant", "Slow", 13, 13),
            item(1, "type", "Bytes", 14, 14),
            item(1, "function", "Buffer<int>::count", 15, 15),
            item(0, "variable", "raw", 17, 17),
        ]
    );
}

// TypeScript and JavaScript -------------------------------------------------------------------

#[test]
fn typescript() {
    let source = "\
import * as vscode from \"vscode\";
/** The answer. */
export const ANSWER = 42;
const pattern = /\"quoted\"/g;
const label = `count: ${items.length} {braces}`;
export function run(x: number): string {
  function inner() { return helper(x); }
  return inner();
}
export interface Shape {
  area(): number;
  name?: string;
}
export type Id = string | number;
export enum Color { Red, Green = \"g\" }
@Component({ selector: \"x\" })
export class Panel extends Base implements Shape {
  private count = 0;
  constructor(private readonly x: number) { super(); }
  get size() { return 1; }
  onClick = () => { this.count++; };
}
declare module \"foo\" {
  export function bar(): void;
}
namespace Util { export const z = 1 }
let a = 1
let b = 2
";
    assert_eq!(
        outline(Language::TypeScript, source),
        [
            item(0, "import", "vscode", 1, 1),
            item(0, "const", "ANSWER", 2, 3),
            item(0, "const", "pattern", 4, 4),
            item(0, "const", "label", 5, 5),
            item(0, "function", "run", 6, 9),
            item(1, "function", "inner", 7, 7),
            item(0, "interface", "Shape", 10, 13),
            item(1, "prototype", "area", 11, 11),
            item(1, "field", "name", 12, 12),
            item(0, "type", "Id", 14, 14),
            item(0, "enum", "Color", 15, 15),
            item(1, "variant", "Red", 15, 15),
            item(1, "variant", "Green", 15, 15),
            item(0, "class", "Panel", 16, 22),
            item(1, "field", "count", 18, 18),
            item(1, "function", "constructor", 19, 19),
            item(1, "function", "size", 20, 20),
            item(1, "function", "onClick", 21, 21),
            item(0, "module", "foo", 23, 25),
            item(1, "prototype", "bar", 24, 24),
            item(0, "module", "Util", 26, 26),
            item(1, "const", "z", 26, 26),
            item(0, "variable", "a", 27, 27),
            item(0, "variable", "b", 28, 28),
        ]
    );
}

#[test]
fn javascript_names_that_are_not_definitions() {
    // `module.exports`, `type` as a property: names, not definitions.
    assert!(outline(Language::JavaScript, "module.exports = { type: 1 };\n").is_empty());
    assert!(has(
        Language::JavaScript,
        "export default class {}\nexport default function () {}\n",
        "class",
        ""
    ));
}
