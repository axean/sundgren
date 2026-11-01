use super::*;
use crate::test_support::{TempDir, git_at};

/// A repository whose commits are an hour apart, so time order is fixed.
struct Repo {
    tmp: TempDir,
    time: i64,
}

impl Repo {
    fn new() -> Self {
        let repo = Repo {
            tmp: TempDir::new(),
            time: 1_700_000_000,
        };
        repo.run(&["init", "-q", "-b", "main"]);
        repo
    }

    fn dir(&self) -> &Path {
        &self.tmp.0
    }

    fn run(&self, args: &[&str]) {
        git_at(self.dir(), args, Some(self.time));
    }

    fn tick(&mut self) {
        self.time += 3600;
    }

    /// Append `text` to `path` and commit it as `message`.
    fn commit(&mut self, path: &str, text: &str, message: &str) {
        self.tick();
        let file = self.dir().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let old = std::fs::read_to_string(&file).unwrap_or_default();
        std::fs::write(&file, format!("{old}{text}\n")).unwrap();
        self.run(&["add", "--", path]);
        self.run(&["commit", "-q", "-m", message]);
    }

    fn merge(&mut self, branch: &str, message: &str) {
        self.tick();
        self.run(&["merge", "-q", "--no-ff", "--no-edit", "-m", message, branch]);
    }

    fn switch(&self, branch: &str) {
        self.run(&["switch", "-q", branch]);
    }

    fn remote(&self, name: &str, rev: &str) {
        self.run(&["update-ref", &format!("refs/remotes/origin/{name}"), rev]);
    }
}

/// The merge and rebase sketch of notes/history-map.md:
/// - feat-a forks at m2, merges main back in (aM), then is merged into main (m6); in sync.
/// - feat-b is rebased onto m7 locally, while origin/feat-b still has the old b1, b2.
/// - feat-c forks from the old b1; local is one ahead of origin/feat-c.
/// - fix-d: d2 was pushed and origin/fix-d merged into main (m5); local is one behind.
///
/// Remote refs are set with update-ref; twins pair up by name (`origin/NAME`).
fn sketch() -> Repo {
    let mut r = Repo::new();
    r.commit("README", "hello", "m1");
    r.commit("src/core.py", "def core(): pass", "m2");
    r.run(&["branch", "feat-a"]);
    r.switch("feat-a");
    r.commit("src/a.py", "def a1(): return 1", "a1");
    r.switch("main");
    r.commit("src/core.py", "def core2(): pass", "m3");
    r.run(&["branch", "feat-b"]);
    r.switch("feat-b");
    r.commit("src/b.py", "def b1(): return 'b1 work here'", "b1");
    r.run(&["branch", "feat-c"]);
    r.switch("feat-a");
    r.commit("src/a.py", "def a2(): return 2", "a2");
    r.switch("main");
    r.commit("src/core.py", "def core3(): pass", "m4");
    r.switch("feat-c");
    r.commit("src/c.py", "def c1(): return 'c1 builds on b1'", "c1");
    r.switch("main");
    r.run(&["branch", "fix-d"]);
    r.switch("fix-d");
    r.commit("src/fix.py", "def fix(): return 'the fix'", "d1");
    r.switch("feat-b");
    r.commit("src/b.py", "def b2(): return 'b2 more work'", "b2");
    r.switch("feat-a");
    r.merge("main", "aM: merge main into feat-a");
    r.switch("fix-d");
    r.commit("src/fix.py", "def fix2(): return 'review fix'", "d2");
    r.remote("fix-d", "HEAD");
    r.run(&["reset", "-q", "--hard", "HEAD~1"]);
    r.switch("main");
    r.merge("origin/fix-d", "m5: merge fix-d");
    r.switch("feat-c");
    r.commit("src/c.py", "def c2(): return 'c2 more'", "c2");
    r.remote("feat-c", "HEAD~1");
    r.switch("feat-a");
    r.commit("src/a.py", "def a3(): return 3", "a3");
    r.remote("feat-a", "HEAD");
    r.switch("main");
    r.merge("feat-a", "m6: merge feat-a");
    r.commit("src/core.py", "def core7(): pass", "m7");
    r.remote("main", "HEAD");
    r.remote("feat-b", "feat-b");
    r.switch("feat-b");
    r.tick();
    r.run(&["rebase", "-q", "main"]);
    r.commit("src/b.py", "def b3(): return 'b3 new'", "b3");
    r.switch("main");
    r
}

