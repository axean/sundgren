//! Bindings for `sundgren._native.run_selectors`.

use pyo3::prelude::*;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "run_selectors")?;
    parent.add_submodule(&m)?;
    Ok(())
}
