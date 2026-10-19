//! Bindings for `sundgren._native.run_selectors`.
//!
//! `_REGISTRY` stays a dict in `sundgren.run.selectors`; every call reads it from there and wraps
//! the entry as a Rust `Selector`.

use std::path::{Path, PathBuf};

use pyo3::exceptions::{PyKeyError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use sundgren_core::patch::Patch;
use sundgren_run::selectors::{self, ChooseError, HostError, Selection, Selector};

use crate::core_patch::{PyPatch, patch_into_py};
use crate::dataclass;

#[pyclass(
    module = "sundgren.run.selectors",
    name = "Selection",
    frozen,
    eq,
    hash
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PySelection {
    pub inner: Selection,
}

#[pymethods]
impl PySelection {
    #[new]
    #[pyo3(signature = (tests, run_all, reason))]
    fn new(tests: Vec<String>, run_all: bool, reason: String) -> Self {
        Selection {
            tests,
            run_all,
            reason,
        }
        .into()
    }

    #[getter]
    fn tests<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, &self.inner.tests)
    }

    #[getter]
    fn run_all(&self) -> bool {
        self.inner.run_all
    }

    #[getter]
    fn reason(&self) -> &str {
        &self.inner.reason
    }
}
dataclass!(PySelection, "Selection", ["tests", "run_all", "reason"]);

impl From<Selection> for PySelection {
    fn from(inner: Selection) -> Self {
        Self { inner }
    }
}

/// A Python selector object (anything with `.select(patch, repo)`) as a Rust `Selector`.
pub struct PySelector(pub Py<PyAny>);

impl Selector for PySelector {
    fn select(&self, patch: &Patch, repo: &Path) -> Result<Selection, HostError> {
        Python::attach(|py| {
            let patch = patch_into_py(py, patch.clone())?;
            let got = self.0.bind(py).call_method1("select", (patch, repo))?;
            Ok::<_, PyErr>(got.cast::<PySelection>()?.get().inner.clone())
        })
        .map_err(|e| Box::new(e) as HostError)
    }
}

/// Python error for a `ChooseError`: KeyError for an unknown id, else what the selector raised.
pub fn choose_err(err: ChooseError) -> PyErr {
    match err {
        ChooseError::Unknown(_) => PyKeyError::new_err(err.to_string()),
        ChooseError::Host(e) => match e.downcast::<PyErr>() {
            Ok(e) => *e,
            Err(e) => PyRuntimeError::new_err(e.to_string()),
        },
    }
}

/// The live `_REGISTRY` dict of `sundgren.run.selectors`.
fn registry(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    Ok(py
        .import("sundgren.run.selectors")?
        .getattr("_REGISTRY")?
        .cast_into::<PyDict>()?)
}

/// Look up a selector in the live registry.
pub fn lookup(py: Python<'_>, id: &str) -> PyResult<Option<PySelector>> {
    Ok(registry(py)?
        .get_item(id)?
        .map(|obj| PySelector(obj.unbind())))
}

#[pyfunction]
fn register_selector(py: Python<'_>, selector_id: &str, selector: Py<PyAny>) -> PyResult<()> {
    registry(py)?.set_item(selector_id, selector)
}

#[pyfunction]
fn get_selector<'py>(py: Python<'py>, selector_id: &str) -> PyResult<Bound<'py, PyAny>> {
    registry(py)?
        .get_item(selector_id)?
        .ok_or_else(|| choose_err(ChooseError::Unknown(selector_id.to_owned())))
}

/// --tests beats the preset's selector, which beats run-all.
#[pyfunction]
fn choose(
    py: Python<'_>,
    tests: Option<Vec<String>>,
    selector_id: Option<String>,
    patch: PyRef<'_, PyPatch>,
    repo: PathBuf,
) -> PyResult<PySelection> {
    // A dict lookup by str key can't fail.
    let get = |id: &str| {
        lookup(py, id)
            .ok()
            .flatten()
            .map(|s| Box::new(s) as Box<dyn Selector>)
    };
    selectors::choose(
        tests.as_deref(),
        selector_id.as_deref(),
        &patch.inner,
        &repo,
        &get,
    )
    .map(PySelection::from)
    .map_err(choose_err)
}

/// Drop path-style tests (`path[::id]`) missing from `tree`; None if none are left.
#[pyfunction]
fn restrict_to_tree(selection: PyRef<'_, PySelection>, tree: PathBuf) -> Option<PySelection> {
    selectors::restrict_to_tree(&selection.inner, &tree).map(PySelection::from)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "run_selectors")?;
    m.add_class::<PySelection>()?;
    m.add("RUN_ALL", PySelection::from(selectors::run_all()))?;
    m.add_function(wrap_pyfunction!(register_selector, &m)?)?;
    m.add_function(wrap_pyfunction!(get_selector, &m)?)?;
    m.add_function(wrap_pyfunction!(choose, &m)?)?;
    m.add_function(wrap_pyfunction!(restrict_to_tree, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
