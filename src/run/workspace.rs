//! Git worktrees for the base and patched sides.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cancel;
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

/// Base and patched worktrees under one work dir. Dropping it removes them unless `keep` (and
/// always after a cancelling signal).
pub struct Workspace {
    repo: PathBuf,
    keep: bool,
    created_workdir: bool,
    /// False for `patched_only`: base/ isn't checked out.
    with_base: bool,
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
        patch: Option<&Path>,
        workdir: Option<&Path>,
        keep: bool,
    ) -> Result<Self> {
        Self::create(repo, base_ref, patch, workdir, keep, true)
    }

    /// Only the patched worktree, in a temp dir: does the patch apply, and how.
    pub fn patched_only(repo: &Path, base_ref: &str, patch: Option<&Path>) -> Result<Self> {
        Self::create(repo, base_ref, patch, None, false, false)
    }

    fn create(
        repo: &Path,
        base_ref: &str,
        patch: Option<&Path>,
        workdir: Option<&Path>,
        keep: bool,
        with_base: bool,
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
            with_base,
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

    fn setup(&mut self, patch: Option<&Path>) -> Result<()> {
        let sides = if self.with_base {
            vec![self.base.clone(), self.patched.clone()]
        } else {
            vec![self.patched.clone()]
        };
        for side in sides {
            // Registered first, so a cancelled add is cleaned up too; a path that already
            // exists isn't ours to delete.
            let fresh = !side.exists();
            if fresh {
                self.worktrees.push(side.clone());
            }
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
            if !fresh {
                self.worktrees.push(side);
            }
        }
        let Some(patch) = patch else {
            return Ok(()); // both trees are the base; nothing applied
        };
        self.offset_paths = apply(&self.patched, &std::path::absolute(patch)?)?;
        let diff = cancel::output(
            Command::new("git")
                .arg("-C")
                .arg(&self.patched)
                .args(DIFF_CONFIG)
                .args(DIFF_ARGS),
        )?;
        if !diff.status.success() {
            return Err(git_error("diff", &diff));
        }
        self.applied_patch = String::from_utf8_lossy(&diff.stdout).into_owned();
        Ok(())
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let cancelled = cancel::signal().is_some();
        if self.keep && !cancelled {
            return;
        }
        if cancelled {
            self.move_to_trash();
        }
        for path in self.worktrees.drain(..).rev() {
            let removed = cleanup_git(&self.repo)
                .args(["worktree", "remove", "--force"])
                .arg(&path)
                .output()
                .is_ok_and(|o| o.status.success());
            if !removed {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
        let _ = cleanup_git(&self.repo).args(["worktree", "prune"]).output();
        if self.created_workdir {
            let _ = std::fs::remove_dir_all(&self.workdir);
        }
    }
}

impl Workspace {
    /// After a cancel: move what we created to a `sundgren-trash-*` sibling and delete it in the
    /// background, so we can exit at once. The prune in `drop` then deregisters the moved
    /// worktrees; anything that couldn't be moved is removed in place as usual.
    fn move_to_trash(&mut self) {
        if self.created_workdir {
            if let Some(trash) = trash(&self.workdir) {
                self.worktrees.clear();
                delete_in_background(&trash);
            }
            return;
        }
        self.worktrees.retain(|path| match trash(path) {
            Some(trash) => {
                delete_in_background(&trash);
                false
            }
            None => true,
        });
    }
}

/// Rename `path` to a fresh `sundgren-trash-*` next to it; None if that fails.
fn trash(path: &Path) -> Option<PathBuf> {
    let slot = tempfile::Builder::new()
        .prefix("sundgren-trash-")
        .tempdir_in(path.parent()?)
        .ok()?
        .keep();
    // Renaming onto an empty directory replaces it.
    match std::fs::rename(path, &slot) {
        Ok(()) => Some(slot),
        Err(_) => {
            let _ = std::fs::remove_dir(&slot);
            None
        }
    }
}

/// `rm -rf` in its own session, not waited for; in place if it can't start.
fn delete_in_background(path: &Path) {
    let mut rm = Command::new("rm");
    rm.arg("-rf")
        .arg("--")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // SAFETY: setsid is async-signal-safe.
    unsafe {
        rm.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    if rm.spawn().is_err() {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// git in `repo`, in its own process group so a Ctrl-C in the terminal doesn't interrupt the
/// cleanup. Not cancellable.
fn cleanup_git(repo: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).process_group(0);
    cmd
}

/// git apply --index; returns the paths git reports as applied at an offset.
fn apply(patched: &Path, patch: &Path) -> Result<Vec<String>> {
    let proc = cancel::output(
        Command::new("git")
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
            .env("LC_ALL", "C"), // the offset report below is parsed
    )?;
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
    let proc = cancel::output(Command::new("git").arg("-C").arg(repo).args(args))?;
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
pub(crate) mod tests;
