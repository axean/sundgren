//! Bindings for `sundgren._native.core_junit`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sundgren_core::junit;

use crate::core_lcov::{debug_logger, with_debug_log};

#[pyfunction]
fn parse_junit<'py>(py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyDict>> {
    let logger = debug_logger(py, "sundgren.core.junit")?;
    let outcomes = with_debug_log(logger, |log| junit::parse_junit_logged(text, log))
        .map_err(|e| PyValueError::new_err(e.0))?;
    let d = PyDict::new(py);
    for (id, outcome) in &outcomes {
        d.set_item(id, outcome.as_str())?;
    }
    Ok(d)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_junit")?;
    m.add_function(wrap_pyfunction!(parse_junit, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
