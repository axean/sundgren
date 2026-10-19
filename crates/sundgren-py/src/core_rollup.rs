//! Bindings for `sundgren._native.core_rollup`.

use pyo3::prelude::*;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_rollup")?;
    parent.add_submodule(&m)?;
    Ok(())
}
