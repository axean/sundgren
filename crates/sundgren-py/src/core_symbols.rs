//! Bindings for `sundgren._native.core_symbols`.
//!
//! The registry (`sundgren.core.symbols._REGISTRY`) stays a Python dict; `registry_lookup`
//! reads it at call time through the Python `get_symbolizer`.

use std::path::PathBuf;

use pyo3::exceptions::PySyntaxError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyRange};
use sundgren_core::model::LineNo;
use sundgren_core::pytext::PyText;
use sundgren_core::symbols::{self, LcovFnSymbolizer, Span, Symbolizer, SymbolizerError};

use crate::core_model::PyFileCoverage;
use crate::dataclass;
use crate::pytext::{pytext_from_py, pytext_into_py};

#[pyclass(frozen, eq, hash, module = "sundgren.core.symbols", name = "Span")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PySpan {
    pub inner: Span,
}

impl From<Span> for PySpan {
    fn from(inner: Span) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySpan {
    #[new]
    #[pyo3(signature = (name, start, end, body_start = None))]
    fn new(name: String, start: LineNo, end: LineNo, body_start: Option<LineNo>) -> Self {
        Span {
            name,
            start,
            end,
            body_start,
        }
        .into()
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn start(&self) -> LineNo {
        self.inner.start
    }

    #[getter]
    fn end(&self) -> LineNo {
        self.inner.end
    }

    #[getter]
    fn body_start(&self) -> Option<LineNo> {
        self.inner.body_start
    }

    #[getter]
    fn body_lines<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyRange>> {
        let lines = self.inner.body_lines();
        PyRange::new(py, *lines.start() as isize, *lines.end() as isize + 1)
    }
}

dataclass!(PySpan, "Span", ["name", "start", "end", "body_start"]);

/// A span from Python: the `Span` class, or anything with its attributes.
fn span_from_py(obj: &Bound<'_, PyAny>) -> PyResult<Span> {
    if let Ok(span) = obj.cast::<PySpan>() {
        return Ok(span.get().inner.clone());
    }
    Ok(Span {
        name: obj.getattr("name")?.extract()?,
        start: obj.getattr("start")?.extract()?,
        end: obj.getattr("end")?.extract()?,
        body_start: obj.getattr("body_start")?.extract()?,
    })
}

fn spans_into_py<'py>(py: Python<'py>, spans: Vec<Span>) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, spans.into_iter().map(PySpan::from))
}

#[pyclass(frozen, module = "sundgren.core.symbols", name = "LcovFnSymbolizer")]
struct PyLcovFnSymbolizer;

#[pymethods]
impl PyLcovFnSymbolizer {
    #[new]
    fn new() -> Self {
        Self
    }

    fn spans<'py>(&self, py: Python<'py>, fc: &PyFileCoverage) -> PyResult<Bound<'py, PyList>> {
        spans_into_py(py, LcovFnSymbolizer.spans(&fc.inner))
    }
}

/// A Python symbolizer object (anything with `functions(source) -> list[Span]`).
struct PySymbolizer(Py<PyAny>);

impl Symbolizer for PySymbolizer {
    fn functions(&self, source: &str) -> Result<Vec<Span>, SymbolizerError> {
        Python::attach(|py| {
            let result = self
                .0
                .bind(py)
                .call_method1("functions", (source,))
                .map_err(from_pyerr)?;
            result
                .try_iter()
                .and_then(|it| it.map(|s| span_from_py(&s?)).collect())
                .map_err(from_pyerr)
        })
    }
}

/// A lookup that failed in Python; reports the error when used.
struct FailedLookup(std::sync::Mutex<Option<PyErr>>);

impl Symbolizer for FailedLookup {
    fn functions(&self, _source: &str) -> Result<Vec<Span>, SymbolizerError> {
        let err = self.0.lock().expect("not poisoned").take();
        Err(SymbolizerError::Other(Box::new(err.unwrap_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("symbolizer lookup failed")
        }))))
    }
}

fn from_pyerr(err: PyErr) -> SymbolizerError {
    let syntax = Python::attach(|py| err.is_instance_of::<PySyntaxError>(py));
    if syntax {
        SymbolizerError::Syntax
    } else {
        SymbolizerError::Other(Box::new(err))
    }
}

/// `SymbolizerError` back to a Python exception; `Other` keeps the original one.
pub(crate) fn into_pyerr(err: SymbolizerError) -> PyErr {
    match err {
        SymbolizerError::Syntax => PySyntaxError::new_err("symbolizer failed"),
        SymbolizerError::Other(e) => match e.downcast::<PyErr>() {
            Ok(e) => *e,
            Err(e) => pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
        },
    }
}

/// The registered symbolizer for a path, read from the live Python registry at call time.
pub(crate) fn registry_lookup(
    py: Python<'_>,
) -> impl Fn(&PyText) -> Option<Box<dyn Symbolizer>> + '_ {
    move |path: &PyText| {
        let found = (|| {
            let get = py
                .import("sundgren.core.symbols")?
                .getattr("get_symbolizer")?;
            get.call1((pytext_into_py(py, path)?,))
        })();
        match found {
            Ok(obj) if obj.is_none() => None,
            Ok(obj) => Some(Box::new(PySymbolizer(obj.unbind())) as Box<dyn Symbolizer>),
            Err(e) => Some(Box::new(FailedLookup(std::sync::Mutex::new(Some(e))))),
        }
    }
}

/// Spans for one side of a file: registered symbolizer, then lcov FN, else None.
#[pyfunction]
fn resolve_spans<'py>(
    py: Python<'py>,
    path: &Bound<'py, PyAny>,
    root: Option<PathBuf>,
    fc: Option<PyRef<'py, PyFileCoverage>>,
) -> PyResult<Option<Bound<'py, PyList>>> {
    let path = pytext_from_py(path)?;
    let symbolizer = registry_lookup(py)(&path);
    let spans = symbols::resolve_spans(
        &path,
        root.as_deref(),
        fc.as_ref().map(|f| &f.inner),
        symbolizer.as_deref(),
    )
    .map_err(into_pyerr)?;
    spans.map(|s| spans_into_py(py, s)).transpose()
}

/// First span per qualified name (duplicates, e.g. redefinitions, keep the earliest).
#[pyfunction]
fn spans_by_name<'py>(py: Python<'py>, spans: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    for span in spans.try_iter()? {
        let span = span?;
        let name = span.getattr("name")?;
        if !out.contains(&name)? {
            out.set_item(name, span)?;
        }
    }
    Ok(out)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_symbols")?;
    m.add_class::<PySpan>()?;
    m.add_class::<PyLcovFnSymbolizer>()?;
    m.add_function(wrap_pyfunction!(resolve_spans, &m)?)?;
    m.add_function(wrap_pyfunction!(spans_by_name, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
