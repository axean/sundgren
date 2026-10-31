use super::*;
use crate::test_support::{self, TempDir, commit_file};

fn record(oid: &str, parents: &str, subject: &str, refs: &str) -> String {
    format!("{oid}\x1f{parents}\x1fAda\x1fada@example.com\x1f100\x1f200\x1f{subject}\x1f{refs}")
}

#[test]
fn parses_records() {
    let out = format!(
        "{}\0\n{}\0",
        record("c2", "c1 c0", "Merge it", "HEAD -> refs/heads/main"),
        record("c1", "", "Root, with \u{e9}", ""),
    );
    let commits = parse_log(&out).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].oid, "c2");
    assert_eq!(commits[0].parents, ["c1", "c0"]);
    assert_eq!(commits[0].author.name, "Ada");
    assert_eq!(commits[0].authored, 100);
    assert_eq!(commits[0].committed, 200);
    assert_eq!(commits[1].parents, Vec::<String>::new());
    assert_eq!(commits[1].subject, "Root, with \u{e9}");
    assert_eq!(commits[1].refs, []);
}

#[test]
fn rejects_malformed_records() {
    assert!(parse_log("c1\x1fonly two\0").is_err());
    let bad_time = "c\x1f\x1fA\x1fa@b\x1fsoon\x1f1\x1fs\x1f\0";
    assert!(parse_log(bad_time).is_err());
}

#[test]
fn classifies_refs() {
    let refs = parse_refs(
        "HEAD -> refs/heads/main, tag: refs/tags/v1.0, refs/remotes/origin/main, \
         refs/heads/feature/x, refs/stash",
    );
    let got: Vec<_> = refs.iter().map(|r| (r.name.as_str(), r.kind)).collect();
    assert_eq!(
        got,
        [
            ("HEAD", RefKind::Head),
            ("main", RefKind::Branch),
            ("v1.0", RefKind::Tag),
            ("origin/main", RefKind::Remote),
            ("feature/x", RefKind::Branch),
            ("stash", RefKind::Other),
        ]
    );
}

#[test]
fn detached_head() {
    let refs = parse_refs("HEAD");
    assert_eq!(refs, [head()]);
}

#[test]
fn parses_added_lines() {
    let diff = "\
diff --git a/src/a.py b/src/a.py
index 1..2 100644
--- a/src/a.py
+++ b/src/a.py
@@ -3,0 +4,2 @@ def f():
+    x = 1
+    y = 2
@@ -10 +12 @@ class C:
-    old
+    new
diff --git a/gone.py b/gone.py
deleted file mode 100644
--- a/gone.py
+++ /dev/null
@@ -1 +0,0 @@
-bye
diff --git a/new.txt b/new.txt
new file mode 100644
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
++plus at the start
++++ b/looks like a header
diff --git a/only_removed.py b/only_removed.py
--- a/only_removed.py
+++ b/only_removed.py
@@ -1 +0,0 @@
-x
Binary files a/img.png and b/img.png differ
";
    let files = parse_added(diff);
    let got: Vec<_> = files
        .iter()
        .map(|f| (f.path.as_str(), f.added.clone()))
        .collect();
    assert_eq!(
        got,
        [
            (
                "src/a.py",
                vec![
                    (4, "    x = 1".to_owned()),
                    (5, "    y = 2".to_owned()),
                    (12, "    new".to_owned()),
                ]
            ),
            (
                "new.txt",
                vec![
                    (1, "+plus at the start".to_owned()),
                    (2, "+++ b/looks like a header".to_owned())
                ]
            ),
        ]
    );
}

#[test]
fn reads_files_at_a_commit() {
    let tmp = TempDir::new();
    let d = &tmp.0;
    test_support::git(d, &["init", "-q", "-b", "main"]);
    commit_file(d, "a b.py", "x = 1\n", "a");
    commit_file(d, "dir/c.rs", "fn c() {}", "c");
    let oid = resolve(d, "HEAD").unwrap();
    let files = files_at(d, &oid, &["a b.py", "missing.py", "dir", "dir/c.rs"]).unwrap();
    assert_eq!(
        files,
        [
            Some(b"x = 1\n".to_vec()),
            None,
            None,
            Some(b"fn c() {}".to_vec())
        ]
    );
    assert!(files_at(d, &oid, &[]).unwrap().is_empty());
}

#[test]
fn bad_batch_output_is_an_error() {
    assert!(parse_batch(b"abc blob 10\nshort", 1).is_err());
    assert!(parse_batch(b"abc blob x\n", 1).is_err());
    assert!(parse_batch(b"", 1).is_err());
    assert_eq!(parse_batch(b"abc missing\n", 1).unwrap(), [None]);
}

#[test]
fn counts_commits_between() {
    let tmp = TempDir::new();
    let d = &tmp.0;
    test_support::git(d, &["init", "-q", "-b", "main"]);
    commit_file(d, "a", "1", "one");
    let first = resolve(d, "HEAD").unwrap();
    commit_file(d, "a", "2", "two");
    commit_file(d, "a", "3", "three");
    let last = resolve(d, "HEAD").unwrap();
    assert_eq!(commit_count(d, &first, &last).unwrap(), 2);
    assert_eq!(commit_count(d, &last, &first).unwrap(), 0);
    assert!(commit_count(d, "nope", &last).is_err());
}