fn options(target: &str) -> Options {
    Options {
        base: "main".to_owned(),
        target: Some(target.to_owned()),
        branches: 5,
        offset: 0,
        columns: 40,
        protected: vec!["main".to_owned(), "master".to_owned()],
        pins: Vec::new(),
        threshold: 0.9,
    }
}

fn lane<'a>(doc: &'a MapDocument, name: &str) -> &'a Lane {
    doc.lanes.iter().find(|l| l.name == name).unwrap()
}

#[test]
fn draws_the_merge_and_rebase_sketch() {
    let r = sketch();
    let doc = map_document(r.dir(), &options("feat-b")).unwrap();
    assert_eq!(
        text(&doc),
        "\
base main · target feat-b · 3 of 3 branches in the frame
                                                  ▼origin/main
     ╭──○────────○──────────────○───────────○                    origin/feat-a  ↑0 ↓5 · merged
     ├──○────────○──────────────○───────────○──╮                 feat-a         ↑0 ↓5 · merged
  ●──●─────●────────●───────────┴─────●────────●──●              main           base · protected
           │        ├─────○           │           │              fix-d          ↑0 ↓8 · merged
           │        ╰─────○────────○──╯           │              origin/fix-d   ↑0 ↓7 · merged
           │  ╭────────○─────────────────○        │              feat-c         ↑3 ↓10 · 33% in target
           │  ├────────○                          │              origin/feat-c  ↑2 ↓10 · 33% in target
           │  │                                   ╰──○──○──○     feat-b         ↑3 ↓0
           ╰──○──────────────○                                   origin/feat-b  ↑2 ↓10
"
    );
}

#[test]
fn lanes_know_their_branch_and_twin() {
    let r = sketch();
    let doc = map_document(r.dir(), &options("feat-b")).unwrap();
    assert_eq!(doc.schema, SCHEMA);
    assert_eq!(doc.lanes[0].kind, LaneKind::Reference);
    assert_eq!(doc.lanes[0].name, "main");
    assert!(doc.lanes[0].protected);
    assert_eq!(doc.lanes[0].family, None);
    for (i, l) in doc.lanes.iter().enumerate() {
        if let Some(t) = l.twin {
            assert_eq!(doc.lanes[t].twin, Some(i), "{}", l.name);
            assert_eq!(doc.lanes[t].branch, l.branch);
            assert_eq!(doc.lanes[t].family, l.family);
        }
    }
    let b = lane(&doc, "feat-b");
    assert!(b.pinned);
    assert_eq!((b.ahead, b.behind), (3, 0));
    assert_eq!(b.score, None);
    assert_eq!(lane(&doc, "origin/feat-b").kind, LaneKind::Remote);
    assert!(lane(&doc, "origin/fix-d").merged);
    assert!(!lane(&doc, "feat-c").merged);
    // feat-c leaves from b1, which only the old feat-b has.
    let c = lane(&doc, "feat-c");
    let b1 = doc.commits.iter().find(|x| x.subject == "b1").unwrap();
    assert_eq!(c.fork.as_deref(), Some(b1.oid.as_str()));
    assert_eq!(
        b1.lanes,
        [doc.lanes
            .iter()
            .position(|l| l.name == "origin/feat-b")
            .unwrap()]
    );
    // A commit twins share is listed once, on both lanes.
    let c1 = doc.commits.iter().find(|x| x.subject == "c1").unwrap();
    assert_eq!(c1.lanes.len(), 2);
    assert_eq!(
        doc.commits.len(),
        doc.grid
            .cells
            .iter()
            .filter(|c| c.node == Some(NodeKind::Commit))
            .map(|c| c.commit)
            .collect::<HashSet<_>>()
            .len()
    );
}

#[test]
fn a_narrow_frame_uses_connectors() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            columns: 12,
            branches: 3,
            ..options("feat-b")
        },
    )
    .unwrap();
    assert!(doc.commits.len() <= 12);
    assert_eq!(doc.frame.newer, None);
    assert_eq!(doc.frame.older, Some(2));
    assert_eq!(
        text(&doc),
        "\
base main · target feat-b · 2 of 2 branches in the frame
              ▼origin/main
⋯─●────────●──●              main           base · protected · 4 earlier
⋯───────○──╯  │              feat-a         ↑0 ↓5 · merged · 3 earlier
⋯───────○     │              origin/feat-a  ↑0 ↓5 · merged · 3 earlier
              ╰──○──○──○     feat-b         ↑3 ↓0
⋯                            origin/feat-b  ↑2 ↓10 · 2 earlier
⋯────○                       feat-c         ↑3 ↓10 · 33% in target · 1 earlier
"
    );
}

