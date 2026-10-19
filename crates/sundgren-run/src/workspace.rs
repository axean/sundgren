//! Git worktrees for the base and patched sides.

use std::fmt;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sundgren_core::pyint::py_strip;

use crate::pyfmt::repr_str;
use crate::pypath;

#[derive(Debug)]
pub enum WorkspaceError {
    /// git apply failed; the message is git's.
    PatchApply(String),
    /// A git command other than `git apply` failed; the message is git's.
    Git(String),
    /// The base ref doesn't name a commit (a kind of `Git`).
    BadRef(String),
    /// git couldn't be started, or a file system call failed.
    Io(io::Error),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceError::PatchApply(m) | WorkspaceError::Git(m) | WorkspaceError::BadRef(m) => {
                f.write_str(m)
            }
            WorkspaceError::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for WorkspaceError {}

impl From<io::Error> for WorkspaceError {
    fn from(e: io::Error) -> Self {
        WorkspaceError::Io(e)
    }
}

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

/// Base and patched worktrees. `enter` sets them up; `exit` removes them unless `keep`.
#[derive(Debug)]
pub struct Workspace {
    pub repo: PathBuf,
    pub base_ref: String,
    pub patch: PathBuf,
    pub keep: bool,
    given_workdir: Option<PathBuf>,
    created_workdir: bool,
    worktrees: Vec<PathBuf>,
    /// Holds base/, patched/ and out/.
    pub workdir: Option<PathBuf>,
    pub base: Option<PathBuf>,
    pub patched: Option<PathBuf>,
    pub base_sha: Option<String>,
    /// `git diff --cached` of patched/: what git apply actually changed. Raw bytes.
    pub applied_patch: Option<Vec<u8>>,
    /// Files with a hunk applied away from its header's line.
    pub offset_paths: Option<Vec<String>>,
}

impl Workspace {
    pub fn new(
        repo: PathBuf,
        base_ref: String,
        patch: PathBuf,
        workdir: Option<PathBuf>,
        keep: bool,
    ) -> Self {
        Workspace {
            repo,
            base_ref,
            patch,
            keep,
            given_workdir: workdir,
            created_workdir: false,
            worktrees: Vec::new(),
            workdir: None,
            base: None,
            patched: None,
            base_sha: None,
            applied_patch: None,
            offset_paths: None,
        }
    }

    /// Set up both worktrees; on failure everything is removed, even with `keep`.
    pub fn enter(&mut self) -> Result<(), WorkspaceError> {
        let result = self.setup();
        if result.is_err() {
            self.cleanup();
        }
        result
    }

    /// Remove the worktrees unless `keep`.
    pub fn exit(&mut self) {
        if !self.keep {
            self.cleanup();
        }
    }

    fn setup(&mut self) -> Result<(), WorkspaceError> {
        let spec = format!("{}^{{commit}}", self.base_ref);
        let sha = match git(&self.repo, &["rev-parse", "--verify", &spec]) {
            Ok(sha) => sha,
            Err(WorkspaceError::Git(msg)) => {
                return Err(WorkspaceError::BadRef(format!(
                    "bad base ref {}: {msg}",
                    repr_str(&self.base_ref)
                )));
            }
            Err(e) => return Err(e),
        };
        self.base_sha = Some(sha.clone());
        let workdir = match &self.given_workdir {
            None => {
                let dir = mkdtemp("sundgren-")?;
                self.created_workdir = true;
                dir
            }
            Some(given) => {
                let dir = pypath::absolute(given)?;
                std::fs::create_dir_all(&dir)?;
                dir
            }
        };
        self.workdir = Some(workdir.clone());
        self.base = Some(self.add_worktree(&workdir, "base", &sha)?);
        let patched = self.add_worktree(&workdir, "patched", &sha)?;
        self.patched = Some(patched.clone());
        self.offset_paths = Some(self.apply(&patched)?);
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(&patched)
            .args(DIFF_CONFIG)
            .args(DIFF_ARGS);
        let diff = cmd.output()?;
        if !diff.status.success() {
            let err = String::from_utf8_lossy(&diff.stderr);
            let msg = py_strip(&err);
            return Err(WorkspaceError::Git(if msg.is_empty() {
                "git diff failed".into()
            } else {
                msg.into()
            }));
        }
        self.applied_patch = Some(diff.stdout);
        Ok(())
    }

