//! Bindings for `sundgren._native.frontend_python`.

use std::path::PathBuf;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use sundgren_frontend_python::{Request, parse_files};

use crate::core_symbols::PySpan;

/// Function spans from the Python IR frontend, one entry per `(file, path)`: a list of spans
/// sorted by (start, name), like `PythonSymbolizer`, or None if the file has no IR.
#[pyfunction]
fn function_spans<'py>(
    py: Python<'py>,
    python: PathBuf,
    files: Vec<(PathBuf, String)>,
) -> PyResult<Vec<Option<Bound<'py, PyList>>>> {
    let requests: Vec<Request> = files
        .into_iter()
        .map(|(file, path)| Request { file, path })
        .collect();
    let results = py
        .detach(|| parse_files(python.as_os_str(), &requests))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    results
        .into_iter()
        .map(|r| {
            let Ok(file) = r else { return Ok(None) };
            let mut spans = sundgren_join::callable_spans(&file, false);
            spans.sort_by(|a, b| (a.start, &a.name).cmp(&(b.start, &b.name)));
            PyList::new(py, spans.into_iter().map(PySpan::from)).map(Some)
        })
        .collect()
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "frontend_python")?;
    m.add_function(wrap_pyfunction!(function_spans, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
