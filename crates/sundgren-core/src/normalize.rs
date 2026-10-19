//! Path normalization: raw SF paths -> repo-relative posix paths. Port of
//! `sundgren/core/normalize.py`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsStr;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

use crate::model::{FileCoverage, Snapshot, merge_file_coverage};

/// `os.path.realpath(path)` (non-strict, POSIX), as Python 3.11/3.12 compute it: symlinks
/// resolved, missing tails kept, a symlink loop ends resolution there. A result that isn't
/// UTF-8 is converted lossily.
pub fn realpath(path: &str) -> String {
    let mut seen = HashMap::new();
    let (p, _) = join_realpath(Vec::new(), path.as_bytes(), &mut seen);
    String::from_utf8_lossy(&abspath(&p)).into_owned()
}

fn is_link(path: &[u8]) -> bool {
    std::fs::symlink_metadata(OsStr::from_bytes(path)).is_ok_and(|m| m.file_type().is_symlink())
}

/// `posixpath.join(a, b)`.
fn pjoin(a: &[u8], b: &[u8]) -> Vec<u8> {
    if b.starts_with(b"/") {
        return b.to_vec();
    }
    let mut out = a.to_vec();
    if !out.is_empty() && !out.ends_with(b"/") {
        out.push(b'/');
    }
    out.extend_from_slice(b);
    out
}

/// `posixpath.split(p)`.
fn psplit(p: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let i = p.iter().rposition(|&b| b == b'/').map_or(0, |i| i + 1);
    let (head, tail) = p.split_at(i);
    let mut head = head.to_vec();
    if !head.is_empty() && head.iter().any(|&b| b != b'/') {
        while head.ends_with(b"/") {
            head.pop();
        }
    }
    (head, tail.to_vec())
}

/// `posixpath._joinrealpath` from Python 3.12.
fn join_realpath(
    mut path: Vec<u8>,
    rest: &[u8],
    seen: &mut HashMap<Vec<u8>, Option<Vec<u8>>>,
) -> (Vec<u8>, bool) {
    let mut rest = rest;
    if rest.starts_with(b"/") {
        rest = &rest[1..];
        path = b"/".to_vec();
    }
    while !rest.is_empty() {
        let (name, tail) = match rest.iter().position(|&b| b == b'/') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        rest = tail;
        if name.is_empty() || name == b"." {
            continue;
        }
        if name == b".." {
            if path.is_empty() {
                path = b"..".to_vec();
            } else {
                let (head, last) = psplit(&path);
                path = if last == b".." {
                    pjoin(&pjoin(&head, b".."), b"..")
                } else {
                    head
                };
            }
            continue;
        }
        let newpath = pjoin(&path, name);
        if !is_link(&newpath) {
            path = newpath;
            continue;
        }
        if let Some(cached) = seen.get(&newpath) {
            match cached {
                Some(resolved) => {
                    path = resolved.clone();
                    continue;
                }
                // A loop: return the resolved part plus the rest unchanged.
                None => return (pjoin(&newpath, rest), false),
            }
        }
        seen.insert(newpath.clone(), None);
        let target = match std::fs::read_link(OsStr::from_bytes(&newpath)) {
            Ok(t) => t.into_os_string().into_vec(),
            Err(_) => {
                path = newpath;
                continue;
            }
        };
        let (resolved, ok) = join_realpath(path, &target, seen);
        if !ok {
            return (pjoin(&resolved, rest), false);
        }
        seen.insert(newpath, Some(resolved.clone()));
        path = resolved;
    }
    (path, true)
}

/// `posixpath.abspath(p)`.
fn abspath(p: &[u8]) -> Vec<u8> {
    let full = if p.starts_with(b"/") {
        p.to_vec()
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.into_os_string().into_vec())
            .unwrap_or_else(|_| b"/".to_vec());
        pjoin(&cwd, p)
    };
    normpath(&full)
}

/// `posixpath.normpath(p)`.
fn normpath(p: &[u8]) -> Vec<u8> {
    if p.is_empty() {
        return b".".to_vec();
    }
    let initial = if p.starts_with(b"//") && !p.starts_with(b"///") {
        2
    } else if p.starts_with(b"/") {
        1
    } else {
        0
    };
    let mut parts: Vec<&[u8]> = Vec::new();
    for comp in p.split(|&b| b == b'/') {
        if comp.is_empty() || comp == b"." {
            continue;
        }
        if comp != b".." || (initial == 0 && parts.is_empty()) || parts.last() == Some(&&b".."[..])
        {
            parts.push(comp);
        } else if !parts.is_empty() {
            parts.pop();
        }
    }
    let mut out = vec![b'/'; initial];
    out.extend_from_slice(&parts.join(&b'/'));
    if out.is_empty() { b".".to_vec() } else { out }
}

