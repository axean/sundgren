//! Test selection: which tests a patch should run.

use std::path::Path;

use crate::patch::{FilePatch, Patch};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub tests: Vec<String>,
    pub run_all: bool,
    pub reason: String,
}

fn run_all(reason: String) -> Selection {
    Selection {
        tests: Vec::new(),
        run_all: true,
        reason,
    }
}

/// --tests beats the preset's selector, which beats run-all.
pub fn choose(
    tests: &[String],
    selector: Option<&str>,
    patch: &Patch,
    repo: &Path,
) -> Result<Selection> {
    if !tests.is_empty() {
        return Ok(Selection {
            tests: tests.to_vec(),
            run_all: false,
            reason: "--tests given".into(),
        });
    }
    match selector {
        None => Ok(run_all("no selector; running all tests".into())),
        Some("python.pytest_files") => Ok(pytest_files(patch, repo)),
        Some(other) => Err(Error::failure(format!("unknown selector '{other}'"))),
    }
}

const PYTEST_CONFIG: [&str; 4] = ["pytest.ini", "pyproject.toml", "setup.cfg", "tox.ini"];

/// Run the touched test files, or everything if config or source changed.
fn pytest_files(patch: &Patch, repo: &Path) -> Selection {
    let mut files: Vec<&FilePatch> = patch.files.iter().collect();
    files.sort_by(|a, b| a.path().cmp(b.path()));
    let mut selected = Vec::new();
    for fp in files {
        let names: Vec<&str> = [&fp.old_path, &fp.new_path]
            .into_iter()
            .flatten()
            .map(|p| p.rsplit('/').next().unwrap_or(p))
            .collect();
        if !names
            .iter()
            .any(|n| n.ends_with(".py") || PYTEST_CONFIG.contains(n))
        {
            continue;
        }
        if !names.iter().all(|n| is_test_file(n)) {
            return run_all(format!("{} changed; running all tests", fp.path()));
        }
        // The same selection runs on both sides, so the file must exist in both.
        if fp.old_path != fp.new_path {
            let kind = match (&fp.old_path, &fp.new_path) {
                (None, _) => "added",
                (_, None) => "deleted",
                _ => "renamed",
            };
            return run_all(format!("test file {} {kind}; running all tests", fp.path()));
        }
        if repo.join(fp.path()).is_file() {
            selected.push(fp.path().to_owned());
        }
    }
    if selected.is_empty() {
        return run_all("no touched test files; running all tests".into());
    }
    selected.sort();
    Selection {
        tests: selected,
        run_all: false,
        reason: "touched test files".into(),
    }
}

fn is_test_file(name: &str) -> bool {
    (name.starts_with("test_") && name.ends_with(".py")) || name.ends_with("_test.py")
}

/// Drop path-style tests (`path[::id]`) missing from `tree`; None if none are left.
pub fn restrict_to_tree(selection: &Selection, tree: &Path) -> Option<Selection> {
    if selection.run_all || selection.tests.is_empty() {
        return Some(selection.clone());
    }
    let kept: Vec<String> = selection
        .tests
        .iter()
        .filter(|t| tree.join(t.split("::").next().unwrap_or_default()).exists())
        .cloned()
        .collect();
    if kept.len() == selection.tests.len() {
        return Some(selection.clone());
    }
    if kept.is_empty() {
        return None;
    }
    let dropped = selection.tests.len() - kept.len();
    Some(Selection {
        tests: kept,
        run_all: false,
        reason: format!("{}; {dropped} missing on this side", selection.reason),
    })
}

#[cfg(test)]
mod tests;
