//! Bindings for `sundgren._native.core_normalize`.

use pyo3::prelude::*;
use sundgren_core::normalize;

use crate::core_model::PySnapshot;

#[pyfunction]
fn realpath(path: &str) -> String {
    normalize::realpath(path)
}

#[pyfunction]
fn normalize_path(raw: &str, roots: Vec<String>) -> (String, bool) {
    normalize::normalize_path(raw, &roots)
}

#[pyfunction]
fn normalize_snapshot(snapshot: &PySnapshot, roots: Vec<String>) -> PySnapshot {
    normalize::normalize_snapshot(&snapshot.inner, &roots).into()
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_normalize")?;
    m.add_function(wrap_pyfunction!(realpath, &m)?)?;
    m.add_function(wrap_pyfunction!(normalize_path, &m)?)?;
    m.add_function(wrap_pyfunction!(normalize_snapshot, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
