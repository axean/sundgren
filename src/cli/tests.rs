use std::process::Command as Process;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

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

/// A temporary directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sundgren-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Process::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Ada", "-c", "user.email=ada@example.com"])
        .args(["-c", "commit.gpgSign=false", "-c", "tag.gpgSign=false"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
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
