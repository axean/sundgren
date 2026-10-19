//! Bindings for `sundgren._native.core_junit`.

use pyo3::prelude::*;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_junit")?;
    parent.add_submodule(&m)?;
    Ok(())
}
