use super::*;
use crate::test_support::TempDir;

fn graph(repo: &str, options: map::Options) -> Command {
    Command::Graph {
        repo: PathBuf::from(repo),
        options,
        format: Format::Json,
    }
}

fn defaults() -> map::Options {
    map::Options {
        base: String::new(),
        show: map::Show::Local,
        branches: 12,
        commits: 50,
        protected: vec!["main".to_owned(), "master".to_owned()],
        pins: Vec::new(),
        hides: Vec::new(),
        since: None,
        until: None,
        threshold: 0.9,
    }
}

fn parse_ok(args: &[&str]) -> Command {
    parse(args.iter().copied()).unwrap()
}

fn parse_err(args: &[&str]) -> Error {
    parse(args.iter().copied()).unwrap_err()
}

#[test]
fn parses_graph() {
    assert_eq!(parse_ok(&["graph"]), graph(".", defaults()));
    let args = [
        "graph",
        "--repo",
        "/r",
        "--base",
        "dev",
        "--like=feat",
        "--branches",
        "5",
        "--commits",
        "70",
        "--protected",
        "release/*",
        "--protected",
        "main",
        "--pin",
        "a",
        "--pin",
        "b",
        "--fuzzy",
        "1",
    ];
    let options = map::Options {
        base: "dev".to_owned(),
        show: map::Show::Like("feat".to_owned()),
        branches: 5,
        commits: 70,
        protected: vec!["release/*".to_owned(), "main".to_owned()],
        pins: vec!["a".to_owned(), "b".to_owned()],
        hides: Vec::new(),
        since: None,
        until: None,
        threshold: 1.0,
    };
    assert_eq!(parse_ok(&args), graph("/r", options));
    for (mode, show) in [
        ("local", map::Show::Local),
        ("recent", map::Show::Recent),
        ("mine", map::Show::Mine),
    ] {
        let Command::Graph { options, .. } = parse_ok(&["graph", "--show", mode]) else {
            panic!("not graph");
        };
        assert_eq!(options.show, show);
    }
    let Command::Graph { format, .. } = parse_ok(&["graph", "--format", "text"]) else {
        panic!("not graph");
    };
    assert_eq!(format, Format::Text);
}

#[test]
fn parses_help_and_version() {
    assert_eq!(parse_ok(&[]), Command::Help);
    assert_eq!(parse_ok(&["--help"]), Command::Help);
    assert_eq!(parse_ok(&["graph", "-h"]), Command::Help);
    assert_eq!(parse_ok(&["-V"]), Command::Version);
}

#[test]
fn rejects_bad_arguments() {
    for args in [
        &["log"][..],
        &["graph", "main"],
        &["graph", "--commits", "0"],
        &["graph", "--show", "like"],
        &["graph", "--show", "recent", "--like", "x"],
        &["graph", "--show", "all"],
        &["graph", "--branches", "many"],
        &["graph", "--fuzzy", "2"],
        &["graph", "--repo"],
        &["graph", "--colour"],
    ] {
        assert_eq!(parse_err(args).code, crate::EXIT_USAGE, "{args:?}");
    }
}

