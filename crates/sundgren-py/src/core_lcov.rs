//! Bindings for `sundgren._native.core_lcov`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sundgren_core::lcov;

use crate::core_model::PySnapshot;

/// A debug-log sink for the Rust side: `logging.getLogger(name).debug`, or None when the
/// logger is not enabled for DEBUG.
pub fn debug_logger<'py>(py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    let logger = py.import("logging")?.call_method1("getLogger", (name,))?;
    if logger.call_method1("isEnabledFor", (10,))?.is_truthy()? {
        Ok(Some(logger))
    } else {
        Ok(None)
    }
}

/// Call `f` with a log sink that forwards to `logger.debug` (errors are dropped).
pub fn with_debug_log<T>(
    logger: Option<Bound<'_, PyAny>>,
    f: impl FnOnce(&mut dyn FnMut(String)) -> T,
) -> T {
    match logger {
        None => f(&mut |_| {}),
        Some(logger) => f(&mut |msg| {
            let _ = logger.call_method1("debug", (msg,));
        }),
    }
}

#[pyfunction]
fn parse(py: Python<'_>, text: &str) -> PyResult<PySnapshot> {
    let logger = debug_logger(py, "sundgren.core.ingest.lcov")?;
    match with_debug_log(logger, |log| lcov::parse_logged(text, log)) {
        Ok(snap) => Ok(snap.into()),
        Err(e) => {
            // Python's repr, so the message matches the prototype exactly.
            let line = e.line.into_pyobject(py)?.repr()?;
            Err(PyValueError::new_err(format!(
                "lcov line {}: malformed {} record: {line}",
                e.lineno, e.tag
            )))
        }
    }
}

#[pyfunction]
fn dump(snapshot: &PySnapshot) -> String {
    lcov::dump(&snapshot.inner)
}

#[pyclass(module = "sundgren.core.ingest.lcov", name = "LcovIngestor", frozen)]
struct PyLcovIngestor;

#[pymethods]
impl PyLcovIngestor {
    #[new]
    fn new() -> Self {
        PyLcovIngestor
    }

    fn ingest(&self, py: Python<'_>, text: &str) -> PyResult<PySnapshot> {
        parse(py, text)
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_lcov")?;
    m.add_function(wrap_pyfunction!(parse, &m)?)?;
    m.add_function(wrap_pyfunction!(dump, &m)?)?;
    m.add_class::<PyLcovIngestor>()?;
    parent.add_submodule(&m)?;
    Ok(())
}
