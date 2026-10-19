//! Bindings for `sundgren._native.core_rollup`.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyString, PyTuple};
use sundgren_core::model::{Count, LineNo};
use sundgren_core::rollup::{
    self, FunctionDelta, FunctionStatus, LineSets, NodeKind, Stats, TreeNode,
};
use sundgren_core::symbols::Span;

use crate::core_linemap::PyLineMap;
use crate::core_model::mapping_items;
use crate::dataclass;
use crate::pytext::{pytext_from_py, pytext_into_py};

#[pyclass(
    module = "sundgren.core.rollup",
    name = "FunctionDelta",
    frozen,
    eq,
    hash
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyFunctionDelta {
    pub inner: FunctionDelta,
}

impl From<FunctionDelta> for PyFunctionDelta {
    fn from(inner: FunctionDelta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFunctionDelta {
    #[new]
    #[pyo3(signature = (name, start, end, status, lost, gained, executable))]
    fn new(
        name: String,
        start: LineNo,
        end: LineNo,
        status: &str,
        lost: i64,
        gained: i64,
        executable: i64,
    ) -> PyResult<Self> {
        let status = FunctionStatus::parse(status)
            .ok_or_else(|| PyValueError::new_err(format!("unknown function status {status:?}")))?;
        Ok(FunctionDelta {
            name,
            start,
            end,
            status,
            lost,
            gained,
            executable,
        }
        .into())
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
    fn status(&self) -> &'static str {
        self.inner.status.as_str()
    }

    #[getter]
    fn lost(&self) -> i64 {
        self.inner.lost
    }

    #[getter]
    fn gained(&self) -> i64 {
        self.inner.gained
    }

    #[getter]
    fn executable(&self) -> i64 {
        self.inner.executable
    }
}
dataclass!(
    PyFunctionDelta,
    "FunctionDelta",
    ["name", "start", "end", "status", "lost", "gained", "executable"]
);

#[pyclass(module = "sundgren.core.rollup", name = "Stats", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyStats {
    pub inner: Stats,
}

impl From<Stats> for PyStats {
    fn from(inner: Stats) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStats {
    #[new]
    #[pyo3(signature = (
        lost = 0, gained = 0, new_hit = 0, new_miss = 0, deleted_hit = 0, deleted_miss = 0,
        executable_base = 0, executable_patched = 0, hit_base = 0, hit_patched = 0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        lost: i64,
        gained: i64,
        new_hit: i64,
        new_miss: i64,
        deleted_hit: i64,
        deleted_miss: i64,
        executable_base: i64,
        executable_patched: i64,
        hit_base: i64,
        hit_patched: i64,
    ) -> Self {
        Stats {
            lost,
            gained,
            new_hit,
            new_miss,
            deleted_hit,
            deleted_miss,
            executable_base,
            executable_patched,
            hit_base,
            hit_patched,
        }
        .into()
    }

    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let py = other.py();
        match other.cast::<PyStats>() {
            Ok(o) => Ok(Py::new(py, PyStats::from(self.inner + o.get().inner))?.into_any()),
            Err(_) => Ok(py.NotImplemented()),
        }
    }

    #[getter]
    fn lost(&self) -> i64 {
        self.inner.lost
    }

    #[getter]
    fn gained(&self) -> i64 {
        self.inner.gained
    }

    #[getter]
    fn new_hit(&self) -> i64 {
        self.inner.new_hit
    }

    #[getter]
    fn new_miss(&self) -> i64 {
        self.inner.new_miss
    }

    #[getter]
    fn deleted_hit(&self) -> i64 {
        self.inner.deleted_hit
    }

    #[getter]
    fn deleted_miss(&self) -> i64 {
        self.inner.deleted_miss
    }

    #[getter]
    fn executable_base(&self) -> i64 {
        self.inner.executable_base
    }

    #[getter]
    fn executable_patched(&self) -> i64 {
        self.inner.executable_patched
    }

    #[getter]
    fn hit_base(&self) -> i64 {
        self.inner.hit_base
    }

    #[getter]
    fn hit_patched(&self) -> i64 {
        self.inner.hit_patched
    }
}
dataclass!(
    PyStats,
    "Stats",
    [
        "lost",
        "gained",
        "new_hit",
        "new_miss",
        "deleted_hit",
        "deleted_miss",
        "executable_base",
        "executable_patched",
        "hit_base",
        "hit_patched"
    ]
);

#[pyclass(module = "sundgren.core.rollup", name = "TreeNode", frozen, eq, hash)]
pub struct PyTreeNode {
    pub inner: TreeNode,
    children: PyOnceLock<Py<PyTuple>>,
}

impl PartialEq for PyTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Hash for PyTreeNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl From<TreeNode> for PyTreeNode {
    fn from(inner: TreeNode) -> Self {
        Self {
            inner,
            children: PyOnceLock::new(),
        }
    }
}

fn node_kind(kind: &str) -> PyResult<NodeKind> {
    match kind {
        "dir" => Ok(NodeKind::Dir),
        "file" => Ok(NodeKind::File),
        _ => Err(PyValueError::new_err(format!("unknown node kind {kind:?}"))),
    }
}

