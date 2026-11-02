//! Two repositories as one, without touching either: a bare repository in the cache that borrows
//! both object stores (`objects/info/alternates`) and mirrors both repositories' refs.
//!
//! This repository's branches, remote branches, tags and HEAD are kept as they are. The other
//! one's look like a remote named [`Join::name`]: its branches are `refs/remotes/NAME/*`, its
//! remote branches `refs/remotes/NAME/REMOTE/*`, its tags `refs/tags/NAME/*`. So anything that
//! reads one repository (`git log --all`, [`crate::map`]) reads both in [`Join::dir`].
//!
//! [`join`] syncs the refs on every call, dropping those that are gone. Nothing is written to
//! either repository.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::{Error, Result, git};

/// Which repository a ref of the join comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    This,
    Other,
}

/// A ref of the join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedRef {
    /// Its full name in the join.
    pub name: String,
    pub source: Source,
    /// Its full name in the repository it comes from.
    pub original: String,
    /// The object it points at, and the commit (a tag peeled once).
    pub oid: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Join {
    /// The bare repository holding both: run git here.
    pub dir: PathBuf,
    /// The other repository's remote name in the join.
    pub name: String,
    /// The directories given: for what only the repositories themselves have (work trees,
    /// config, the current branch).
    pub this_repo: PathBuf,
    pub other_repo: PathBuf,
    /// Their git directories (shared by all work trees), absolute.
    pub this: PathBuf,
    pub other: PathBuf,
    /// The refs mirrored, this repository's first. HEAD is not one of them.
    pub refs: Vec<JoinedRef>,
}

impl Join {
    /// The name in the join of the other repository's ref `full` (`refs/heads/main` ->
    /// `refs/remotes/NAME/main`). None for refs that are not mirrored.
    pub fn other_ref(&self, full: &str) -> Option<String> {
        other_ref(&self.name, full)
    }
}

fn other_ref(name: &str, full: &str) -> Option<String> {
    if let Some(rest) = full
        .strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/remotes/"))
    {
        return Some(format!("refs/remotes/{name}/{rest}"));
    }
    full.strip_prefix("refs/tags/")
        .map(|rest| format!("refs/tags/{name}/{rest}"))
}

/// The git directory shared by `repo`'s work trees, absolute.
fn common_dir(repo: &Path) -> Result<PathBuf> {
    let mut cmd = git::git(repo);
    cmd.args(["rev-parse", "--path-format=absolute", "--git-common-dir"]);
    let dir = PathBuf::from(git::output(cmd)?.trim_end_matches('\n'));
    Ok(dir.canonicalize().unwrap_or(dir))
}

/// `text` as one ref name component: letters, digits, `.`, `_` and `-`; anything else as `-`.
pub fn ref_safe(text: &str) -> String {
    let name: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let name = name.replace("..", "-");
    let name = name.trim_start_matches(['.', '-']);
    let name = name.strip_suffix(".lock").unwrap_or(name);
    name.trim_end_matches('.').to_owned()
}

/// The default name of a repository with git directory `dir`: its folder's name.
fn folder_name(dir: &Path) -> String {
    let folder = if dir.file_name().is_some_and(|n| n == ".git") {
        dir.parent().unwrap_or(dir)
    } else {
        dir
    };
    let name = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = ref_safe(name.strip_suffix(".git").unwrap_or(&name));
    if name.is_empty() {
        "other".to_owned()
    } else {
        name
    }
}

/// 64-bit FNV-1a: a stable hash for the cache directory's name.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Where joins are kept: `$XDG_CACHE_HOME/sundgren/join`, else `~/.cache/sundgren/join`.
fn cache_root() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .ok_or_else(|| Error::failure("no cache directory: set XDG_CACHE_HOME or HOME"))?;
    Ok(base.join("sundgren").join("join"))
}

/// (full name, object, peeled object) of the branches, remote branches and tags of the
/// repository at `dir`. Symbolic refs (`origin/HEAD`) are left out.
fn list_refs(dir: &Path) -> Result<Vec<(String, String, String)>> {
    let mut cmd = git::git(dir);
    cmd.args([
        "for-each-ref",
        "--format=%(refname)%1f%(objectname)%1f%(*objectname)%1f%(symref)",
        "refs/heads",
        "refs/remotes",
        "refs/tags",
    ]);
    Ok(git::output(cmd)?
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\x1f').collect();
            let [full, oid, peeled, symref] = fields[..] else {
                return None;
            };
            let peeled = if peeled.is_empty() { oid } else { peeled };
            symref
                .is_empty()
                .then(|| (full.to_owned(), oid.to_owned(), peeled.to_owned()))
        })
        .collect())
}

/// Join `repo` and `other` in the cache, `other` named `name` (default: its folder's name).
pub fn join(repo: &Path, other: &Path, name: Option<&str>) -> Result<Join> {
    join_in(&cache_root()?, repo, other, name)
}

