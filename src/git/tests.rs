use super::*;

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