#[test]
fn walks_back_in_time() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            columns: 12,
            branches: 3,
            offset: 3,
            ..options("feat-b")
        },
    )
    .unwrap();
    assert_eq!(doc.frame.offset, 3);
    assert_eq!(doc.frame.newer, Some(0));
    assert_eq!(doc.frame.older, None);
    assert!(lane(&doc, "main").after);
    assert_eq!(
        text(&doc),
        "\
base main · target feat-b · 3 of 3 branches in the frame
  ●──●─────●────────●──⋯  main           base · protected · more later
     │     │        ├──⋯  fix-d          ↑0 ↓8 · merged · more later
     │     │        ╰──⋯  origin/fix-d   ↑0 ↓7 · merged · more later
     │     │  ╭────────⋯  feat-c         ↑3 ↓10 · 33% in target · more later
     │     │  ├────────⋯  origin/feat-c  ↑2 ↓10 · 33% in target · more later
     │     │  │        ⋯  feat-b         ↑3 ↓0 · more later
     │     ╰──○────────⋯  origin/feat-b  ↑2 ↓10 · more later
     ├──○────────○─────⋯  feat-a         ↑0 ↓5 · merged · more later
     ╰──○────────○─────⋯  origin/feat-a  ↑0 ↓5 · merged · more later
"
    );
}

#[test]
fn ranks_by_containment_in_the_target() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            branches: 1,
            ..options("feat-c")
        },
    )
    .unwrap();
    // feat-b's b1 is in feat-c; fix-d and feat-a share nothing with it.
    let shown: Vec<&str> = doc.lanes.iter().map(|l| l.branch.as_str()).collect();
    assert!(shown.contains(&"feat-b"), "{shown:?}");
    assert!(!shown.contains(&"fix-d"), "{shown:?}");
    assert!(lane(&doc, "feat-b").score.is_some_and(|s| s > 0.3));
}

#[test]
fn without_a_target_the_newest_branches_come_first() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            target: None,
            branches: 1,
            ..options("x")
        },
    )
    .unwrap();
    // The checked-out branch is main, the base: no target.
    assert_eq!(doc.target, None);
    let shown: HashSet<&str> = doc.lanes.iter().map(|l| l.branch.as_str()).collect();
    assert_eq!(shown, HashSet::from(["main", "feat-b"]));
}

#[test]
fn local_only_and_remote_only_branches() {
    let mut r = Repo::new();
    r.commit("a", "a", "a");
    r.run(&["branch", "mine"]);
    r.switch("mine");
    r.commit("m", "mine", "mine");
    r.switch("main");
    r.commit("a", "b", "b");
    r.run(&["branch", "theirs"]);
    r.switch("theirs");
    r.commit("t", "theirs", "theirs");
    r.remote("theirs", "HEAD");
    r.switch("main");
    r.run(&["branch", "-q", "-D", "theirs"]);
    let doc = map_document(
        r.dir(),
        &Options {
            target: None,
            ..options("x")
        },
    )
    .unwrap();
    assert_eq!(lane(&doc, "mine").only, Some(Only::Local));
    let theirs = lane(&doc, "origin/theirs");
    assert_eq!(theirs.only, Some(Only::Remote));
    assert_eq!(theirs.branch, "theirs");
    assert_eq!(theirs.kind, LaneKind::Remote);
}

#[test]
fn unknown_pins_and_bases_are_usage_errors() {
    let r = sketch();
    let bad_pin = Options {
        pins: vec!["nope".to_owned()],
        ..options("feat-b")
    };
    assert_eq!(
        map_document(r.dir(), &bad_pin).unwrap_err().code,
        crate::EXIT_USAGE
    );
    let bad_base = Options {
        base: "nope".to_owned(),
        ..options("feat-b")
    };
    assert_eq!(
        map_document(r.dir(), &bad_base).unwrap_err().code,
        crate::EXIT_USAGE
    );
}

