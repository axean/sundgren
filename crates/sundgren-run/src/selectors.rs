//! Test selectors: pick which tests a patch should run. The registry keyed by id is the caller's.

use std::error::Error;
use std::fmt;
use std::path::Path;

use sundgren_core::patch::Patch;

use crate::pyfmt::repr_str;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Selection {
    pub tests: Vec<String>,
    pub run_all: bool,
    pub reason: String,
}

/// An error raised inside a selector the caller supplied.
pub type HostError = Box<dyn Error + Send + Sync>;

pub trait Selector {
    fn select(&self, patch: &Patch, repo: &Path) -> Result<Selection, HostError>;
}

#[derive(Debug)]
pub enum ChooseError {
    /// No selector registered under this id (Python: KeyError).
    Unknown(String),
    Host(HostError),
}

impl fmt::Display for ChooseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChooseError::Unknown(id) => write!(f, "unknown selector {}", repr_str(id)),
            ChooseError::Host(e) => e.fmt(f),
        }
    }
}

impl Error for ChooseError {}

pub fn run_all() -> Selection {
    Selection {
        tests: Vec::new(),
        run_all: true,
        reason: "no selector; running all tests".into(),
    }
}

/// --tests beats the preset's selector, which beats run-all. `lookup` finds a selector by id.
pub fn choose<'a>(
    tests: Option<&[String]>,
    selector_id: Option<&str>,
    patch: &Patch,
    repo: &Path,
    lookup: &dyn Fn(&str) -> Option<Box<dyn Selector + 'a>>,
) -> Result<Selection, ChooseError> {
    if let Some(tests) = tests.filter(|t| !t.is_empty()) {
        return Ok(Selection {
            tests: tests.to_vec(),
            run_all: false,
            reason: "--tests given".into(),
        });
    }
    match selector_id {
        Some(id) => lookup(id)
            .ok_or_else(|| ChooseError::Unknown(id.to_owned()))?
            .select(patch, repo)
            .map_err(ChooseError::Host),
        None => Ok(run_all()),
    }
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
mod tests {
    use super::*;

    fn sel(tests: &[&str]) -> Selection {
        Selection {
            tests: tests.iter().map(|t| t.to_string()).collect(),
            run_all: false,
            reason: "r".into(),
        }
    }

    #[test]
    fn restrict_drops_missing() {
        let dir = std::env::temp_dir().join(format!("sundgren-sel-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("t")).unwrap();
        std::fs::write(dir.join("t/a.py"), "").unwrap();
        let s = sel(&["t/a.py::x", "t/b.py"]);
        let got = restrict_to_tree(&s, &dir).unwrap();
        assert_eq!(got.tests, ["t/a.py::x"]);
        assert_eq!(got.reason, "r; 1 missing on this side");
        assert_eq!(restrict_to_tree(&sel(&["t/b.py"]), &dir), None);
        assert_eq!(
            restrict_to_tree(&sel(&["t/a.py"]), &dir),
            Some(sel(&["t/a.py"]))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn choose_precedence() {
        let patch = Patch::default();
        let none = |_: &str| -> Option<Box<dyn Selector>> { None };
        let tests = vec!["x".to_string()];
        let got = choose(Some(&tests), Some("nope"), &patch, Path::new("."), &none).unwrap();
        assert_eq!(got.reason, "--tests given");
        assert_eq!(
            choose(Some(&[]), None, &patch, Path::new("."), &none).unwrap(),
            run_all()
        );
        let err = choose(None, Some("nope"), &patch, Path::new("."), &none).unwrap_err();
        assert_eq!(err.to_string(), "unknown selector 'nope'");
    }
}