/// (anchor, parts); anchor is "" (relative), "/" or a drive like "C:/".
type Split = (String, Vec<String>);

/// Lexically clean a path: backslashes to '/', drop '.', resolve '..'.
fn split(path: &str) -> Split {
    let p = path.replace('\\', "/");
    let mut chars = p.chars();
    let (c0, c1, c2) = (chars.next(), chars.next(), chars.next());
    let (anchor, rest) = match (c0, c1, c2) {
        (Some(d), Some(':'), None | Some('/')) if d.is_ascii_alphabetic() => {
            (format!("{}:/", d.to_ascii_uppercase()), &p[2..])
        }
        _ if p.starts_with('/') => ("/".to_string(), p.as_str()),
        _ => (String::new(), p.as_str()),
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
        let absolute = !s.0.is_empty() && root.starts_with('/');
        out.insert(s);
        if absolute {
            out.insert(split(&realpath(root)));
        }
    }
    let mut v: Vec<Split> = out.into_iter().collect();
    v.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.cmp(b)));
    v
}

fn strip<'a>(s: &'a Split, roots: &[Split]) -> Option<&'a [String]> {
    let (anchor, parts) = s;
    roots
        .iter()
        .find(|(ra, rp)| ra == anchor && parts.len() > rp.len() && parts.starts_with(rp))
        .map(|(_, rp)| &parts[rp.len()..])
}

fn normalize(raw: &str, roots: &[Split]) -> (String, bool) {
    let s = split(raw);
    let cleaned = join(&s);
    let rest = strip(&s, roots).map(|r| r.join("/"));
    let rest = match rest {
        None if !s.0.is_empty() && cleaned.starts_with('/') => {
            let real = split(&realpath(&cleaned));
            strip(&real, roots).map(|r| r.join("/"))
        }
        r => r,
    };
    if let Some(rest) = rest {
        return (rest, false);
    }
    let (anchor, parts) = &s;
    if anchor.is_empty() && parts.first().is_some_and(|p| p != "..") {
        return (cleaned, false);
    }
    (cleaned, true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_join() {
        let j = |p: &str| join(&split(p));
        assert_eq!(j("a/./b//c/"), "a/b/c");
        assert_eq!(j("a\\..\\..\\b"), "../b");
        assert_eq!(j("/../x"), "/x");
        assert_eq!(j("c:\\x\\y"), "C:/x/y");
        assert_eq!(j("c:"), "C:/");
        assert_eq!(j("cd:/x"), "cd:/x");
        assert_eq!(j(""), ".");
    }

    #[test]
    fn roots() {
        let roots = ["/repo", "/repo/sub"];
        assert_eq!(
            normalize_path("/repo/sub/a.py", &roots),
            ("a.py".into(), false)
        );
        assert_eq!(normalize_path("/repo/b.py", &roots), ("b.py".into(), false));
        assert_eq!(
            normalize_path("/repox/b.py", &roots),
            ("/repox/b.py".into(), true)
        );
        assert_eq!(normalize_path("x/../../y", &roots), ("../y".into(), true));
        assert_eq!(normalize_path("./x/y", &roots), ("x/y".into(), false));
        assert_eq!(
            normalize_path("C:\\r\\a.rs", &["c:/r"]),
            ("a.rs".into(), false)
        );
    }

    #[test]
    fn realpath_follows_links() {
        let dir = std::env::temp_dir().join(format!("sundgren-rp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real/sub")).unwrap();
        std::os::unix::fs::symlink(dir.join("real"), dir.join("link")).unwrap();
        std::os::unix::fs::symlink("loop", dir.join("loop")).unwrap();
        let base = realpath(dir.to_str().unwrap());
        let rp = |p: &str| realpath(&format!("{base}/{p}"));
        assert_eq!(
            rp("link/sub/missing/x"),
            format!("{base}/real/sub/missing/x")
        );
        assert_eq!(rp("link/../link/sub"), format!("{base}/real/sub"));
        assert_eq!(rp("loop/x"), format!("{base}/loop/x"));
        assert_eq!(realpath("/"), "/");
        assert_eq!(realpath("//a/../.."), "/");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
