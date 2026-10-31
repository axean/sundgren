use super::*;

#[test]
fn languages_from_extensions() {
    assert_eq!(Language::from_path("a/b.py"), Some(Language::Python));
    assert_eq!(Language::from_path("b.pyi"), Some(Language::Python));
    assert_eq!(Language::from_path("src/x.rs"), Some(Language::Rust));
    assert_eq!(Language::from_path("x.c"), Some(Language::C));
    assert_eq!(Language::from_path("x.h"), Some(Language::C));
    assert_eq!(Language::from_path("x.cpp"), None);
    assert_eq!(Language::from_path("v1.2/Makefile"), None);
    assert_eq!(Language::C.name(), "c");
}

#[test]
fn brackets_match_and_unbalanced_ones_do_not() {
    let source = "f(a[1], {b}) )";
    let tokens = lex(Language::Rust, source);
    let m = matching(source, &tokens);
    let text: Vec<&str> = tokens.iter().map(|t| t.text(source)).collect();
    assert_eq!(text[1], "(");
    assert_eq!(m[1], Some(10));
    assert_eq!(m[3], Some(5));
    assert_eq!(m[11], None);
}

#[test]
fn join_collapses_whitespace() {
    let source = "Display  for\n Error<'a,  T>";
    let tokens = lex(Language::Rust, source);
    assert_eq!(join(source, &tokens), "Display for Error<'a, T>");
}

#[test]
fn items_by_language() {
    assert_eq!(items(Language::Python, "def f():\n    pass\n")[0].name, "f");
    assert_eq!(items(Language::Rust, "fn f() {}\n")[0].name, "f");
    assert_eq!(
        items(Language::C, "int f(void) { return 0; }\n")[0].name,
        "f"
    );
}
