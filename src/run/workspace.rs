//! Git worktrees for the base and patched sides.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{EXIT_PATCH, Error, Result};

// Pin the shape of the applied diff against user config (prefixes, colour, external tools).
const DIFF_CONFIG: [&str; 8] = [
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "diff.relative=false",
    "-c",
    "core.quotePath=true",
];
const DIFF_ARGS: [&str; 13] = [
    "diff",
    "--cached",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "-M",
    "--unified=3",
    "--inter-hunk-context=0",
    "--diff-algorithm=myers",
    "--indent-heuristic",
    "--submodule=short",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

/// Base and patched worktrees under one work dir. Dropping it removes them unless `keep`.
pub struct Workspace {
    repo: PathBuf,
    keep: bool,
    created_workdir: bool,
    worktrees: Vec<PathBuf>,
    /// Holds base/, patched/ and out/.
    pub workdir: PathBuf,
    pub base: PathBuf,
    pub patched: PathBuf,
    pub base_sha: String,
    /// `git diff --cached` of patched/: what git apply actually changed.
    pub applied_patch: String,
    /// Files with a hunk applied away from its header's line.
    pub offset_paths: Vec<String>,
}

impl Workspace {
    /// Check out `base_ref` twice and apply `patch` to the second; on failure nothing is left.
    pub fn new(
        repo: &Path,
        base_ref: &str,
        patch: &Path,
        workdir: Option<&Path>,
        keep: bool,
    ) -> Result<Self> {
        let sha = git(
            repo,
            &["rev-parse", "--verify", &format!("{base_ref}^{{commit}}")],
        )
        .map_err(|e| Error::usage(format!("bad base ref '{base_ref}': {}", e.msg)))?;
        let (workdir, created_workdir) = match workdir {
            None => (
                tempfile::Builder::new()
                    .prefix("sundgren-")
                    .tempdir()?
                    .keep(),
                true,
            ),
            Some(dir) => {
                let dir = std::path::absolute(dir)?;
                std::fs::create_dir_all(&dir)?;
                (dir, false)
            }
        };
        let mut ws = Workspace {
            repo: repo.to_path_buf(),
            keep,
            created_workdir,
            worktrees: Vec::new(),
            base: workdir.join("base"),
            patched: workdir.join("patched"),
            workdir,
            base_sha: sha,
            applied_patch: String::new(),
            offset_paths: Vec::new(),
        };
        if let Err(e) = ws.setup(patch) {
            ws.keep = false; // drop cleans up
            return Err(e);
        }
        Ok(ws)
    }

    fn setup(&mut self, patch: &Path) -> Result<()> {
        for side in [self.base.clone(), self.patched.clone()] {
            git(
                &self.repo,
                &[
                    "worktree",
                    "add",
                    "--detach",
                    &side.to_string_lossy(),
                    &self.base_sha,
                ],
            )?;
            self.worktrees.push(side);
        }
        self.offset_paths = apply(&self.patched, &std::path::absolute(patch)?)?;
        let diff = Command::new("git")
            .arg("-C")
            .arg(&self.patched)
            .args(DIFF_CONFIG)
            .args(DIFF_ARGS)
            .output()?;
        if !diff.status.success() {
            return Err(git_error("diff", &diff));
        }
        self.applied_patch = String::from_utf8_lossy(&diff.stdout).into_owned();
        Ok(())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        for path in self.worktrees.drain(..).rev() {
            let removed = Command::new("git")
                .arg("-C")
                .arg(&self.repo)
                .args(["worktree", "remove", "--force"])
                .arg(&path)
                .output()
                .is_ok_and(|o| o.status.success());
            if !removed {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
        let _ = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(["worktree", "prune"])
            .output();
        if self.created_workdir {
            let _ = std::fs::remove_dir_all(&self.workdir);
        }
    }
}

/// git apply --index; returns the paths git reports as applied at an offset.
fn apply(patched: &Path, patch: &Path) -> Result<Vec<String>> {
    let proc = Command::new("git")
        .arg("-C")
        .arg(patched)
        .args([
            "-c",
            "core.quotePath=false",
            "apply",
            "--index",
            "--verbose",
            "--whitespace=nowarn",
        ])
        .arg(patch)
        .env("LC_ALL", "C") // the offset report below is parsed
        .output()?;
    let stderr = String::from_utf8_lossy(&proc.stderr);
    if !proc.status.success() {
        let noise = ["Checking patch ", "Hunk #", "Applied patch "];
        let message: Vec<&str> = stderr
            .lines()
            .filter(|ln| !ln.is_empty() && !noise.iter().any(|n| ln.starts_with(n)))
            .collect();
        let message = if message.is_empty() {
            format!("git apply exited {}", proc.status)
        } else {
            message.join("\n")
        };
        return Err(Error {
            code: EXIT_PATCH,
            msg: format!("patch does not apply:\n{message}"),
        });
    }
    let mut offset = Vec::new();
    let mut current = None;
    for line in stderr.lines() {
        if let Some(path) = line
            .strip_prefix("Checking patch ")
            .and_then(|r| r.strip_suffix("..."))
        {
            current = Some(path);
        } else if line.starts_with("Hunk #")
            && line.contains(" succeeded at ")
            && let Some(path) = current.filter(|p| !p.is_empty())
        {
            offset.push(path.to_owned());
        }
    }
    offset.sort();
    offset.dedup();
    Ok(offset)
}

pub fn repo_root(path: &Path) -> Result<PathBuf> {
    Ok(PathBuf::from(git(path, &["rev-parse", "--show-toplevel"])?))
}

/// Tracked changes only; untracked files don't affect the base ref.
pub fn is_dirty(repo: &Path) -> Result<bool> {
    Ok(!git(repo, &["status", "--porcelain", "--untracked-files=no"])?.is_empty())
}

/// Run git in `repo`; stdout trimmed. A failure is `git: <stderr>` (exit 1).
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let proc = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if !proc.status.success() {
        return Err(git_error(args[0], &proc));
    }
    Ok(String::from_utf8_lossy(&proc.stdout).trim().to_owned())
}

fn git_error(what: &str, proc: &std::process::Output) -> Error {
    let err = String::from_utf8_lossy(&proc.stderr);
    let msg = match err.trim() {
        "" => format!("git {what} exited {}", proc.status),
        msg => msg.to_owned(),
    };
    Error::failure(format!("git: {msg}"))
}

#[cfg(test)]
pub(crate) mod tests {
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
        let ws = Workspace::new(repo.path(), "HEAD", &p, None, false).unwrap();
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
        let ws = Workspace::new(repo.path(), "HEAD", &p, None, false).unwrap();
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
        let e = Workspace::new(repo.path(), "HEAD", &bad, None, false)
            .err()
            .unwrap();
        assert_eq!(e.code, EXIT_PATCH);
        assert!(e.msg.starts_with("patch does not apply:\n"), "{}", e.msg);
        assert_eq!(worktree_count(repo.path()), 1);
        let e = Workspace::new(repo.path(), "no-such-ref", &bad, None, false)
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
        let ws =
            Workspace::new(repo.path(), "HEAD", &p, Some(&work.path().join("w")), true).unwrap();
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
}
