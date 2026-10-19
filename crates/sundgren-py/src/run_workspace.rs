//! Bindings for `sundgren._native.run_workspace`.

use pyo3::prelude::*;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "run_workspace")?;
    parent.add_submodule(&m)?;
    Ok(())
}
