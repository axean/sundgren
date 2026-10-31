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
