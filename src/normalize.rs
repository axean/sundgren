//! Path normalization: raw SF paths -> repo-relative posix paths.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::model::{FileCoverage, Snapshot, merge_file_coverage};

/// `os.path.realpath`, non-strict: symlinks resolved, missing tails kept as they are.
pub fn realpath(path: &str) -> String {
    let p = Path::new(path);
    let mut out = if p.is_absolute() {
        PathBuf::from("/")
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    };
    for c in p.components() {
        match c {
            Component::RootDir | Component::Prefix(_) | Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(name) => {
                out.push(name);
                if out.is_symlink()
                    && let Ok(real) = out.canonicalize()
                {
                    out = real;
                }
            }
        }
    }
    out.to_string_lossy().into_owned()
}

/// (anchor, parts); anchor is "" (relative), "/" or a drive like "C:/".
type Split = (String, Vec<String>);

/// Lexically clean a path: backslashes to '/', drop '.', resolve '..'.
fn split(path: &str) -> Split {
    let p = path.replace('\\', "/");
    let b = p.as_bytes();
    let (anchor, rest) = if b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'/')
    {
        (format!("{}:/", (b[0] as char).to_ascii_uppercase()), &p[2..])
    } else if p.starts_with('/') {
        ("/".to_string(), p.as_str())
    } else {
        (String::new(), p.as_str())
    };
    let mut parts: Vec<String> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|l| l != "..") {
                    parts.pop();
                } else if anchor.is_empty() {
                    // Escapes the tree; '..' above an absolute root is dropped.
                    parts.push("..".into());
                }
            }
            _ => parts.push(part.to_string()),
        }
    }
    (anchor, parts)
}

fn join((anchor, parts): &Split) -> String {
    if anchor.is_empty() && parts.is_empty() {
        ".".into()
    } else {
        format!("{anchor}{}", parts.join("/"))
    }
}

/// Clean roots, add realpaths of absolute ones, longest first.
fn prepare<S: AsRef<str>>(roots: &[S]) -> Vec<Split> {
    let mut out: BTreeSet<Split> = BTreeSet::new();
    for root in roots {
        let root = root.as_ref();
        let s = split(root);
        if s.0.is_empty() && s.1.is_empty() {
            continue;
        }
        out.insert(s);
        if root.starts_with('/') {
            out.insert(split(&realpath(root)));
        }
    }
    let mut v: Vec<Split> = out.into_iter().collect();
    v.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.cmp(b)));
    v
}

/// The path below the longest matching root, matching on whole components.
fn strip(s: &Split, roots: &[Split]) -> Option<String> {
    let (anchor, parts) = s;
    roots
        .iter()
        .find(|(ra, rp)| ra == anchor && parts.len() > rp.len() && parts.starts_with(rp))
        .map(|(_, rp)| parts[rp.len()..].join("/"))
}

fn normalize(raw: &str, roots: &[Split]) -> (String, bool) {
    let s = split(raw);
    let cleaned = join(&s);
    let mut rest = strip(&s, roots);
    if rest.is_none() && cleaned.starts_with('/') {
        // /tmp vs /private/tmp and the like.
        rest = strip(&split(&realpath(&cleaned)), roots);
    }
    if let Some(rest) = rest {
        return (rest, false);
    }
    let (anchor, parts) = &s;
    let inside = anchor.is_empty() && parts.first().is_some_and(|p| p != "..");
    (cleaned, !inside)
}

/// Return (path, external). Strips the longest matching root; external if unresolvable.
pub fn normalize_path<S: AsRef<str>>(raw: &str, roots: &[S]) -> (String, bool) {
    normalize(raw, &prepare(roots))
}

/// Re-key a Snapshot by normalized path, merging records that collide.
pub fn normalize_snapshot<S: AsRef<str>>(snapshot: &Snapshot, roots: &[S]) -> Snapshot {
    let prepared = prepare(roots);
    let mut files: BTreeMap<String, FileCoverage> = BTreeMap::new();
    for (raw, fc) in &snapshot.files {
        let (path, external) = normalize(raw, &prepared);
        let fc = FileCoverage {
            path: path.clone(),
            external,
            ..fc.clone()
        };
        let merged = match files.remove(&path) {
            None => fc,
            Some(old) => merge_file_coverage(&old, &fc),
        };
        files.insert(path, merged);
    }
    Snapshot {
        files,
        test_names: snapshot.test_names.clone(),
    }
}
