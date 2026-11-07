use super::*;

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

/// "kind name" of each item, indented by depth.
fn names(language: Language, source: &str) -> Vec<String> {
    outline(language, source)
        .into_iter()
        .map(|(depth, kind, name, _, _)| format!("{}{kind} {name}", "  ".repeat(depth)))
        .collect()
}

#[test]
fn languages_from_extensions() {
    assert_eq!(Language::from_path("a/b.py"), Some(Language::Python));
    assert_eq!(Language::from_path("b.pyi"), Some(Language::Python));
    assert_eq!(Language::from_path("src/x.rs"), Some(Language::Rust));
    assert_eq!(Language::from_path("x.c"), Some(Language::C));
    assert_eq!(Language::from_path("x.h"), Some(Language::C));
    assert_eq!(Language::from_path("x.cpp"), Some(Language::Cpp));
    assert_eq!(Language::from_path("x.ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_path("x.tsx"), Some(Language::Tsx));
    assert_eq!(Language::from_path("x.mjs"), Some(Language::JavaScript));
    assert_eq!(Language::from_path("x.go"), None);
    assert_eq!(Language::from_path("v1.2/Makefile"), None);
    assert_eq!(Language::Tsx.name(), "typescript");
}

#[test]
fn every_language_reads() {
    // Each grammar and query loads, and finds a function.
    let cases = [
        (Language::Python, "def f():\n    pass\n"),
        (Language::Rust, "fn f() {}\n"),
        (Language::C, "int f(void) { return 0; }\n"),
        (Language::Cpp, "int f() { return 0; }\n"),
        (Language::JavaScript, "function f() {}\n"),
        (Language::TypeScript, "function f(): void {}\n"),
        (Language::Tsx, "function f() { return <div />; }\n"),
    ];
    for (language, source) in cases {
        assert_eq!(names(language, source), ["function f"], "{language:?}");
    }
}

// The same rules for every language -----------------------------------------------------------

#[test]
fn inside_functions_only_definitions() {
    let cases = [
        (
            Language::Python,
            "def f():\n    x = 1\n    g = lambda: 2\n    import os\n    def inner():\n        pass\n",
        ),
        (
            Language::Rust,
            "fn f() {\n    let x = 1;\n    use std::fmt;\n    fn inner() {}\n}\n",
        ),
        (
            Language::Cpp,
            "void f() {\n    int x = 1;\n    struct inner { int a; };\n}\n",
        ),
        (
            Language::TypeScript,
            "function f() {\n    const x = 1;\n    const g = () => 2;\n    function inner() {}\n}\n",
        ),
    ];
    for (language, source) in cases {
        let found = names(language, source);
        let children: Vec<&String> = found
            .iter()
            .filter(|n| n.starts_with("  ") && !n.starts_with("    "))
            .collect();
        assert_eq!(found[0], "function f", "{language:?}");
        assert_eq!(children.len(), 1, "{language:?}: {found:?}");
        assert!(children[0].ends_with(" inner"), "{language:?}: {found:?}");
    }
}

#[test]
fn anonymous_functions_are_inside_a_function() {
    let source = "var api = (function () {\n    var helper = function () {};\n    function named() {}\n})();\n";
    assert_eq!(
        names(Language::JavaScript, source),
        ["variable api", "function named"]
    );
}

#[test]
fn values_hold_no_items() {
    // What's inside a value belongs to the value's parent.
    let source = "const _: () = {\n    struct Inner;\n};\n";
    assert_eq!(names(Language::Rust, source), ["const _", "struct Inner"]);
}

#[test]
fn variables_in_classes_are_fields() {
    assert_eq!(
        names(Language::Python, "class A:\n    x = 1\n"),
        ["class A", "  field x"]
    );
}

#[test]
fn functions_as_values() {
    assert_eq!(names(Language::Python, "f = lambda x: x\n"), ["function f"]);
    assert_eq!(
        names(
            Language::TypeScript,
            "export const f = async () => {};\nconst C = class {};\n"
        ),
        ["function f", "class C"]
    );
}

#[test]
fn top_level_calls_are_items() {
    assert_eq!(names(Language::Python, "main()\n"), ["call main"]);
    assert_eq!(names(Language::JavaScript, "run();\n"), ["call run"]);
    assert_eq!(
        names(Language::Rust, "thread_local! { }\n"),
        ["call thread_local!"]
    );
}

#[test]
fn imports() {
    let cases: [(Language, &str, &[&str]); 5] = [
        (
            Language::Python,
            "import os\nfrom a.b import c\n",
            &["import os", "import a.b"],
        ),
        (Language::Rust, "use a::{b, c};\n", &["import a::{b, c}"]),
        (
            Language::C,
            "#include <stdio.h>\n#include \"x.h\"\n",
            &["import <stdio.h>", "import x.h"],
        ),
        (
            Language::Cpp,
            "using std::vector;\n",
            &["import std::vector"],
        ),
        (
            Language::TypeScript,
            "import * as x from \"vscode\";\nexport { a } from './a.js';\n",
            &["import vscode", "import ./a.js"],
        ),
    ];
    for (language, source, expected) in cases {
        assert_eq!(names(language, source), expected, "{language:?}");
    }
}

#[test]
fn comments_and_attributes_before_an_item_belong_to_it() {
    let rust = "//! The module.\n\n/// Doc.\n#[derive(Debug)]\nstruct A;\n// Not doc.\nfn f() {}\n";
    assert_eq!(
        outline(Language::Rust, rust),
        [
            (0, "struct", "A".into(), 3, 5),
            (0, "function", "f".into(), 7, 7)
        ]
    );
    let ts = "/** Doc. */\nexport function f() {}\nclass C {\n  @dec\n  m() {}\n}\n";
    assert_eq!(
        outline(Language::TypeScript, ts),
        [
            (0, "function", "f".into(), 1, 2),
            (0, "class", "C".into(), 3, 6),
            (1, "function", "m".into(), 4, 5),
        ]
    );
    let python = "@decorator\ndef f():\n    pass\n";
    assert_eq!(
        outline(Language::Python, python),
        [(0, "function", "f".into(), 1, 3)]
    );
}

#[test]
fn broken_code_still_reads() {
    assert_eq!(
        names(Language::Python, "def f(:\n    pass\ndef g():\n    pass\n")
            .last()
            .unwrap(),
        "function g"
    );
    // An unclosed function is lost, what follows it isn't.
    assert_eq!(
        names(
            Language::Rust,
            "fn f() {\n    let x = 1;\n\nfn g() {}\nstruct S;\n"
        ),
        ["function g", "struct S"]
    );
}

// Per language ---------------------------------------------------------------------------------

#[test]
fn python() {
    let source = "\"\"\"Doc.\"\"\"\nX = 1\n\nclass A(B):\n    \"\"\"Doc.\"\"\"\n    y = 2\n\n    async def m(self):\n        pass\n\ntype T = int\n";
    assert_eq!(
        names(Language::Python, source),
        [
            "variable X",
            "class A",
            "  field y",
            "  function m",
            "type T"
        ]
    );
}

#[test]
fn rust() {
    let source = "pub mod m {\n    pub trait T { fn f(&self); type Out; }\n}\nimpl<T> Display for Wrapper<T> {\n    fn fmt(&self) {}\n}\nenum E { A, B(u8) }\nstruct S { x: i32 }\nunion U { a: u8 }\nconst N: u8 = 1;\nstatic S2: u8 = 1;\ntype Alias = u8;\nmacro_rules! m { () => {} }\n";
    assert_eq!(
        names(Language::Rust, source),
        [
            "module m",
            "  trait T",
            "    prototype f",
            "    type Out",
            "impl Display for Wrapper<T>",
            "  function fmt",
            "enum E",
            "  variant A",
            "  variant B",
            "struct S",
            "  field x",
            "union U",
            "  field a",
            "const N",
            "static S2",
            "type Alias",
            "macro m",
        ]
    );
}

#[test]
fn c() {
    let source = "#define MAX 3\n#define SQ(x) ((x) * (x))\nstruct point { int x; int *y; };\ntypedef struct { int a; } pair;\ntypedef unsigned long size;\nenum color { RED, GREEN };\nint count = 0;\nstatic int *make(void);\nchar *name(int i) { return 0; }\n";
    assert_eq!(
        names(Language::C, source),
        [
            "macro MAX",
            "macro SQ",
            "struct point",
            "  field x",
            "  field y",
            "struct pair",
            "  field a",
            "type size",
            "enum color",
            "  variant RED",
            "  variant GREEN",
            "variable count",
            "prototype make",
            "function name",
        ]
    );
}

#[test]
fn cpp() {
    let source = "namespace ns {\nclass A : public B {\npublic:\n    A(int x);\n    ~A();\n    int get() const { return 0; }\n    int value;\n};\n}\nint ns::A::twice(int x) { return x; }\nusing Size = unsigned;\n";
    assert_eq!(
        names(Language::Cpp, source),
        [
            "module ns",
            "  class A",
            "    prototype A",
            "    prototype ~A",
            "    function get",
            "    field value",
            "function ns::A::twice",
            "type Size",
        ]
    );
}

#[test]
fn typescript() {
    let source = "interface I { a: string; m(): void; }\nenum E { A, B = 2 }\ntype T = string;\nabstract class C {\n  private x = 1;\n  #p = 2;\n  handler = () => {};\n  abstract m(): void;\n  get v() { return 1; }\n}\ndeclare module \"m\" { export function d(): void; }\nnamespace N { export const k = 1; }\nlet y = 2;\n";
    assert_eq!(
        names(Language::TypeScript, source),
        [
            "interface I",
            "  field a",
            "  prototype m",
            "enum E",
            "  variant A",
            "  variant B",
            "type T",
            "class C",
            "  field x",
            "  field #p",
            "  function handler",
            "  prototype m",
            "  function v",
            "module m",
            "  prototype d",
            "module N",
            "  const k",
            "variable y",
        ]
    );
}

#[test]
fn javascript() {
    let source = "class A {\n  x = 1;\n  static f() {}\n}\nmodule.exports = { create(context) {} };\nfunction* gen() {}\n";
    assert_eq!(
        names(Language::JavaScript, source),
        ["class A", "  field x", "  function f", "function gen"]
    );
}

// Tokens ---------------------------------------------------------------------------------------

fn token_kinds(language: Language, source: &str, kind: Kind) -> Vec<&str> {
    tokens(language, source)
        .into_iter()
        .filter(|t| t.kind == kind)
        .map(|t| t.text(source))
        .collect()
}

#[test]
fn tokens_by_kind() {
    let python = "\"\"\"Doc.\"\"\"\nx = f'a#b' + 2  # yes\n";
    assert_eq!(
        token_kinds(Language::Python, python, Kind::Comment),
        ["\"\"\"Doc.\"\"\"", "# yes"]
    );
    assert_eq!(
        token_kinds(Language::Python, python, Kind::String),
        ["f'a#b'"]
    );
    assert_eq!(token_kinds(Language::Python, python, Kind::Number), ["2"]);
    assert_eq!(token_kinds(Language::Python, python, Kind::Ident), ["x"]);

    let c = "#include <a.h>\nint a = 1; /* c */\n";
    assert_eq!(
        token_kinds(Language::C, c, Kind::Directive),
        ["#include <a.h>\n"]
    );
    assert_eq!(token_kinds(Language::C, c, Kind::Comment), ["/* c */"]);

    let ts = "const r = /a+/g, t = `x${y}`, $el = this.#p;\n";
    assert_eq!(
        token_kinds(Language::TypeScript, ts, Kind::String),
        ["/a+/g", "`x${y}`"]
    );
    assert!(token_kinds(Language::TypeScript, ts, Kind::Ident).contains(&"$el"));
}

#[test]
fn tokens_span_lines() {
    let source = "a = '''x\ny'''\nb = 1\n";
    let t: Vec<(Kind, usize, usize)> = tokens(Language::Python, source)
        .iter()
        .map(|t| (t.kind, t.line, t.end_line))
        .collect();
    assert_eq!(t[2], (Kind::String, 1, 2));
    assert_eq!(t[3], (Kind::Ident, 3, 3));
}
