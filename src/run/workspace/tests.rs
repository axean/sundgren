use super::*;

pub fn sh(dir: &Path, script: &str) {
    let ok = Command::new("sh")
        .arg("-c")
        .arg(script)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap()
        .success();
    assert!(ok, "{script}");
}

/// A repo with `f.txt` = lines 1..=10, committed.
pub fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    sh(
        dir.path(),
        "git init -q && seq 1 10 > f.txt && git add f.txt && git commit -qm base",
    );
    dir
}

/// p.patch from `git diff` after running `edit` in the repo, which is then restored.
pub fn patch(dir: &Path, edit: &str) -> PathBuf {
    sh(
        dir,
        &format!("{edit} && git diff > p.patch && git checkout -q f.txt"),
    );
    dir.join("p.patch")
}

fn worktree_count(repo: &Path) -> usize {
    git(repo, &["worktree", "list"]).unwrap().lines().count()
}

#[test]
fn worktrees_and_applied_diff() {
    let repo = repo();
    let p = patch(repo.path(), "sed -i s/^2$/two/ f.txt");
    let ws = Workspace::new(repo.path(), "HEAD", Some(&p), None, false).unwrap();
    assert_eq!(
        std::fs::read_to_string(ws.base.join("f.txt")).unwrap(),
        "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n"
    );
    assert!(
        std::fs::read_to_string(ws.patched.join("f.txt"))
            .unwrap()
            .starts_with("1\ntwo\n3\n")
    );
    assert!(ws.applied_patch.starts_with("diff --git a/f.txt b/f.txt\n"));
    assert!(ws.applied_patch.contains("\n-2\n+two\n"));
    assert_eq!(ws.base_sha.len(), 40);
    assert!(ws.offset_paths.is_empty());
    let workdir = ws.workdir.clone();
    assert_eq!(worktree_count(repo.path()), 3);
    drop(ws);
    assert_eq!(worktree_count(repo.path()), 1);
    assert!(!workdir.exists());
}

#[test]
fn offset_hunks_are_reported() {
    let repo = repo();
    let p = patch(
        repo.path(),
        "sed -i s/^8$/eight/ f.txt && git diff > p.patch",
    );
    // Shift the hunk header by two lines, so git applies it at an offset.
    let text = std::fs::read_to_string(&p)
        .unwrap()
        .replace("@@ -5,6 +5,6 @@", "@@ -3,6 +3,6 @@");
    std::fs::write(&p, text).unwrap();
    let ws = Workspace::new(repo.path(), "HEAD", Some(&p), None, false).unwrap();
    assert_eq!(ws.offset_paths, ["f.txt"]);
    // The applied diff has the real line numbers.
    assert!(
        ws.applied_patch.contains("@@ -5,6 +5,6 @@"),
        "{}",
        ws.applied_patch
    );
}

#[test]
fn failures_clean_up() {
    let repo = repo();
    let bad = patch(repo.path(), "sed -i s/^2$/two/ f.txt");
    sh(
        repo.path(),
        "sed -i s/^2$/nope/ f.txt && git commit -qam nope",
    );
    let e = Workspace::new(repo.path(), "HEAD", Some(&bad), None, false)
        .err()
        .unwrap();
    assert_eq!(e.code, EXIT_PATCH);
    assert!(e.msg.starts_with("patch does not apply:\n"), "{}", e.msg);
    assert_eq!(worktree_count(repo.path()), 1);
    let e = Workspace::new(repo.path(), "no-such-ref", Some(&bad), None, false)
        .err()
        .unwrap();
    assert_eq!(e.code, crate::EXIT_USAGE);
    assert!(
        e.msg.starts_with("bad base ref 'no-such-ref': "),
        "{}",
        e.msg
    );
}

#[test]
fn keep_and_workdir() {
    let repo = repo();
    let work = tempfile::tempdir().unwrap();
    let p = patch(repo.path(), "sed -i s/^2$/two/ f.txt");
    let ws = Workspace::new(
        repo.path(),
        "HEAD",
        Some(&p),
        Some(&work.path().join("w")),
        true,
    )
    .unwrap();
    assert_eq!(ws.base, work.path().join("w/base"));
    drop(ws);
    assert!(work.path().join("w/patched/f.txt").is_file());
    assert_eq!(worktree_count(repo.path()), 3);
}

#[test]
fn dirty_and_root() {
    let repo = repo();
    assert!(!is_dirty(repo.path()).unwrap());
    std::fs::write(repo.path().join("new.txt"), "x").unwrap();
    assert!(!is_dirty(repo.path()).unwrap()); // untracked doesn't count
    std::fs::write(repo.path().join("f.txt"), "x").unwrap();
    assert!(is_dirty(repo.path()).unwrap());
    std::fs::create_dir(repo.path().join("sub")).unwrap();
    let root = repo_root(&repo.path().join("sub")).unwrap();
    assert_eq!(
        root.canonicalize().unwrap(),
        repo.path().canonicalize().unwrap()
    );
    assert!(repo_root(Path::new("/")).is_err());
}

#[test]
fn without_a_patch_both_trees_are_the_base() {
    let repo = repo();
    let ws = Workspace::new(repo.path(), "HEAD", None, None, false).unwrap();
    for side in [&ws.base, &ws.patched] {
        assert_eq!(
            std::fs::read_to_string(side.join("f.txt")).unwrap(),
            "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n"
        );
    }
    assert!(ws.applied_patch.is_empty());
    assert!(ws.offset_paths.is_empty());
    let workdir = ws.workdir.clone();
    drop(ws);
    assert!(!workdir.exists());
    assert_eq!(worktree_count(repo.path()), 1);
}
