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

fn options(show: Show) -> Options {
    Options {
        base: "main".to_owned(),
        show,
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

fn lane<'a>(doc: &'a MapDocument, name: &str) -> &'a Lane {
    doc.lanes.iter().find(|l| l.name == name).unwrap()
}

fn has_lane(doc: &MapDocument, name: &str) -> bool {
    doc.lanes.iter().any(|l| l.name == name)
}

#[test]
fn draws_the_merge_and_rebase_sketch() {
    let r = sketch();
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    assert_eq!(
        text(&doc),
        "\
base main · show local first · 4 of 4 branches · 20 commits
          ○    feat-b ↑3 ↓0                             ×3 b3
  ●───────╯    main base · = origin/main · protected    m7
╭─●                                                     m6: merge feat-a
○ │            feat-a ↑0 ↓5 · = origin/feat-a · merged  a3
│ │     ○      feat-c ↑3 ↓10                            c2
│ ●───╮ │                                               m5: merge fix-d
│ │   ○ │      origin/fix-d ↑0 ↓7 · merged              d2
○─┤   │ │                                               aM: merge main into feat-a
│ │   │ │   ○  origin/feat-b ↑2 ↓10                     b2
│ │ ○─╯ │   │  fix-d ↑0 ↓8 · merged                     d1
│ │ │   ○   │  (origin/feat-c)                          c1
│ ●─╯   │   │                                           m4
○ │     │   │                                           a2
│ │     ╰───○                                           b1
│ ●─────────╯                                           m3
○ │                                                     a1
╰─●                                                     m2
  ●                                                     m1
"
    );
}

#[test]
fn twins_draw_shared_history_once() {
    let r = sketch();
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    assert_eq!(doc.schema, SCHEMA);
    let main = &doc.lanes[0];
    assert_eq!(
        (main.kind, main.name.as_str()),
        (LaneKind::Reference, "main")
    );
    assert_eq!(main.remote.as_deref(), Some("origin/main"));
    assert!(main.protected);
    assert_eq!(main.family, None);
    for (i, l) in doc.lanes.iter().enumerate() {
        if let Some(t) = l.twin {
            assert_eq!(doc.lanes[t].twin, Some(i), "{}", l.name);
            assert_eq!(doc.lanes[t].family, l.family);
        }
    }
    // In sync: one lane.
    assert_eq!(
        lane(&doc, "feat-a").remote.as_deref(),
        Some("origin/feat-a")
    );
    assert!(!has_lane(&doc, "origin/feat-a"));
    // Behind its local: a ref on the commit, no lane.
    assert!(!has_lane(&doc, "origin/feat-c"));
    let c1 = doc.commits.iter().find(|c| c.subject == "c1").unwrap();
    assert!(c1.refs.contains(&"origin/feat-c".to_owned()));
    assert_eq!(c1.lanes.len(), 1);
    // Ahead of its local: only its own commit, off the local lane.
    let d1 = doc.commits.iter().find(|c| c.subject == "d1").unwrap();
    assert_eq!(
        lane(&doc, "origin/fix-d").fork.as_deref(),
        Some(d1.oid.as_str())
    );
    // feat-c leaves from the old b1, on origin/feat-b (the rebased b1 is on feat-b).
    let old = doc
        .lanes
        .iter()
        .position(|l| l.name == "origin/feat-b")
        .unwrap();
    let b1 = doc
        .commits
        .iter()
        .find(|c| c.subject == "b1" && c.lanes == [old])
        .unwrap();
    assert_eq!(lane(&doc, "feat-c").fork.as_deref(), Some(b1.oid.as_str()));
    let b = lane(&doc, "feat-b");
    assert_eq!((b.ahead, b.behind), (3, 0));
    assert!(lane(&doc, "origin/fix-d").merged);
}

#[test]
fn rows_and_labels_line_up() {
    let r = sketch();
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    for (n, c) in doc.commits.iter().enumerate() {
        let node = doc
            .grid
            .cells
            .iter()
            .find(|cell| cell.row == c.row && cell.commit == Some(n));
        assert!(node.is_some(), "{} has no node on row {}", c.subject, c.row);
    }
    // Newest first.
    assert!(doc.commits.windows(2).all(|w| w[0].row < w[1].row));
    for l in &doc.lanes {
        let tip = doc.commits.iter().find(|c| c.oid == l.tip);
        if let Some(tip) = tip {
            assert_eq!(l.label_row, tip.row, "{}", l.name);
        }
    }
}