#[pymethods]
impl PyTreeNode {
    #[new]
    #[pyo3(signature = (name, path, kind, stats, children = None))]
    fn new(
        name: &Bound<'_, PyAny>,
        path: &Bound<'_, PyAny>,
        kind: &str,
        stats: &PyStats,
        children: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let children = match children {
            None => Vec::new(),
            Some(c) => c
                .try_iter()?
                .map(|n| Ok(n?.cast::<PyTreeNode>()?.get().inner.clone()))
                .collect::<PyResult<_>>()?,
        };
        Ok(TreeNode {
            name: pytext_from_py(name)?,
            path: pytext_from_py(path)?,
            kind: node_kind(kind)?,
            stats: stats.inner,
            children,
        }
        .into())
    }

    #[getter]
    fn name<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        pytext_into_py(py, &self.inner.name)
    }

    #[getter]
    fn path<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        pytext_into_py(py, &self.inner.path)
    }

    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.kind.as_str()
    }

    #[getter]
    fn stats(&self) -> PyStats {
        self.inner.stats.into()
    }

    #[getter]
    fn children(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let t = self.children.get_or_try_init(py, || {
            let items = self
                .inner
                .children
                .iter()
                .map(|c| PyTreeNode::from(c.clone()));
            Ok::<_, PyErr>(PyTuple::new(py, items)?.unbind())
        })?;
        Ok(t.clone_ref(py))
    }
}
dataclass!(
    PyTreeNode,
    "TreeNode",
    ["name", "path", "kind", "stats", "children"]
);

/// A Span from any object with the Span attributes.
pub fn span_from_py(obj: &Bound<'_, PyAny>) -> PyResult<Span> {
    Ok(Span {
        name: obj.getattr("name")?.extract()?,
        start: obj.getattr("start")?.extract()?,
        end: obj.getattr("end")?.extract()?,
        body_start: obj.getattr("body_start")?.extract()?,
    })
}

fn spans_from_py(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Vec<Span>>> {
    obj.map(|o| o.try_iter()?.map(|s| span_from_py(&s?)).collect())
        .transpose()
}

fn lines_from_py(obj: &Bound<'_, PyAny>) -> PyResult<BTreeMap<LineNo, Count>> {
    mapping_items(obj)?
        .into_iter()
        .map(|(k, v)| Ok((k.extract()?, v.extract()?)))
        .collect()
}

fn set_from_py(obj: Option<&Bound<'_, PyAny>>) -> PyResult<BTreeSet<LineNo>> {
    match obj {
        None => Ok(BTreeSet::new()),
        Some(o) => o.try_iter()?.map(|x| x?.extract()).collect(),
    }
}

#[pyfunction]
#[pyo3(signature = (
    base_spans, patched_spans, lm, base_lines, patched_lines, *,
    lost = None, gained = None, flaky_base = None, flaky_patched = None,
))]
#[allow(clippy::too_many_arguments)]
fn function_deltas<'py>(
    py: Python<'py>,
    base_spans: Option<&Bound<'py, PyAny>>,
    patched_spans: Option<&Bound<'py, PyAny>>,
    lm: &PyLineMap,
    base_lines: &Bound<'py, PyAny>,
    patched_lines: &Bound<'py, PyAny>,
    lost: Option<&Bound<'py, PyAny>>,
    gained: Option<&Bound<'py, PyAny>>,
    flaky_base: Option<&Bound<'py, PyAny>>,
    flaky_patched: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyTuple>> {
    let base_spans = spans_from_py(base_spans)?;
    let patched_spans = spans_from_py(patched_spans)?;
    let sets = LineSets {
        lost: set_from_py(lost)?,
        gained: set_from_py(gained)?,
        flaky_base: set_from_py(flaky_base)?,
        flaky_patched: set_from_py(flaky_patched)?,
    };
    let out = rollup::function_deltas(
        base_spans.as_deref(),
        patched_spans.as_deref(),
        &lm.inner,
        &lines_from_py(base_lines)?,
        &lines_from_py(patched_lines)?,
        &sets,
    );
    PyTuple::new(py, out.into_iter().map(PyFunctionDelta::from))
}

/// Directory tree over repo-relative posix paths; dir stats are sums of their children.
#[pyfunction]
fn build_tree(files: &Bound<'_, PyAny>) -> PyResult<PyTreeNode> {
    let entries = files
        .try_iter()?
        .map(|item| {
            let (path, stats): (Bound<'_, PyAny>, Bound<'_, PyAny>) = item?.extract()?;
            Ok((pytext_from_py(&path)?, stats.cast::<PyStats>()?.get().inner))
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(rollup::build_tree(entries).into())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_rollup")?;
    m.add_class::<PyFunctionDelta>()?;
    m.add_class::<PyStats>()?;
    m.add_class::<PyTreeNode>()?;
    m.add_function(wrap_pyfunction!(function_deltas, &m)?)?;
    m.add_function(wrap_pyfunction!(build_tree, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
