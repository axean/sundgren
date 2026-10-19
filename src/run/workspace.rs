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
pub(crate) mod tests;
