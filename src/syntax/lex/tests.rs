use super::*;

/// (kind, text) of each token.
fn tokens(language: Language, source: &str) -> Vec<(Kind, &str)> {
    lex(language, source)
        .iter()
        .map(|t| (t.kind, t.text(source)))
        .collect()
}

fn of_kind(language: Language, source: &str, kind: Kind) -> Vec<&str> {
    tokens(language, source)
        .into_iter()
        .filter(|&(k, _)| k == kind)
        .map(|(_, text)| text)
        .collect()
}

#[test]
fn python_strings_and_comments() {
    let source = "x = f'a#b' + \"\"\"two # not\nlines\"\"\"  # yes\nr'\\'' b\"\"";
    assert_eq!(
        of_kind(Language::Python, source, Kind::String),
        ["f'a#b'", "\"\"\"two # not\nlines\"\"\"", "r'\\''", "b\"\""]
    );
    assert_eq!(of_kind(Language::Python, source, Kind::Comment), ["# yes"]);
    // A name that only looks like a prefix.
    assert_eq!(
        tokens(Language::Python, "rb = fr"),
        [(Kind::Ident, "rb"), (Kind::Punct, "="), (Kind::Ident, "fr")]
    );
}

#[test]
fn lines_of_tokens() {
    let source = "a = '''x\ny'''\nb = 1 + \\\n    2\n";
    let lexed = lex(Language::Python, source);
    let lines: Vec<(&str, usize, usize)> = lexed
        .iter()
        .map(|t| (t.text(source), t.line, t.end_line))
        .collect();
    assert_eq!(lines[2], ("'''x\ny'''", 1, 2));
    assert_eq!(lines[3], ("b", 3, 3));
    assert_eq!(lines[7], ("2", 4, 4));
    assert_eq!(lines.len(), 8);
}

#[test]
fn rust_lifetimes_chars_and_raw_strings() {
    let source = "fn f<'a>(x: &'a str) -> char { let c = '\\''; let d = 'é'; 'outer: loop {} }";
    assert_eq!(
        of_kind(Language::Rust, source, Kind::String),
        ["'\\''", "'é'"]
    );
    let idents = of_kind(Language::Rust, source, Kind::Ident);
    assert!(idents.contains(&"'a"));
    assert!(idents.contains(&"'outer"));

    let source = r###"r#"a "quoted" b"# br"x" b'\n' c"y" r#type "\"" "two
lines""###;
    assert_eq!(
        of_kind(Language::Rust, source, Kind::String),
        [
            r##"r#"a "quoted" b"#"##,
            r#"br"x""#,
            r"b'\n'",
            r#"c"y""#,
            r#""\"""#,
            "\"two\nlines\""
        ]
    );
    assert_eq!(of_kind(Language::Rust, source, Kind::Ident), ["r#type"]);
}

#[test]
fn rust_comments_nest() {
    let source = "a /* x /* y */ z */ b // c\n/// doc\nd";
    assert_eq!(
        of_kind(Language::Rust, source, Kind::Comment),
        ["/* x /* y */ z */", "// c", "/// doc"]
    );
    assert_eq!(
        of_kind(Language::Rust, source, Kind::Ident),
        ["a", "b", "d"]
    );
}

#[test]
fn numbers() {
    let source = "1..2 3.5e-7 0x1e-5 1_000u32 x.0";
    assert_eq!(
        of_kind(Language::Rust, source, Kind::Number),
        ["1", "2", "3.5e-7", "0x1e", "5", "1_000u32", "0"]
    );
}

#[test]
fn c_directives_comments_and_strings() {
    let source = "#include <a.h> // why\n  # define TWO(x) \\\n    (x + x) /* twice */\nint a = b # c;\nchar *s = L\"wide\", c = '\\'';\n/* one\n /* two */ x";
    assert_eq!(
        of_kind(Language::C, source, Kind::Directive),
        ["#include <a.h> ", "# define TWO(x) \\\n    (x + x) "]
    );
    assert_eq!(
        of_kind(Language::C, source, Kind::Comment),
        ["// why", "/* twice */", "/* one\n /* two */"]
    );
    assert_eq!(
        of_kind(Language::C, source, Kind::String),
        ["L\"wide\"", "'\\''"]
    );
    assert!(of_kind(Language::C, source, Kind::Punct).contains(&"#"));
    let lexed = lex(Language::C, source);
    let x = lexed.last().unwrap();
    assert_eq!((x.text(source), x.line), ("x", 7));
}

#[test]
fn unterminated_input_still_lexes() {
    assert_eq!(of_kind(Language::C, "\"abc\nx", Kind::String), ["\"abc"]);
    assert_eq!(
        of_kind(Language::Rust, "/* open", Kind::Comment),
        ["/* open"]
    );
    assert_eq!(
        of_kind(Language::Python, "'''open", Kind::String),
        ["'''open"]
    );
    assert_eq!(
        of_kind(Language::Rust, "r#\"open", Kind::String),
        ["r#\"open"]
    );
    assert_eq!(
        tokens(Language::Python, "a \\\r\n b"),
        [(Kind::Ident, "a"), (Kind::Ident, "b")]
    );
}

#[test]
fn non_ascii_identifiers() {
    assert_eq!(
        of_kind(Language::Python, "π = größe·2", Kind::Ident),
        ["π", "größe·2"]
    );
}
