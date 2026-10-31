//! Reading history with the git CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Person {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefKind {
    /// `HEAD`: the branch it points at follows as a separate `branch` ref.
    Head,
    Branch,
    Remote,
    Tag,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ref {
    /// Short name: `main`, `origin/main`, `v1.0`, `HEAD`.
    pub name: String,
    pub kind: RefKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Commit {
    pub oid: String,
    pub parents: Vec<String>,
    pub author: Person,
    /// Unix seconds.
    pub authored: i64,
    pub committed: i64,
    pub subject: String,
    pub refs: Vec<Ref>,
}

const FIELD: char = '\x1f';
const FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%ct%x1f%s%x1f%D";

fn git(repo: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("--no-pager")
        .args(["-c", "log.showSignature=false", "-c", "color.ui=false"])
        .arg("-C")
        .arg(repo);
    cmd
}

fn output(mut cmd: Command) -> Result<String> {
    let out = cmd
        .output()
        .map_err(|e| Error::failure(format!("can't run git: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let msg = stderr.trim().trim_start_matches("fatal: ");
        return Err(Error::failure(format!("git: {msg}")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The top of the work tree that contains `repo`.
pub fn toplevel(repo: &Path) -> Result<PathBuf> {
    let mut cmd = git(repo);
    cmd.args(["rev-parse", "--show-toplevel"]);
    Ok(PathBuf::from(output(cmd)?.trim_end_matches('\n')))
}

/// The commit `rev` names.
pub fn resolve(repo: &Path, rev: &str) -> Result<String> {
    let mut cmd = git(repo);
    cmd.args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
        .arg(format!("{rev}^{{commit}}"));
    match output(cmd) {
        Ok(oid) => Ok(oid.trim().to_owned()),
        Err(_) => Err(Error::usage(format!("{rev} is not a commit"))),
    }
}

/// Up to `limit` commits reachable from `oid`, children before parents.
pub fn log(repo: &Path, oid: &str, limit: usize) -> Result<Vec<Commit>> {
    let mut cmd = git(repo);
    cmd.args(["log", "-z", "--topo-order", "--decorate=full", FORMAT])
        .arg(format!("--max-count={limit}"))
        .arg(oid)
        .arg("--");
    parse_log(&output(cmd)?)
}

/// The merge base of two commits, if they have one.
pub fn merge_base(repo: &Path, a: &str, b: &str) -> Result<Option<String>> {
    let mut cmd = git(repo);
    cmd.args(["merge-base", "--end-of-options", a, b]);
    match cmd.output() {
        Ok(out) if out.status.success() => {
            Ok(Some(String::from_utf8_lossy(&out.stdout).trim().to_owned()))
        }
        // Exit 1 without output: no common ancestor.
        Ok(out) if out.status.code() == Some(1) && out.stdout.is_empty() => Ok(None),
        Ok(out) => Err(Error::failure(format!(
            "git: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
        Err(e) => Err(Error::failure(format!("can't run git: {e}"))),
    }
}

/// Lines a file gained between two commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLines {
    pub path: String,
    /// (line number in the new file, text), in file order.
    pub added: Vec<(u32, String)>,
}

/// The lines each file gained from `from` to `to` (`git diff -U0`, no rename detection).
pub fn added_lines(repo: &Path, from: &str, to: &str) -> Result<Vec<FileLines>> {
    let mut cmd = git(repo);
    cmd.args(["-c", "core.quotePath=false", "diff", "-U0", "--no-ext-diff"])
        .args([
            "--no-renames",
            "--no-textconv",
            "--end-of-options",
            from,
            to,
        ]);
    Ok(parse_added(&output(cmd)?))
}

/// Added lines from `git diff -U0` output.
pub fn parse_added(diff: &str) -> Vec<FileLines> {
    let mut files: Vec<FileLines> = Vec::new();
    let mut current: Option<FileLines> = None;
    let mut number: u32 = 0;
    // Lines still to come in the current hunk: (removed, added). Between hunks both are 0, and
    // only then is a line a header.
    let mut left = (0u32, 0u32);
    for line in diff.lines() {
        if left != (0, 0) {
            if let Some(text) = line.strip_prefix('+') {
                left.1 = left.1.saturating_sub(1);
                if let Some(file) = current.as_mut() {
                    file.added.push((number, text.to_owned()));
                }
                number += 1;
            } else if line.starts_with('-') {
                left.0 = left.0.saturating_sub(1);
            }
            // `\ No newline at end of file` and anything else: not counted.
            continue;
        }
        if line.starts_with("diff --git ") {
            files.extend(current.take().filter(|f| !f.added.is_empty()));
        } else if let Some(path) = line.strip_prefix("+++ ") {
            // `+++ /dev/null` is a deleted file: nothing added.
            current = path.strip_prefix("b/").map(|path| FileLines {
                path: path.to_owned(),
                added: Vec::new(),
            });
        } else if let Some(header) = line.strip_prefix("@@ ")
            && let Some((removed, start, added)) = hunk_header(header)
        {
            number = start;
            left = (removed, added);
        }
    }
    files.extend(current.filter(|f| !f.added.is_empty()));
    files
}

/// (removed lines, first new line, added lines) from a hunk header `-a,b +c,d @@ ...`; a
/// missing count is 1.
fn hunk_header(header: &str) -> Option<(u32, u32, u32)> {
    let mut words = header.split_whitespace();
    let range = |word: &str| -> Option<(u32, u32)> {
        let mut parts = word.split(',');
        let start = parts.next()?.parse().ok()?;
        let count = parts.next().map_or(Some(1), |c| c.parse().ok())?;
        Some((start, count))
    };
    let (_, removed) = range(words.next()?.strip_prefix('-')?)?;
    let (start, added) = range(words.next()?.strip_prefix('+')?)?;
    Some((removed, start, added))
}

/// Commits from `git log -z` with [`FORMAT`].
pub fn parse_log(out: &str) -> Result<Vec<Commit>> {
    out.split('\0')
        .filter(|record| !record.trim().is_empty())
        .map(parse_commit)
        .collect()
}

fn parse_commit(record: &str) -> Result<Commit> {
    let record = record.strip_prefix('\n').unwrap_or(record);
    let fields: Vec<&str> = record.split(FIELD).collect();
    let [
        oid,
        parents,
        name,
        email,
        authored,
        committed,
        subject,
        refs,
    ] = fields[..]
    else {
        return Err(Error::failure(format!(
            "unexpected git log record: {record:?}"
        )));
    };
    let time = |s: &str| {
        s.parse::<i64>()
            .map_err(|_| Error::failure(format!("bad time {s:?} in commit {oid}")))
    };
    Ok(Commit {
        oid: oid.to_owned(),
        parents: parents.split_whitespace().map(str::to_owned).collect(),
        author: Person {
            name: name.to_owned(),
            email: email.to_owned(),
        },
        authored: time(authored)?,
        committed: time(committed)?,
        subject: subject.to_owned(),
        refs: parse_refs(refs),
    })
}

/// Refs from `%D` with `--decorate=full`: `HEAD -> refs/heads/main, tag: refs/tags/v1, ...`.
pub fn parse_refs(decoration: &str) -> Vec<Ref> {
    let mut refs = Vec::new();
    for item in decoration.split(", ").filter(|s| !s.is_empty()) {
        if let Some(branch) = item.strip_prefix("HEAD -> ") {
            refs.push(head());
            refs.push(classify(branch));
        } else if item == "HEAD" {
            refs.push(head());
        } else if let Some(tag) = item.strip_prefix("tag: ") {
            refs.push(classify(tag));
        } else {
            refs.push(classify(item));
        }
    }
    refs
}

fn head() -> Ref {
    Ref {
        name: "HEAD".to_owned(),
        kind: RefKind::Head,
    }
}

fn classify(full: &str) -> Ref {
    let kinds = [
        ("refs/heads/", RefKind::Branch),
        ("refs/remotes/", RefKind::Remote),
        ("refs/tags/", RefKind::Tag),
    ];
    for (prefix, kind) in kinds {
        if let Some(name) = full.strip_prefix(prefix) {
            return Ref {
                name: name.to_owned(),
                kind,
            };
        }
    }
    Ref {
        name: full.strip_prefix("refs/").unwrap_or(full).to_owned(),
        kind: RefKind::Other,
    }
}

#[cfg(test)]
mod tests;
