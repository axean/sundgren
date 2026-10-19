//! Bindings for `sundgren._native.core_hashing`.

use std::path::PathBuf;

use pyo3::prelude::*;
use sundgren_core::hashing;

#[pyfunction]
fn sha256_bytes(data: &[u8]) -> String {
    hashing::sha256_bytes(data)
}

#[pyfunction]
fn sha256_file(path: PathBuf) -> PyResult<String> {
    Ok(hashing::sha256_file(&path)?)
}

#[pyfunction]
#[pyo3(signature = (path))]
fn sha256_file_or_none(path: Option<PathBuf>) -> PyResult<Option<String>> {
    Ok(hashing::sha256_file_or_none(path.as_deref())?)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_hashing")?;
    m.add_function(wrap_pyfunction!(sha256_bytes, &m)?)?;
    m.add_function(wrap_pyfunction!(sha256_file, &m)?)?;
    m.add_function(wrap_pyfunction!(sha256_file_or_none, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