#[test]
fn a_quiet_run_folds() {
    let r = sketch();
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    let b = doc.lanes.iter().position(|l| l.name == "feat-b").unwrap();
    let fold = doc.folds.iter().find(|f| f.lane == b).unwrap();
    let subjects: Vec<&str> = fold
        .commits
        .iter()
        .map(|&n| doc.commits[n].subject.as_str())
        .collect();
    assert_eq!(subjects, ["b3", "b2", "b1"]);
    assert_eq!(
        fold.rows,
        [
            doc.commits[fold.commits[0]].row,
            doc.commits[fold.commits[2]].row
        ]
    );
    // Nothing else folds: every other run has a merge, a fork or a ref in it.
    assert_eq!(doc.folds.len(), 1);
}

#[test]
fn a_short_page_ends_in_connectors() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            commits: 3,
            ..options(Show::Local)
        },
    )
    .unwrap();
    assert!(doc.page.more);
    assert_eq!(doc.page.commits, 3);
    assert!(text(&doc).ends_with("below: main 4, feat-a 3, feat-c 1\n"));
    // m7, m6 and m5 of main; m4 and older are not on the page.
    let main: Vec<&str> = doc
        .commits
        .iter()
        .filter(|c| c.lanes.contains(&0))
        .map(|c| c.subject.as_str())
        .collect();
    assert_eq!(main, ["m7", "m6: merge feat-a", "m5: merge fix-d"]);
    assert_eq!(doc.lanes[0].before, Some(4));
    let bottom = doc.grid.height - 1;
    assert!(
        doc.grid
            .cells
            .iter()
            .any(|c| c.row == bottom && c.node == Some(NodeKind::Connector))
    );
    let all = map_document(r.dir(), &options(Show::Local)).unwrap();
    assert!(!all.page.more);
}

#[test]
fn like_ranks_by_containment() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            branches: 1,
            ..options(Show::Like("feat-c".to_owned()))
        },
    )
    .unwrap();
    assert_eq!(doc.show.mode, Mode::Like);
    assert_eq!(
        doc.show.like.as_ref().map(|l| l.name.as_str()),
        Some("feat-c")
    );
    // feat-b's b1 is in feat-c; fix-d and feat-a share nothing with it.
    let shown: HashSet<&str> = doc.lanes.iter().map(|l| l.branch.as_str()).collect();
    assert_eq!(shown, HashSet::from(["main", "feat-c", "feat-b"]));
    assert!(lane(&doc, "feat-b").score.is_some_and(|s| s > 0.3));
    assert!(lane(&doc, "feat-c").pinned);
}

