use super::*;
use crate::test_support::{TempDir, commit_file, git};

/// Numbered lines of plausible code for feature `name`: `count` distinct lines.
fn code(name: &str, count: usize) -> String {
    (0..count)
        .map(|i| format!("    {name}_value_{i} = compute_{name}(step={i}, scale=0.{i})\n"))
        .collect()
}

/// main has app.py. dev adds feature one (app.py) and feature two (two.py).
fn repo() -> TempDir {
    let tmp = TempDir::new();
    let d = &tmp.0;
    git(d, &["init", "-q", "-b", "main"]);
    commit_file(d, "app.py", "def main():\n    pass\n", "start");
    git(d, &["switch", "-q", "-c", "dev"]);
    commit_file(
        d,
        "app.py",
        &format!("def main():\n    pass\n{}", code("one", 10)),
        "one",
    );
    commit_file(d, "two.py", &code("two", 10), "two");
    git(d, &["switch", "-q", "main"]);
    tmp
}

fn doc(tmp: &TempDir, a: &str, b: &str, threshold: f64) -> ContainmentDocument {
    containment_document(&tmp.0, "main", a, b, threshold).unwrap()
}

#[test]
fn an_extraction_is_in_its_dev_branch() {
    let tmp = repo();
    let d = &tmp.0;
    git(d, &["switch", "-q", "-c", "extract", "main"]);
    commit_file(
        d,
        "app.py",
        &format!("def main():\n    pass\n{}", code("one", 10)),
        "one",
    );
    let doc = doc(&tmp, "extract", "dev", 2.0);
    assert_eq!(doc.a.lines, 10);
    assert_eq!(doc.b.lines, 20);
    assert_eq!(doc.a_in_b.containment, Some(1.0));
    assert_eq!(doc.b_in_a.containment, Some(0.5));
    assert_eq!(doc.a_in_b.ordered_containment, Some(1.0));
    assert_eq!(doc.relation, "a-in-b");
    let file = |path: &str, exact| FileShare {
        path: path.to_owned(),
        lines: 10,
        exact,
        fuzzy: 0,
    };
    assert_eq!(doc.b_in_a.files, [file("app.py", 10), file("two.py", 0)]);
}

#[test]
fn a_rebased_branch_is_the_same_work() {
    let tmp = repo();
    let d = &tmp.0;
    commit_file(d, "other.py", "unrelated = True\n", "main moves on");
    git(d, &["branch", "rebased", "dev"]);
    git(d, &["rebase", "-q", "main", "rebased"]);
    let doc = doc(&tmp, "rebased", "dev", 2.0);
    // Each is measured from where it left main, so main's own commit is in neither.
    assert_eq!(doc.a.lines, 20);
    assert_eq!(doc.relation, "same");
    assert_ne!(doc.a.fork, doc.b.fork);
}

#[test]
fn review_edits_are_found_fuzzily() {
    let tmp = repo();
    let d = &tmp.0;
    git(d, &["switch", "-q", "-c", "extract", "main"]);
    let edited = code("one", 10).replace("scale=", "scale = ");
    commit_file(
        d,
        "app.py",
        &format!("def main():\n    pass\n{edited}"),
        "one, reviewed",
    );
    let exact = doc(&tmp, "extract", "dev", 2.0);
    assert_eq!(exact.a_in_b.exact, 0);
    assert_eq!(exact.relation, "unrelated");
    let fuzzy = doc(&tmp, "extract", "dev", 0.9);
    assert_eq!(fuzzy.a_in_b.fuzzy, 10);
    assert_eq!(fuzzy.a_in_b.containment, Some(1.0));
    assert_eq!(fuzzy.relation, "a-in-b");
}

#[test]
fn unrelated_work_and_empty_branches() {
    let tmp = repo();
    let d = &tmp.0;
    git(d, &["switch", "-q", "-c", "other", "main"]);
    let three = code("three", 10).replace("compute", "fetch");
    commit_file(d, "three.py", &three, "three");
    assert_eq!(doc(&tmp, "other", "dev", 0.9).relation, "unrelated");
    let empty = doc(&tmp, "main", "dev", 0.9);
    assert_eq!(empty.a.lines, 0);
    assert_eq!(empty.a_in_b.containment, None);
    assert_eq!(empty.relation, "empty");
}

#[test]
fn relation_thresholds() {
    assert_eq!(relation(Some(0.8), Some(0.8)), "same");
    assert_eq!(relation(Some(0.9), Some(0.1)), "a-in-b");
    assert_eq!(relation(Some(0.1), Some(0.9)), "b-in-a");
    assert_eq!(relation(Some(0.2), Some(0.0)), "overlap");
    assert_eq!(relation(Some(0.19), Some(0.1)), "unrelated");
    assert_eq!(relation(None, Some(1.0)), "empty");
}