    /// git apply --index; return the paths git reports as applied at an offset.
    fn apply(&self, patched: &Path) -> Result<Vec<String>, WorkspaceError> {
        let patch = pypath::absolute(&self.patch)?;
        let proc = Command::new("git")
            .arg("-C")
            .arg(patched)
            .args(["-c", "core.quotePath=false"])
            .args(["apply", "--index", "--verbose", "--whitespace=nowarn"])
            .arg(&patch)
            .env("LC_ALL", "C") // the offset report below is parsed
            .output()?;
        let stderr = String::from_utf8_lossy(&proc.stderr);
        let lines: Vec<&str> = stderr.split('\n').collect();
        if !proc.status.success() {
            let noise = ["Checking patch ", "Hunk #", "Applied patch "];
            let message: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|ln| !ln.is_empty() && !noise.iter().any(|n| ln.starts_with(n)))
                .collect();
            return Err(WorkspaceError::PatchApply(if message.is_empty() {
                format!("git apply exited {}", exit_code(&proc))
            } else {
                message.join("\n")
            }));
        }
        let mut offset = Vec::new();
        let mut current: Option<&str> = None;
        for line in &lines {
            if let Some(rest) = line.strip_prefix("Checking patch ") {
                if let Some(path) = rest.strip_suffix("...") {
                    current = Some(path);
                }
            } else if line.starts_with("Hunk #") && line.contains(" succeeded at ") {
                if let Some(path) = current.filter(|p| !p.is_empty()) {
                    offset.push(path.to_owned());
                }
            }
        }
        offset.sort();
        offset.dedup();
        Ok(offset)
    }

    fn add_worktree(
        &mut self,
        workdir: &Path,
        name: &str,
        sha: &str,
    ) -> Result<PathBuf, WorkspaceError> {
        let path = workdir.join(name);
        let path_str = path.to_string_lossy();
        git(
            &self.repo,
            &["worktree", "add", "--detach", path_str.as_ref(), sha],
        )?;
        self.worktrees.push(path.clone());
        Ok(path)
    }

    fn cleanup(&mut self) {
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
            if let Some(dir) = &self.workdir {
                let _ = std::fs::remove_dir_all(dir);
            }
            self.created_workdir = false;
        }
    }
}

pub fn repo_root(path: &Path) -> Result<PathBuf, WorkspaceError> {
    Ok(PathBuf::from(git(path, &["rev-parse", "--show-toplevel"])?))
}

/// Tracked changes only; untracked files don't affect the base ref.
pub fn is_dirty(repo: &Path) -> Result<bool, WorkspaceError> {
    Ok(!git(repo, &["status", "--porcelain", "--untracked-files=no"])?.is_empty())
}

fn exit_code(out: &Output) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    out.status
        .code()
        .unwrap_or_else(|| -out.status.signal().unwrap_or(0))
}

fn git(repo: &Path, args: &[&str]) -> Result<String, WorkspaceError> {
    let proc = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if !proc.status.success() {
        let err = String::from_utf8_lossy(&proc.stderr);
        let msg = py_strip(&err);
        return Err(WorkspaceError::Git(if msg.is_empty() {
            format!("git {} exited {}", args[0], exit_code(&proc))
        } else {
            msg.into()
        }));
    }
    Ok(py_strip(&String::from_utf8_lossy(&proc.stdout)).to_owned())
}

/// `tempfile.mkdtemp(prefix=...)`: a new 0700 directory under `tempfile.gettempdir()`.
pub fn mkdtemp(prefix: &str) -> io::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_";
    let mut last_err = None;
    for dir in temp_candidates() {
        for _ in 0..100 {
            let mut h = std::hash::RandomState::new().build_hasher();
            h.write_u128(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos()),
            );
            let mut bits = h.finish();
            let name: String = (0..8)
                .map(|_| {
                    let c = CHARS[(bits % CHARS.len() as u64) as usize] as char;
                    bits /= CHARS.len() as u64;
                    c
                })
                .collect();
            let path = dir.join(format!("{prefix}{name}"));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(path),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    last_err = Some(e);
                    break;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::other("no usable temporary directory")))
}

/// `tempfile._candidate_tempdir_list()`, made absolute.
fn temp_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = ["TMPDIR", "TEMP", "TMP"]
        .iter()
        .filter_map(std::env::var_os)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .collect();
    out.extend(["/tmp", "/var/tmp", "/usr/tmp"].map(PathBuf::from));
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd);
    }
    out.into_iter()
        .filter_map(|p| pypath::absolute(&p).ok())
        .collect()
}