/// As [`join`], in the directory `cache` instead of the user's cache.
pub fn join_in(cache: &Path, repo: &Path, other: &Path, name: Option<&str>) -> Result<Join> {
    let this = common_dir(repo)?;
    let that = common_dir(other)?;
    if this == that {
        return Err(Error::usage(format!(
            "{} is this repository",
            other.display()
        )));
    }
    let name = match name {
        Some(n) if ref_safe(n) != n || n.is_empty() => {
            return Err(Error::usage(format!(
                "--name must be letters, digits, '.', '_' or '-', not {n:?}"
            )));
        }
        Some(n) => n.to_owned(),
        None => folder_name(&that),
    };
    let mut remotes = git::git(repo);
    remotes.arg("remote");
    let this_refs = list_refs(repo)?;
    let prefix = format!("refs/remotes/{name}/");
    if git::output(remotes)?.lines().any(|r| r == name)
        || this_refs
            .iter()
            .any(|(full, _, _)| full.starts_with(&prefix))
    {
        return Err(Error::usage(format!(
            "this repository already has a remote {name:?}; name the other one with --name"
        )));
    }

    let key = format!("{}\0{}", this.display(), that.display());
    let dir = cache.join(format!("{:016x}", fnv1a(key.as_bytes())));
    if !dir.join("HEAD").is_file() {
        std::fs::create_dir_all(&dir)?;
        let mut init = git::git(&dir);
        init.args(["init", "--quiet", "--bare", "."]);
        git::output(init)?;
    }
    for (key, value) in [("gc.auto", "0"), ("core.logAllRefUpdates", "false")] {
        let mut config = git::git(&dir);
        config.args(["config", key, value]);
        git::output(config)?;
    }
    let objects = |d: &Path| d.join("objects").display().to_string();
    std::fs::write(
        dir.join("objects").join("info").join("alternates"),
        format!("{}\n{}\n", objects(&this), objects(&that)),
    )?;
    std::fs::write(
        dir.join("description"),
        format!(
            "sundgren join of {} and {} (as {name})\n",
            this.display(),
            that.display()
        ),
    )?;

    let mut refs: Vec<JoinedRef> = Vec::new();
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mirrored = this_refs
        .into_iter()
        .map(|(full, oid, commit)| (Source::This, full.clone(), full, oid, commit))
        .chain(
            list_refs(other)?
                .into_iter()
                .filter_map(|(full, oid, commit)| {
                    let joined = other_ref(&name, &full)?;
                    Some((Source::Other, joined, full, oid, commit))
                }),
        );
    for (source, joined, original, oid, commit) in mirrored {
        if taken.insert(joined.clone()) {
            refs.push(JoinedRef {
                name: joined,
                source,
                original,
                oid,
                commit,
            });
        }
    }
    sync_refs(&dir, &refs)?;
    sync_head(repo, &dir)?;
    Ok(Join {
        dir,
        name,
        this_repo: repo.to_path_buf(),
        other_repo: other.to_path_buf(),
        this,
        other: that,
        refs,
    })
}

/// Make the join's refs (HEAD aside) exactly `refs`: stale ones first, in their own
/// transaction, so a ref can become a directory of refs and back.
fn sync_refs(dir: &Path, refs: &[JoinedRef]) -> Result<()> {
    let mut list = git::git(dir);
    list.args(["for-each-ref", "--format=%(refname) %(objectname)"]);
    let have: BTreeMap<String, String> = git::output(list)?
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(r, o)| (r.to_owned(), o.to_owned()))
        .collect();
    let want: BTreeMap<&str, &str> = refs
        .iter()
        .map(|r| (r.name.as_str(), r.oid.as_str()))
        .collect();
    let deletes: String = have
        .keys()
        .filter(|r| !want.contains_key(r.as_str()))
        .map(|r| format!("delete {r}\n"))
        .collect();
    let updates: String = want
        .iter()
        .filter(|(r, o)| have.get(**r).is_none_or(|h| h != *o))
        .map(|(r, o)| format!("update {r} {o}\n"))
        .collect();
    for input in [deletes, updates] {
        if !input.is_empty() {
            let mut cmd = git::git(dir);
            cmd.args(["update-ref", "--stdin"]);
            git::output_with_input(cmd, input)?;
        }
    }
    Ok(())
}

/// Point the join's HEAD where `repo`'s is: at the same branch, or detached at the same commit.
fn sync_head(repo: &Path, dir: &Path) -> Result<()> {
    let mut cmd = git::git(dir);
    if let Some(branch) = git::current_branch(repo)? {
        cmd.args(["symbolic-ref", "HEAD"])
            .arg(format!("refs/heads/{branch}"));
    } else if let Ok(oid) = git::resolve(repo, "HEAD") {
        cmd.args(["update-ref", "--no-deref", "HEAD", &oid]);
    } else {
        return Ok(());
    }
    git::output(cmd).map(|_| ())
}

#[cfg(test)]
mod tests;