#[test]
fn text_summary() {
    let tmp = repo();
    let text = text(&doc(&tmp, "dev", "dev", 0.9));
    assert!(
        text.contains("A in B: 100% (exact 100%, in order 100%): 20 exact + 0 fuzzy of 20 lines")
    );
    assert!(text.contains("relation: same"));
    assert!(text.contains("  100%    10 lines  two.py"));
}

/// A Python module: a license comment, then a class with `methods`, each with distinct lines.
fn module(class: &str, methods: &[&str]) -> String {
    let mut text = "# Copyright 2026 Example Lab\n# Licensed under Apache 2.0\n\n".to_owned();
    text.push_str(&format!("class {class}:\n"));
    for m in methods {
        text.push_str(&format!(
            "    def {m}(self, values):\n        {m}_total = sum(values) * len('{m}')\n        return {m}_total / max(len(values), 1)\n\n"
        ));
    }
    text
}

#[test]
fn files_sundgren_reads_get_items_and_comment_lines() {
    let tmp = repo();
    let d = &tmp.0;
    git(&tmp.0, &["switch", "-q", "dev"]);
    commit_file(
        d,
        "shapes.py",
        &module("Shape", &["area", "perimeter"]),
        "shapes",
    );
    // An extraction that moved `area` to another file and class, and added a note.
    git(d, &["switch", "-q", "-c", "moved", "main"]);
    commit_file(
        d,
        "geometry.py",
        &format!(
            "{}NOTES = 'unrelated text here'\n",
            module("Geometry", &["area"])
        ),
        "area",
    );

    let doc = doc(&tmp, "moved", "dev", 2.0);
    assert_eq!(doc.a.files_read, 1);
    // dev also changed app.py and added two.py: all Python.
    assert_eq!(doc.b.files_read, 3);

    let d = &doc.a_in_b;
    // The two license lines are comments, and found in dev.
    assert_eq!(d.comment_lines, Some(2));
    // 2 comments, class, def, 2 body lines and NOTES; the class line and NOTES aren't in dev.
    assert_eq!(d.lines, 7);
    assert_eq!(d.exact, 5);
    assert_eq!(d.containment, Some(5.0 / 7.0));
    assert_eq!(d.code_containment, Some(3.0 / 5.0));

    let area = d
        .items
        .iter()
        .find(|i| i.item.as_deref() == Some("Geometry.area"))
        .unwrap();
    assert_eq!(
        area,
        &ItemShare {
            path: "geometry.py".to_owned(),
            item: Some("Geometry.area".to_owned()),
            kind: Some("function"),
            start: Some(5),
            end: Some(7),
            lines: 3,
            exact: 3,
            fuzzy: 0,
            counterpart: Some(Counterpart {
                path: "shapes.py".to_owned(),
                item: Some("Shape.area".to_owned()),
                start: Some(5),
                end: Some(7),
                lines: 3,
            }),
        }
    );
    // Lines outside items: the comments and NOTES, found among dev's comments.
    let outside = d.items.iter().find(|i| i.item.is_none()).unwrap();
    assert_eq!((outside.lines, outside.exact), (3, 2));
    assert_eq!(outside.counterpart.as_ref().unwrap().item, None);

    let text = text(&doc);
    assert!(
        text.contains("A in B without comment lines: 60% (2 comment lines)"),
        "{text}"
    );
    assert!(
        text.contains("100%     3 lines  geometry.py Geometry.area  -> shapes.py Shape.area"),
        "{text}"
    );
    // Same file on the other side: just the item.
    let back = containment_document(&tmp.0, "main", "dev", "moved", 2.0).unwrap();
    assert!(
        super::text(&back).contains("shapes.py Shape.area  -> geometry.py Geometry.area"),
        "{}",
        super::text(&back)
    );
}

#[test]
fn files_sundgren_does_not_read_are_as_before() {
    let tmp = TempDir::new();
    let d = &tmp.0;
    git(d, &["init", "-q", "-b", "main"]);
    commit_file(d, "notes.txt", "start\n", "start");
    git(d, &["switch", "-q", "-c", "a"]);
    commit_file(d, "notes.txt", "start\n# a comment line\nmore text\n", "a");
    let doc = containment_document(d, "main", "a", "a", 2.0).unwrap();
    assert_eq!(doc.a.files_read, 0);
    assert_eq!(doc.a_in_b.comment_lines, None);
    assert_eq!(doc.a_in_b.code_containment, None);
    assert!(doc.a_in_b.items.is_empty());
    let json = serde_json::to_value(&doc).unwrap();
    assert!(json["a_in_b"].get("items").is_none());
    assert!(json["a_in_b"].get("comment_lines").is_none());
}
