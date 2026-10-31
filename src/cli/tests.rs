use super::*;
use crate::test_support::{TempDir, git};

fn graph(rev: &str, repo: &str, max_count: usize) -> Command {
    Command::Graph {
        rev: rev.to_owned(),
        repo: PathBuf::from(repo),
        max_count,
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
    assert_eq!(parse_ok(&["graph"]), graph("HEAD", ".", 1000));
    assert_eq!(
        parse_ok(&["graph", "main", "--repo", "/r", "-n", "5"]),
        graph("main", "/r", 5)
    );
    assert_eq!(
        parse_ok(&["graph", "--max-count=7", "--repo=/r", "--", "-odd"]),
        graph("-odd", "/r", 7)
    );
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
        &["graph", "a", "b"],
        &["graph", "--max-count", "0"],
        &["graph", "-n", "many"],
        &["graph", "--repo"],
        &["graph", "--colour"],
    ] {
        assert_eq!(parse_err(args).code, crate::EXIT_USAGE, "{args:?}");
    }
}

/// main: a - b - m (merges feature: a - f)
fn repo_with_merge() -> TempDir {
    let tmp = TempDir::new();
    let d = &tmp.0;
    git(d, &["init", "-q", "-b", "main"]);
    git(d, &["commit", "-q", "--allow-empty", "-m", "a"]);
    git(d, &["switch", "-q", "-c", "feature"]);
    git(d, &["commit", "-q", "--allow-empty", "-m", "f"]);
    git(d, &["switch", "-q", "main"]);
    git(d, &["commit", "-q", "--allow-empty", "-m", "b"]);
    git(d, &["merge", "-q", "--no-ff", "-m", "m", "feature"]);
    git(d, &["tag", "v1"]);
    tmp
}

#[test]
fn graph_of_a_merge() {
    let tmp = repo_with_merge();
    let doc = graph_document(&tmp.0, "main", 1000).unwrap();
    assert_eq!(doc.schema, GRAPH_SCHEMA);
    assert!(!doc.truncated);
    assert_eq!(doc.columns, 2);
    let subjects: Vec<_> = doc
        .commits
        .iter()
        .map(|n| n.commit.subject.as_str())
        .collect();
    assert_eq!(subjects, ["m", "f", "b", "a"]);
    let columns: Vec<_> = doc.commits.iter().map(|n| n.column).collect();
    assert_eq!(columns, [0, 1, 0, 0]);
    let refs: Vec<_> = doc.commits[0]
        .commit
        .refs
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(refs, ["HEAD", "main", "v1"]);
    assert_eq!(doc.oid, doc.commits[0].commit.oid);
}

#[test]
fn graph_truncates_and_keeps_open_lines() {
    let tmp = repo_with_merge();
    let doc = graph_document(&tmp.0, "main", 2).unwrap();
    assert!(doc.truncated);
    assert_eq!(doc.commits.len(), 2);
    assert_eq!(doc.tail.len(), 2);
}

#[test]
fn graph_of_another_branch_from_a_subdirectory() {
    let tmp = repo_with_merge();
    let sub = tmp.0.join("sub");
    std::fs::create_dir(&sub).unwrap();
    let doc = graph_document(&sub, "feature", 10).unwrap();
    let subjects: Vec<_> = doc
        .commits
        .iter()
        .map(|n| n.commit.subject.as_str())
        .collect();
    assert_eq!(subjects, ["f", "a"]);
    assert_eq!(doc.rev, "feature");
}

#[test]
fn graph_document_serializes_flat() {
    let tmp = repo_with_merge();
    let doc = graph_document(&tmp.0, "HEAD", 10).unwrap();
    let json = serde_json::to_value(&doc).unwrap();
    let first = &json["commits"][0];
    for key in [
        "oid",
        "parents",
        "author",
        "authored",
        "committed",
        "subject",
        "refs",
        "column",
        "color",
        "lines",
    ] {
        assert!(first.get(key).is_some(), "{key} missing");
    }
    assert_eq!(
        first["refs"][0],
        serde_json::json!({"name": "HEAD", "kind": "head"})
    );
}

#[test]
fn graph_errors() {
    let tmp = repo_with_merge();
    assert_eq!(
        graph_document(&tmp.0, "nope", 10).unwrap_err().code,
        crate::EXIT_USAGE
    );
    let empty = TempDir::new();
    let err = graph_document(&empty.0, "HEAD", 10).unwrap_err();
    assert_eq!(err.code, crate::EXIT_FAILURE);
}

#[test]
fn parses_contain() {
    let contain = |fuzzy: f64, format: Format| Command::Contain {
        a: "x".to_owned(),
        b: "y".to_owned(),
        base: "main".to_owned(),
        repo: PathBuf::from("."),
        fuzzy,
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
    for args in [
        &["contain", "x", "--base", "main"][..],
        &["contain", "x", "y"],
        &["contain", "x", "y", "--base", "main", "--fuzzy", "1.5"],
        &["contain", "x", "y", "--base", "main", "--format", "yaml"],
        &["contain", "x", "y", "--base", "main", "-n", "3"],
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
