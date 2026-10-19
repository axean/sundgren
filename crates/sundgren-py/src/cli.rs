//! Bindings for `sundgren._native.cli`.

use pyo3::prelude::*;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "cli")?;
    parent.add_submodule(&m)?;
    Ok(())
}