#[test]
fn globs() {
    assert!(glob("main", "main"));
    assert!(!glob("main", "mainline"));
    assert!(glob("release/*", "release/1.0"));
    assert!(glob("*", ""));
    assert!(glob("a*b*c", "aXbYc"));
    assert!(!glob("a*b", "ac"));
    assert!(glob("*é", "café"));
}

#[test]
fn glyphs_follow_the_sides() {
    let cell = |sides: u8, node: Option<NodeKind>| Cell {
        row: 0,
        col: 0,
        sides,
        lane: 0,
        node,
        commit: None,
    };
    let got: String = [
        LEFT | RIGHT,
        UP | DOWN,
        RIGHT | DOWN,
        LEFT | DOWN,
        UP | RIGHT,
        UP | LEFT,
        UP | RIGHT | DOWN,
        UP | LEFT | DOWN,
        LEFT | RIGHT | DOWN,
        UP | LEFT | RIGHT,
        15,
        LEFT,
        RIGHT,
    ]
    .iter()
    .map(|&s| glyph(&cell(s, None), false))
    .collect();
    assert_eq!(got, "─│╭╮╰╯├┤┬┴┼╴╶");
    assert_eq!(glyph(&cell(LEFT, Some(NodeKind::Commit)), true), '●');
    assert_eq!(glyph(&cell(LEFT, Some(NodeKind::Commit)), false), '○');
    assert_eq!(glyph(&cell(RIGHT, Some(NodeKind::Connector)), false), '⋯');
}

#[test]
fn a_crossing_becomes_a_via() {
    let mut board = Board {
        cells: BTreeMap::new(),
        vias: 0,
    };
    // Lane 1 runs along row 1; a vertical of lane 2 crosses it at column 2.
    for x in 0..5 {
        let sides = if x == 0 {
            RIGHT
        } else if x == 4 {
            LEFT
        } else {
            LEFT | RIGHT
        };
        board.add(1, x, sides, 1);
    }
    board.vertical(2, 0, 2, 2);
    assert_eq!(board.vias, 1);
    let sides = |r, c| board.cells[&(r, c)].sides;
    assert_eq!(sides(1, 1), LEFT);
    assert_eq!(sides(1, 2), UP | DOWN);
    assert_eq!(board.cells[&(1, 2)].lane, 2);
    assert_eq!(sides(1, 3), RIGHT);
    assert_eq!(sides(0, 2), DOWN);
    assert_eq!(sides(2, 2), UP);
}

#[test]
fn branches_pair_locals_with_their_remotes() {
    let history = History::new(
        ["c1", "c2", "c3", "c4"]
            .iter()
            .map(|oid| git::Node {
                oid: (*oid).to_owned(),
                parents: Vec::new(),
                committed: 0,
                author: String::new(),
                subject: String::new(),
            })
            .collect(),
    );
    let tip = |name: &str, kind, oid: &str, upstream: Option<&str>| Tip {
        name: name.to_owned(),
        kind,
        oid: oid.to_owned(),
        upstream: upstream.map(str::to_owned),
    };
    let tips = [
        tip("main", RefKind::Branch, "c1", Some("origin/main")),
        tip("work", RefKind::Branch, "c2", Some("fork/work")),
        tip("solo", RefKind::Branch, "c2", None),
        tip("origin/main", RefKind::Remote, "c1", None),
        tip("fork/work", RefKind::Remote, "c3", None),
        tip("origin/work", RefKind::Remote, "c4", None),
        tip("origin/other", RefKind::Remote, "c4", None),
        tip("v1", RefKind::Tag, "c1", None),
    ];
    let got: Vec<(String, Option<String>, Option<String>)> = branches(&tips, &history)
        .into_iter()
        .map(|b| (b.name, b.local.map(|l| l.0), b.remote.map(|r| r.0)))
        .collect();
    let s = |x: &str| Some(x.to_owned());
    assert_eq!(
        got,
        [
            ("main".to_owned(), s("main"), s("origin/main")),
            ("work".to_owned(), s("work"), s("fork/work")),
            ("solo".to_owned(), s("solo"), None),
            ("other".to_owned(), None, s("origin/other")),
            // origin/work is not work's upstream, and the name work is taken.
            ("origin/work".to_owned(), None, s("origin/work")),
        ]
    );
}
