//! Bindings for `sundgren._native.core_model`.
//!
//! Each class wraps its Rust value in `inner`. Map and tuple fields are converted on first
//! access and cached, so repeated access is cheap and returns the same object.

use std::collections::BTreeMap;

use pyo3::IntoPyObjectExt;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyDict, PyList, PyTuple};
use sundgren_core::model::{self, Count, LineNo};

use crate::dataclass;

/// Iterate `(key, value)` pairs of any mapping (calls `.items()`).
pub fn mapping_items<'py>(
    obj: &Bound<'py, PyAny>,
) -> PyResult<Vec<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
    obj.call_method0("items")?
        .try_iter()?
        .map(|item| item?.extract())
        .collect()
}

/// `json`-style Python object from a serde_json value (ints stay exact).
pub fn json_to_py<'py>(py: Python<'py>, v: &serde_json::Value) -> PyResult<Bound<'py, PyAny>> {
    use serde_json::Value;
    Ok(match v {
        Value::Null => py.None().into_bound(py),
        Value::Bool(b) => b.into_bound_py_any(py)?,
        Value::Number(n) => {
            let s = n.to_string();
            match s.parse::<i128>() {
                Ok(i) => i.into_bound_py_any(py)?,
                Err(_) => py.import("builtins")?.getattr("float")?.call1((s,))?,
            }
        }
        Value::String(s) => s.into_bound_py_any(py)?,
        Value::Array(a) => {
            let items = a
                .iter()
                .map(|x| json_to_py(py, x))
                .collect::<PyResult<Vec<_>>>()?;
            PyList::new(py, items)?.into_any()
        }
        Value::Object(o) => {
            let d = PyDict::new(py);
            for (k, x) in o {
                d.set_item(k, json_to_py(py, x)?)?;
            }
            d.into_any()
        }
    })
}

#[pyclass(
    module = "sundgren.core.model",
    name = "LineStatus",
    frozen,
    eq,
    eq_int,
    hash
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyLineStatus {
    #[pyo3(name = "ABSENT")]
    Absent,
    #[pyo3(name = "MISS")]
    Miss,
    #[pyo3(name = "HIT")]
    Hit,
}

impl From<model::LineStatus> for PyLineStatus {
    fn from(s: model::LineStatus) -> Self {
        match s {
            model::LineStatus::Absent => PyLineStatus::Absent,
            model::LineStatus::Miss => PyLineStatus::Miss,
            model::LineStatus::Hit => PyLineStatus::Hit,
        }
    }
}

impl PyLineStatus {
    fn rust(self) -> model::LineStatus {
        match self {
            PyLineStatus::Absent => model::LineStatus::Absent,
            PyLineStatus::Miss => model::LineStatus::Miss,
            PyLineStatus::Hit => model::LineStatus::Hit,
        }
    }
}

#[pymethods]
impl PyLineStatus {
    #[getter]
    fn value(&self) -> &'static str {
        self.rust().as_str()
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self {
            PyLineStatus::Absent => "ABSENT",
            PyLineStatus::Miss => "MISS",
            PyLineStatus::Hit => "HIT",
        }
    }

    fn __repr__(&self) -> String {
        format!("<LineStatus.{}: '{}'>", self.name(), self.value())
    }
}

#[pyclass(
    module = "sundgren.core.model",
    name = "BranchKey",
    frozen,
    eq,
    ord,
    hash
)]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PyBranchKey {
    pub inner: model::BranchKey,
}

#[pymethods]
impl PyBranchKey {
    #[new]
    #[pyo3(signature = (line, block, branch))]
    fn new(line: LineNo, block: String, branch: String) -> Self {
        model::BranchKey {
            line,
            block,
            branch,
        }
        .into()
    }

    #[getter]
    fn line(&self) -> LineNo {
        self.inner.line
    }

    #[getter]
    fn block(&self) -> &str {
        &self.inner.block
    }

