use super::*;
use crate::test_support::{TempDir, git, git_at};

fn commit(dir: &Path, message: &str, time: i64) {
    git_at(
        dir,
        &["commit", "-q", "--allow-empty", "-m", message],
        Some(time),
    );
}

fn sync(dir: &Path, message: &str, time: i64) {
    git_at(dir, &["fetch", "-q", "origin"], None);
    git_at(
        dir,
        &["merge", "-q", "--no-ff", "-m", message, "origin/main"],
        Some(time),
    );
}

/// upstream: u1 .. u7 on main. copy: cloned at u2; own work a1, a2; merges upstream's main in
/// twice (m1 brings u3 and u4, m2 brings u5); a side branch merges u6 (s1); then upstream adds
/// u7.
fn pair() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new();
    let upstream = tmp.0.join("upstream");
    let copy = tmp.0.join("copy");
    std::fs::create_dir_all(&upstream).unwrap();
    git(&upstream, &["init", "-q", "-b", "main"]);
    commit(&upstream, "u1", 100);
    commit(&upstream, "u2", 200);
    git(&tmp.0, &["clone", "-q", "upstream", "copy"]);
    commit(&upstream, "u3", 300);
    commit(&upstream, "u4", 400);
    commit(&copy, "a1", 500);
    sync(&copy, "m1", 600);
    commit(&upstream, "u5", 700);
    commit(&copy, "a2", 800);
    sync(&copy, "m2", 900);
    commit(&upstream, "u6", 1000);
    git(&copy, &["switch", "-q", "-c", "side"]);
    sync(&copy, "s1", 1100);
    git(&copy, &["switch", "-q", "main"]);
    commit(&upstream, "u7", 1200);
    (tmp, copy, upstream)
}

fn relate(tmp: &TempDir, this: &Path, other: &Path) -> RelateDocument {
    let joined = join::join_in(&tmp.0.join("cache"), this, other, None).unwrap();
    joined_document(&joined, &Options::default()).unwrap()
}

fn subject(p: &Point) -> &str {
    &p.subject
}

#[test]
fn finds_where_a_copy_left_and_how_it_was_synced() {
    let (tmp, copy, upstream) = pair();
    let doc = relate(&tmp, &copy, &upstream);
    assert!(doc.related);
    assert_eq!(doc.shared.commits, 6);
    assert_eq!(doc.shared.roots.len(), 1);
    assert_eq!((doc.this.commits, doc.this.base_commits), (11, 9));
    assert_eq!((doc.other.commits, doc.other.base_commits), (7, 7));
    assert_eq!(
        (doc.this.name.as_str(), doc.other.name.as_str()),
        ("copy", "upstream")
    );

    let c = doc.this.copy_point.as_ref().unwrap();
    assert_eq!((subject(&c.commit), subject(&c.first_own)), ("u2", "a1"));
    // Upstream's main moved on at u3, before the first own commit.
    assert_eq!(
        c.window,
        Window {
            from: 200,
            until: 300
        }
    );
    let c = doc.other.copy_point.as_ref().unwrap();
    assert_eq!((subject(&c.commit), subject(&c.first_own)), ("u5", "u6"));

    assert_eq!(doc.last_sync.as_ref().map(subject), Some("u5"));
    let newest = doc.newest_shared.as_ref().unwrap();
    assert_eq!(subject(&newest.commit), "u6");
    assert!(!newest.on_this_base && newest.on_other_base);
    assert_eq!(newest.this_refs, ["side", "origin/main"]);
    assert!(newest.other_refs.is_empty());

    let merges: Vec<_> = doc
        .this
        .sync_merges
        .iter()
        .map(|m| (subject(&m.commit), m.count, subject(&m.newest)))
        .collect();
    assert_eq!(merges, [("m1", 2, "u4"), ("m2", 1, "u5")]);
    assert!(doc.other.sync_merges.is_empty());
    assert_eq!(doc.copy, Some(Which::This));
    assert_eq!(
        doc.direction,
        Some(Direction {
            commits: 3,
            this_first: 0,
            other_first: 3,
            same_time: 0
        })
    );
    let kinds: Vec<_> = doc
        .marks
        .iter()
        .map(|m| (m.kind, m.side, m.count))
        .collect();
    assert_eq!(
        kinds,
        [
            (MarkKind::CopyPoint, Some(Which::This), None),
            (MarkKind::SyncMerge, Some(Which::This), Some(2)),
            (MarkKind::SyncMerge, Some(Which::This), Some(1)),
            (MarkKind::CopyPoint, Some(Which::Other), None),
            (MarkKind::LastSync, None, None),
            (MarkKind::NewestShared, None, None),
        ]
    );
}

