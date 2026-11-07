use super::*;
use crate::test_support::{TempDir, git, git_at};

fn commit(dir: &Path, message: &str, time: i64) {
    git_at(
        dir,
        &["commit", "-q", "--allow-empty", "-m", message],
        Some(time),
    );
}

fn out(dir: &Path, args: &[&str]) -> String {
    let mut cmd = git::git(dir);
    cmd.args(args);
    git::output(cmd).unwrap()
}

/// upstream: main (u1 - u2), tag v1, branch topic. copy: a clone of it, with its own work.
fn pair() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new();
    let upstream = tmp.0.join("upstream");
    let copy = tmp.0.join("copy");
    std::fs::create_dir_all(&upstream).unwrap();
    git(&upstream, &["init", "-q", "-b", "main"]);
    commit(&upstream, "u1", 1);
    git(&upstream, &["tag", "v1"]);
    git(&upstream, &["branch", "topic"]);
    commit(&upstream, "u2", 2);
    git(&tmp.0, &["clone", "-q", "upstream", "copy"]);
    commit(&copy, "own", 3);
    (tmp, copy, upstream)
}

/// Refs, objects and config of a repository, to see that a join leaves it alone.
fn state(dir: &Path) -> String {
    let git_dir = dir.join(".git");
    let mut files: Vec<String> = Vec::new();
    let mut todo = vec![git_dir.clone()];
    while let Some(d) = todo.pop() {
        for entry in std::fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else {
                files.push(path.strip_prefix(&git_dir).unwrap().display().to_string());
            }
        }
    }
    files.sort();
    format!("{}{}", out(dir, &["for-each-ref"]), files.join("\n"))
}

#[test]
fn the_other_repository_is_a_remote() {
    let (tmp, copy, upstream) = pair();
    let before = (state(&copy), state(&upstream));
    let cache = tmp.0.join("cache");
    let joined = join_in(&cache, &copy, &upstream, None).unwrap();
    assert_eq!(joined.name, "upstream");
    assert!(joined.dir.starts_with(&cache));
    assert_eq!(before, (state(&copy), state(&upstream)));
    let refs = out(&joined.dir, &["for-each-ref", "--format=%(refname)"]);
    assert_eq!(
        refs.lines().collect::<Vec<_>>(),
        [
            "refs/heads/main",
            "refs/remotes/origin/main",
            "refs/remotes/origin/topic",
            "refs/remotes/upstream/main",
            "refs/remotes/upstream/topic",
            "refs/tags/upstream/v1",
            "refs/tags/v1",
        ]
    );
    assert_eq!(
        out(&joined.dir, &["symbolic-ref", "HEAD"]),
        "refs/heads/main\n"
    );
    // Both histories, in one repository.
    let log = out(&joined.dir, &["log", "--all", "--format=%s"]);
    assert_eq!(log.lines().collect::<Vec<_>>(), ["own", "u2", "u1"]);
    let mine = joined
        .refs
        .iter()
        .find(|r| r.name == "refs/remotes/upstream/topic");
    assert_eq!(
        mine.map(|r| (r.source, r.original.as_str())),
        Some((Source::Other, "refs/heads/topic"))
    );
    assert_eq!(
        joined.other_ref("refs/tags/v1").as_deref(),
        Some("refs/tags/upstream/v1")
    );
}

#[test]
fn refs_follow_both_repositories() {
    let (tmp, copy, upstream) = pair();
    let cache = tmp.0.join("cache");
    let first = join_in(&cache, &copy, &upstream, None).unwrap();
    git(&upstream, &["branch", "-q", "-D", "topic"]);
    git(&upstream, &["branch", "topic/deeper"]);
    git(&copy, &["switch", "-q", "--detach"]);
    let second = join_in(&cache, &copy, &upstream, None).unwrap();
    assert_eq!(first.dir, second.dir);
    let refs = out(
        &second.dir,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/remotes/upstream",
        ],
    );
    assert_eq!(
        refs.lines().collect::<Vec<_>>(),
        [
            "refs/remotes/upstream/main",
            "refs/remotes/upstream/topic/deeper"
        ]
    );
    let head = out(&second.dir, &["rev-parse", "HEAD"]);
    assert_eq!(head, out(&copy, &["rev-parse", "HEAD"]));
    // The other way round is another join.
    let back = join_in(&cache, &upstream, &copy, None).unwrap();
    assert_ne!(back.dir, second.dir);
    assert_eq!(back.name, "copy");
}

#[test]
fn a_name_taken_by_a_remote_is_refused() {
    let (tmp, copy, upstream) = pair();
    let cache = tmp.0.join("cache");
    let err = join_in(&cache, &copy, &upstream, Some("origin")).unwrap_err();
    assert_eq!(err.code, crate::EXIT_USAGE);
    assert!(err.msg.contains("--name"), "{}", err.msg);
    git(&copy, &["remote", "add", "upstream", "../upstream"]);
    assert!(join_in(&cache, &copy, &upstream, None).is_err());
    let joined = join_in(&cache, &copy, &upstream, Some("mirror")).unwrap();
    assert_eq!(joined.name, "mirror");
    assert!(join_in(&cache, &copy, &upstream, Some("a/b")).is_err());
    assert!(join_in(&cache, &copy, &copy, None).is_err());
}

#[test]
fn names_are_ref_safe() {
    assert_eq!(ref_safe("my repo"), "my-repo");
    assert_eq!(ref_safe(".hidden..x.lock"), "hidden-x");
    assert_eq!(folder_name(Path::new("/x/project/.git")), "project");
    assert_eq!(folder_name(Path::new("/x/mirror.git")), "mirror");
}