#[test]
fn recent_shows_the_newest_work_first() {
    let r = sketch();
    let doc = map_document(
        r.dir(),
        &Options {
            branches: 1,
            ..options(Show::Recent)
        },
    )
    .unwrap();
    let shown: HashSet<&str> = doc.lanes.iter().map(|l| l.branch.as_str()).collect();
    assert_eq!(shown, HashSet::from(["main", "feat-b"]));
    assert_eq!(doc.show.like, None);
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
    let doc = map_document(r.dir(), &options(Show::Recent)).unwrap();
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
        hides: Vec::new(),
        since: None,
        until: None,
        ..options(Show::Local)
    };
    assert_eq!(
        map_document(r.dir(), &bad_pin).unwrap_err().code,
        crate::EXIT_USAGE
    );
    let bad_base = Options {
        base: "nope".to_owned(),
        ..options(Show::Local)
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
        rows: Vec::new(),
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
                email: String::new(),
                subject: String::new(),
                body: String::new(),
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

#[test]
fn merging_an_older_commit_runs_along_its_own_row() {
    // feature merges m2 after main already has m3: the line can't leave main at the merge
    // commit's column, where m3 would look like the merged commit.
    let mut r = Repo::new();
    r.commit("a", "m1", "m1");
    r.run(&["branch", "feature"]);
    r.commit("a", "m2", "m2");
    r.commit("a", "m3", "m3");
    r.switch("feature");
    r.commit("f", "f1", "f1");
    r.merge("main~1", "fM: merge m2");
    r.switch("main");
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    assert_eq!(
        text(&doc),
        "\
base main · show local first · 1 of 1 branches · 5 commits
  ╭─○  feature ↑2 ↓1 · local only  fM: merge m2
  │ ○                              f1
● │ │  main base · protected       m3
●─╯ │                              m2
●───╯                              m1
"
    );
}

#[test]
fn hiding_a_branch_does_not_bring_in_the_next() {
    let r = sketch();
    let top = map_document(
        r.dir(),
        &Options {
            branches: 2,
            ..options(Show::Local)
        },
    )
    .unwrap();
    // Every branch is listed, in show order, drawn or not.
    let names: Vec<String> = top.branches.iter().map(|b| b.name.clone()).collect();
    assert_eq!(names.len(), 4);
    let [first, second, third, ..] = &names[..] else {
        panic!("{names:?}");
    };
    let drawn = |doc: &MapDocument, name: &str| doc.lanes.iter().any(|l| l.branch == name);
    assert!(drawn(&top, first) && drawn(&top, second) && !drawn(&top, third));

    let hidden = map_document(
        r.dir(),
        &Options {
            branches: 2,
            hides: vec![first.clone(), "no-such-branch".to_owned()],
            pins: vec![third.clone()],
            ..options(Show::Local)
        },
    )
    .unwrap();
    assert_eq!(hidden.branches, top.branches);
    assert!(!drawn(&hidden, first));
    assert!(drawn(&hidden, second) && drawn(&hidden, third));
    assert_eq!(names.iter().filter(|name| drawn(&hidden, name)).count(), 2);
}

#[test]
fn branches_say_where_they_are_and_who_wrote_them() {
    let r = sketch();
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    let b = doc.branches.iter().find(|b| b.name == "feat-b").unwrap();
    assert!(b.local);
    // Test commits are all by the same author.
    assert_eq!(b.authors.len(), 1);
    assert_eq!(doc.commits[0].email, b.authors[0]);
}

#[test]
fn since_and_until_bound_the_page() {
    let r = sketch();
    let all = map_document(r.dir(), &options(Show::Local)).unwrap();
    let time = |subject: &str| {
        all.commits
            .iter()
            .find(|c| c.subject == subject)
            .unwrap()
            .committed
    };
    let doc = map_document(
        r.dir(),
        &Options {
            since: Some(time("m3").to_string()),
            until: Some(time("m5: merge fix-d").to_string()),
            ..options(Show::Local)
        },
    )
    .unwrap();
    let times: Vec<i64> = doc.commits.iter().map(|c| c.committed).collect();
    assert!(!times.is_empty());
    assert!(
        times
            .iter()
            .all(|t| (time("m3")..=time("m5: merge fix-d")).contains(t))
    );
    assert!(doc.page.more && doc.page.newer);
    assert_eq!(doc.page.since, Some(time("m3")));
    // Lanes going on above the page end in connectors at the top.
    assert!(
        doc.grid
            .cells
            .iter()
            .any(|c| c.row == 0 && c.node == Some(NodeKind::Connector))
    );

    let bad = map_document(
        r.dir(),
        &Options {
            since: Some("not a date at all".to_owned()),
            ..options(Show::Local)
        },
    );
    assert!(bad.is_err());
}

#[test]
fn commits_carry_their_body() {
    let mut r = Repo::new();
    r.commit(
        "README",
        "hello",
        "Subject line\n\nThe body,\nover two lines.\n\n",
    );
    let doc = map_document(r.dir(), &options(Show::Local)).unwrap();
    assert_eq!(doc.commits[0].subject, "Subject line");
    assert_eq!(doc.commits[0].body, "The body,\nover two lines.");
}

#[test]
fn a_detached_head_is_on_the_map() {
    let mut r = Repo::new();
    r.commit("README", "hello", "m1");
    r.commit("README", "more", "m2");
    r.run(&["switch", "-q", "--detach", "HEAD"]);
    r.commit("README", "detached", "d1");
    r.run(&["branch", "-q", "-D", "main"]);
    let doc = map_document(
        r.dir(),
        &Options {
            base: "HEAD".to_owned(),
            ..options(Show::Local)
        },
    )
    .unwrap();
    let subjects: Vec<&str> = doc.commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(subjects, ["d1", "m2", "m1"]);
}
