//! Wrong-tree guard: coverage measured the main checkout instead of the worktree.

use std::collections::BTreeSet;
use std::path::Path;

use sundgren_core::normalize::realpath;

/// Raw SF paths outside `worktree` but inside `main_repo`, sorted.
pub fn wrong_tree_paths<S: AsRef<str>>(
    raw_paths: impl IntoIterator<Item = S>,
    worktree: &Path,
    main_repo: &Path,
) -> Vec<String> {
    let tree = realpath(&worktree.to_string_lossy());
    let repo = realpath(&main_repo.to_string_lossy());
    let mut bad = BTreeSet::new();
    for raw in raw_paths {
        let raw = raw.as_ref();
        // Relative SF paths are relative to the worktree.
        if !raw.starts_with('/') {
            continue;
        }
        let real = realpath(raw);
        // The worktree may itself sit inside main_repo (--workdir in the repo).
        if !within(&real, &tree) && within(&real, &repo) {
            bad.insert(raw.to_owned());
        }
    }
    bad.into_iter().collect()
}

/// `os.path.commonpath([path, root]) == root` for absolute paths.
fn within(path: &str, root: &str) -> bool {
    fn parts(p: &str) -> Vec<&str> {
        p.split('/')
            .filter(|c| !c.is_empty() && *c != ".")
            .collect()
    }
    let (a, b) = (parts(path), parts(root));
    let n = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    format!("/{}", a[..n].join("/")) == root
}

#[cfg(test)]
mod tests {
    use super::within;

    #[test]
    fn commonpath_rules() {
        assert!(within("/a/b/c", "/a/b"));
        assert!(within("/a/b", "/a/b"));
        assert!(!within("/a/bc", "/a/b"));
        assert!(within("/x", "/"));
        assert!(!within("/a", "/a/b"));
    }
}