#[test]
fn parses_contain() {
    let contain = |fuzzy: f64, format: Format| Command::Contain {
        a: "x".to_owned(),
        b: "y".to_owned(),
        base: "main".to_owned(),
        repo: PathBuf::from("."),
        fuzzy,
        comments: Comments::Ignore,
        lines: false,
        format,
    };
    assert_eq!(
        parse_ok(&["contain", "x", "y", "--base", "main"]),
        contain(0.9, Format::Json)
    );
    assert_eq!(
        parse_ok(&[
            "contain",
            "--base=main",
            "x",
            "y",
            "--fuzzy",
            "1",
            "--format",
            "text"
        ]),
        contain(1.0, Format::Text)
    );
    assert_eq!(
        parse_ok(&[
            "contain",
            "x",
            "y",
            "--base",
            "main",
            "--comments",
            "count",
            "--lines"
        ]),
        Command::Contain {
            a: "x".to_owned(),
            b: "y".to_owned(),
            base: "main".to_owned(),
            repo: PathBuf::from("."),
            fuzzy: 0.9,
            comments: Comments::Count,
            lines: true,
            format: Format::Json,
        }
    );
    for args in [
        &["contain", "x", "--base", "main"][..],
        &["contain", "x", "y"],
        &["contain", "x", "y", "--base", "main", "--fuzzy", "1.5"],
        &["contain", "x", "y", "--base", "main", "--format", "yaml"],
        &["contain", "x", "y", "--base", "main", "-n", "3"],
        &["contain", "x", "y", "--base", "main", "--comments", "some"],
    ] {
        assert_eq!(parse_err(args).code, crate::EXIT_USAGE, "{args:?}");
    }
}

#[test]
fn parses_outline() {
    assert_eq!(
        parse_ok(&["outline", "a.py", "b.rs"]),
        Command::Outline {
            files: vec![PathBuf::from("a.py"), PathBuf::from("b.rs")],
            tokens: false,
            format: Format::Text,
        }
    );
    assert_eq!(
        parse_ok(&["outline", "--tokens", "a.c", "--format", "json"]),
        Command::Outline {
            files: vec![PathBuf::from("a.c")],
            tokens: true,
            format: Format::Json,
        }
    );
    assert_eq!(parse_ok(&["outline", "--help"]), Command::Help);
    assert_eq!(parse_err(&["outline"]).code, crate::EXIT_USAGE);
    assert_eq!(
        parse_err(&["outline", "a.py", "--format", "xml"]).code,
        crate::EXIT_USAGE
    );
    // A switch takes no value.
    assert_eq!(
        parse_err(&["outline", "a.py", "--tokens=yes"]).code,
        crate::EXIT_USAGE
    );
}

#[test]
fn outlines_files() {
    let dir = TempDir::new();
    let python = dir.0.join("m.py");
    std::fs::write(
        &python,
        "class A:\n    def f(self):\n        return 'x'  # y\n",
    )
    .unwrap();
    let other = dir.0.join("notes.txt");
    std::fs::write(&other, "def g(): pass\n").unwrap();

    let doc = outline_document(&[python.clone(), other.clone()], true).unwrap();
    assert_eq!(doc.schema, OUTLINE_SCHEMA);
    assert_eq!(doc.files[0].language, Some(Language::Python));
    assert_eq!(doc.files[0].items[0].children[0].name, "f");
    assert_eq!(doc.files[1].language, None);
    assert!(doc.files[1].items.is_empty());

    let text = outline_text(&doc);
    assert!(text.contains("m.py (python)\n  1-3        class A\n    2-3        function f\n"));
    assert!(text.contains("      3  return [string 'x'] [comment # y]\n"));
    assert!(text.contains("notes.txt: not read (unknown language)\n"));

    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(json["files"][0]["language"], "python");
    assert_eq!(json["files"][0]["tokens"][0]["kind"], "ident");
    assert!(json["files"][1].get("language").is_none());

    let missing = dir.0.join("missing.rs");
    assert_eq!(
        outline_document(&[missing], false).unwrap_err().code,
        crate::EXIT_FAILURE
    );
    run(&Command::Outline {
        files: vec![python],
        tokens: false,
        format: Format::Json,
    })
    .unwrap();
}

#[test]
fn long_tokens_are_shortened() {
    let token = TokenOut {
        kind: Kind::String,
        line: 1,
        end_line: 2,
        text: format!("\"{}\nb\"", "a".repeat(50)),
    };
    let shown = show_token(&token);
    assert!(shown.starts_with("[string \"aaa"));
    assert!(shown.ends_with("…]"));
    assert!(!shown.contains('\n'));
}
