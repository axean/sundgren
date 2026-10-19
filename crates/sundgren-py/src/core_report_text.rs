//! Bindings for `sundgren._native.core_report_text`.

use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyString;
use sundgren_core::report_text;

use crate::core_report_json::py_to_json;
use crate::pytext::{pytext_from_py, pytext_into_py};

/// Test changes, summary counts, top files with their functions, warnings, hints.
#[pyfunction]
#[pyo3(signature = (report, hints = None))]
fn render_text<'py>(
    py: Python<'py>,
    report: &Bound<'py, PyAny>,
    hints: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyString>> {
    let Some(value) = py_to_json(report)? else {
        return Err(PyTypeError::new_err("report must be plain JSON data"));
    };
    let hints = match hints {
        Some(h) => h
            .try_iter()?
            .map(|x| pytext_from_py(&x?))
            .collect::<PyResult<Vec<_>>>()?,
        None => Vec::new(),
    };
    let text = report_text::render_text(&value, &hints).map_err(|e| PyKeyError::new_err(e.0))?;
    pytext_into_py(py, &text)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_report_text")?;
    m.add("TOP_FILES", report_text::TOP_FILES)?;
    m.add_function(wrap_pyfunction!(render_text, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