    #[getter]
    fn branch(&self) -> &str {
        &self.inner.branch
    }
}
dataclass!(PyBranchKey, "BranchKey", ["line", "block", "branch"]);

impl From<model::BranchKey> for PyBranchKey {
    fn from(inner: model::BranchKey) -> Self {
        Self { inner }
    }
}

#[pyclass(
    module = "sundgren.core.model",
    name = "FunctionRecord",
    frozen,
    eq,
    hash
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyFunctionRecord {
    pub inner: model::FunctionRecord,
}

#[pymethods]
impl PyFunctionRecord {
    #[new]
    #[pyo3(signature = (name, start, end, hits))]
    fn new(name: String, start: LineNo, end: Option<LineNo>, hits: Option<Count>) -> Self {
        model::FunctionRecord {
            name,
            start,
            end,
            hits,
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
    fn end(&self) -> Option<LineNo> {
        self.inner.end
    }

    #[getter]
    fn hits(&self) -> Option<Count> {
        self.inner.hits
    }
}
dataclass!(
    PyFunctionRecord,
    "FunctionRecord",
    ["name", "start", "end", "hits"]
);

impl From<model::FunctionRecord> for PyFunctionRecord {
    fn from(inner: model::FunctionRecord) -> Self {
        Self { inner }
    }
}

/// Not hashable, like the dataclass (it holds dicts).
#[pyclass(module = "sundgren.core.model", name = "FileCoverage", frozen, eq)]
pub struct PyFileCoverage {
    pub inner: model::FileCoverage,
    lines: PyOnceLock<Py<PyDict>>,
    branches: PyOnceLock<Py<PyDict>>,
    functions: PyOnceLock<Py<PyTuple>>,
}

impl PartialEq for PyFileCoverage {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl From<model::FileCoverage> for PyFileCoverage {
    fn from(inner: model::FileCoverage) -> Self {
        Self {
            inner,
            lines: PyOnceLock::new(),
            branches: PyOnceLock::new(),
            functions: PyOnceLock::new(),
        }
    }
}

#[pymethods]
impl PyFileCoverage {
    #[new]
    #[pyo3(signature = (path, lines, branches, functions, external))]
    fn new(
        path: String,
        lines: &Bound<'_, PyAny>,
        branches: &Bound<'_, PyAny>,
        functions: &Bound<'_, PyAny>,
        external: bool,
    ) -> PyResult<Self> {
        let lines = mapping_items(lines)?
            .into_iter()
            .map(|(k, v)| Ok((k.extract()?, v.extract()?)))
            .collect::<PyResult<BTreeMap<LineNo, Count>>>()?;
        let branches = mapping_items(branches)?
            .into_iter()
            .map(|(k, v)| {
                let key = k.cast::<PyBranchKey>()?.get().inner.clone();
                Ok((key, v.extract()?))
            })
            .collect::<PyResult<_>>()?;
        let functions = functions
            .try_iter()?
            .map(|f| Ok(f?.cast::<PyFunctionRecord>()?.get().inner.clone()))
            .collect::<PyResult<_>>()?;
        Ok(model::FileCoverage {
            path,
            lines,
            branches,
            functions,
            external,
        }
        .into())
    }

    #[getter]
    fn path(&self) -> &str {
        &self.inner.path
    }

    #[getter]
    fn lines(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = self.lines.get_or_try_init(py, || {
            Ok::<_, PyErr>((&self.inner.lines).into_pyobject(py)?.unbind())
        })?;
        Ok(d.clone_ref(py))
    }

    #[getter]
    fn branches(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = self.branches.get_or_try_init(py, || {
            let d = PyDict::new(py);
            for (k, v) in &self.inner.branches {
                d.set_item(PyBranchKey::from(k.clone()), v)?;
            }
            Ok::<_, PyErr>(d.unbind())
        })?;
        Ok(d.clone_ref(py))
    }

    #[getter]
    fn functions(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let t = self.functions.get_or_try_init(py, || {
            let items = self
                .inner
                .functions
                .iter()
                .map(|f| PyFunctionRecord::from(f.clone()));
            Ok::<_, PyErr>(PyTuple::new(py, items)?.unbind())
        })?;
        Ok(t.clone_ref(py))
    }

    #[getter]
    fn external(&self) -> bool {
        self.inner.external
    }

    fn status(&self, line: LineNo) -> PyLineStatus {
        self.inner.status(line).into()
    }
}
dataclass!(
    PyFileCoverage,
    "FileCoverage",
    ["path", "lines", "branches", "functions", "external"]
);

/// Not hashable, like the dataclass.
#[pyclass(module = "sundgren.core.model", name = "Snapshot", frozen, eq)]
pub struct PySnapshot {
    pub inner: model::Snapshot,
    files: PyOnceLock<Py<PyDict>>,
}

impl PartialEq for PySnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl From<model::Snapshot> for PySnapshot {
    fn from(inner: model::Snapshot) -> Self {
        Self {
            inner,
            files: PyOnceLock::new(),
        }
    }
}

#[pymethods]
impl PySnapshot {
    #[new]
    #[pyo3(signature = (files, test_names))]
    fn new(files: &Bound<'_, PyAny>, test_names: Vec<String>) -> PyResult<Self> {
        let files = mapping_items(files)?
            .into_iter()
            .map(|(k, v)| {
                Ok((
                    k.extract()?,
                    v.cast::<PyFileCoverage>()?.get().inner.clone(),
                ))
            })
            .collect::<PyResult<_>>()?;
        Ok(model::Snapshot { files, test_names }.into())
    }

    #[getter]
    fn files(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = self.files.get_or_try_init(py, || {
            let d = PyDict::new(py);
            for (k, v) in &self.inner.files {
                d.set_item(k, PyFileCoverage::from(v.clone()))?;
            }
            Ok::<_, PyErr>(d.unbind())
        })?;
        Ok(d.clone_ref(py))
    }

    #[getter]
    fn test_names<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, &self.inner.test_names)
    }
}
dataclass!(PySnapshot, "Snapshot", ["files", "test_names"]);

#[pyclass(
    module = "sundgren.core.model",
    name = "ReportWarning",
    frozen,
    eq,
    hash
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyReportWarning {
    pub inner: model::ReportWarning,
}

#[pymethods]
impl PyReportWarning {
    #[new]
    #[pyo3(signature = (code, message))]
    fn new(code: String, message: String) -> Self {
        model::ReportWarning { code, message }.into()
    }

