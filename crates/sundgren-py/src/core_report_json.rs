//! Bindings for `sundgren._native.core_report_json`.

use pyo3::prelude::*;

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_report_json")?;
    parent.add_submodule(&m)?;
    Ok(())
}
