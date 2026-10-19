//! Bindings for `sundgren._native.run_guards`.

use std::path::PathBuf;

use pyo3::prelude::*;
use sundgren_run::guards;

/// Raw SF paths outside `worktree` but inside `main_repo`, sorted.
#[pyfunction]
fn wrong_tree_paths(
    raw_paths: &Bound<'_, PyAny>,
    worktree: PathBuf,
    main_repo: PathBuf,
) -> PyResult<Vec<String>> {
    let raw = raw_paths
        .try_iter()?
        .map(|p| p?.extract::<String>())
        .collect::<PyResult<Vec<_>>>()?;
    Ok(guards::wrong_tree_paths(raw, &worktree, &main_repo))
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "run_guards")?;
    m.add_function(wrap_pyfunction!(wrong_tree_paths, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