    #[getter]
    fn code(&self) -> &str {
        &self.inner.code
    }

    #[getter]
    fn message(&self) -> &str {
        &self.inner.message
    }
}
dataclass!(PyReportWarning, "ReportWarning", ["code", "message"]);

impl From<model::ReportWarning> for PyReportWarning {
    fn from(inner: model::ReportWarning) -> Self {
        Self { inner }
    }
}

#[pyfunction]
fn merge_file_coverage(a: &PyFileCoverage, b: &PyFileCoverage) -> PyFileCoverage {
    model::merge_file_coverage(&a.inner, &b.inner).into()
}

#[pyfunction]
fn snapshot_to_dict<'py>(py: Python<'py>, snapshot: &PySnapshot) -> PyResult<Bound<'py, PyAny>> {
    json_to_py(py, &model::snapshot_to_dict(&snapshot.inner))
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_model")?;
    m.add_class::<PyLineStatus>()?;
    m.add_class::<PyBranchKey>()?;
    m.add_class::<PyFunctionRecord>()?;
    m.add_class::<PyFileCoverage>()?;
    m.add_class::<PySnapshot>()?;
    m.add_class::<PyReportWarning>()?;
    m.add_function(wrap_pyfunction!(merge_file_coverage, &m)?)?;
    m.add_function(wrap_pyfunction!(snapshot_to_dict, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