#[test]
fn the_other_way_round() {
    let (tmp, copy, upstream) = pair();
    let doc = relate(&tmp, &upstream, &copy);
    let c = doc.this.copy_point.as_ref().unwrap();
    assert_eq!(subject(&c.commit), "u5");
    assert_eq!(doc.other.sync_merges.len(), 2);
    let newest = doc.newest_shared.as_ref().unwrap();
    assert_eq!(newest.other_refs, ["side", "origin/main"]);
    assert_eq!(doc.copy, Some(Which::Other));
    let d = doc.direction.unwrap();
    assert_eq!((d.this_first, d.other_first), (3, 0));
}

#[test]
fn unrelated_repositories() {
    let tmp = TempDir::new();
    let (a, b) = (tmp.0.join("a"), tmp.0.join("b"));
    for (dir, message) in [(&a, "a"), (&b, "b")] {
        std::fs::create_dir_all(dir).unwrap();
        git(dir, &["init", "-q", "-b", "main"]);
        commit(dir, message, 100);
    }
    let doc = relate(&tmp, &a, &b);
    assert!(!doc.related);
    assert_eq!(
        (doc.this.commits, doc.other.commits, doc.shared.commits),
        (1, 1, 0)
    );
    assert!(doc.marks.is_empty() && doc.last_sync.is_none());
    assert!(text(&doc).ends_with("unrelated: no shared commits\n"));
}

#[test]
fn bases_can_be_given() {
    let (tmp, copy, upstream) = pair();
    let joined = join::join_in(&tmp.0.join("cache"), &copy, &upstream, None).unwrap();
    let options = Options {
        base: Some("side".to_owned()),
        other_base: Some("main~1".to_owned()),
        name: None,
    };
    let doc = joined_document(&joined, &options).unwrap();
    assert_eq!(doc.this.base, "side");
    // Upstream's main~1 is u6, which side has: upstream's base left nothing behind.
    assert!(doc.other.copy_point.is_none());
    assert_eq!(doc.last_sync.as_ref().map(subject), Some("u6"));
    let bad = Options {
        base: Some("nope".to_owned()),
        ..Options::default()
    };
    assert_eq!(
        joined_document(&joined, &bad).unwrap_err().code,
        crate::EXIT_USAGE
    );
}

#[test]
fn reads_as_a_short_report() {
    let (tmp, copy, upstream) = pair();
    let out = text(&relate(&tmp, &copy, &upstream));
    assert!(out.contains("shared: 6 commits, 1 root\n"), "{out}");
    assert!(
        out.contains("copy's main left upstream's main at "),
        "{out}"
    );
    assert!(
        out.contains("  the copy: made between 1970-01-01 00:03 and 1970-01-01 00:05\n"),
        "{out}"
    );
    assert!(
        out.contains("not on copy's main; reaches it through side, origin/main\n"),
        "{out}"
    );
    assert!(out.contains("+2 up to "), "{out}");
}

#[test]
fn utc_dates() {
    assert_eq!(utc(0), "1970-01-01 00:00");
    assert_eq!(utc(1_750_154_646), "2025-06-17 10:04");
    assert_eq!(utc(951_825_600), "2000-02-29 12:00");
    assert_eq!(utc(-60), "1969-12-31 23:59");
}
