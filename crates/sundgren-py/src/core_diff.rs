//! Bindings for `sundgren._native.core_diff`.
//!
//! Each class wraps its Rust value in `inner`; tuple fields are converted on first access and
//! cached, so repeated access returns the same object.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyString, PyTuple};
use sundgren_core::diff::{
    BranchCategory, BranchDelta, Delta, DiffError, DiffOptions, FileDelta, FlakyLine,
    LineCategory, LineDelta, OutcomePair, Side,
};
use sundgren_core::linemap::LineMap;
use sundgren_core::model::{Count, LineNo, Outcome, Snapshot};
use sundgren_core::pytext::PyText;

use crate::core_linemap::PyLineMap;
use crate::core_model::{PyReportWarning, PySnapshot, mapping_items};
use crate::core_rollup::{PyFunctionDelta, PyStats, PyTreeNode};
use crate::core_symbols::{into_pyerr, registry_lookup};
use crate::dataclass;
use crate::pytext::{opt_pytext_from_py, opt_pytext_into_py, pytext_from_py, pytext_into_py};

type Pair = (Option<Count>, Option<Count>);

fn side(s: &str) -> PyResult<Side> {
    match s {
        "base" => Ok(Side::Base),
        "patched" => Ok(Side::Patched),
        _ => Err(PyValueError::new_err(format!("unknown side {s:?}"))),
    }
}

fn line_category(s: &str) -> PyResult<LineCategory> {
    use LineCategory::*;
    [
        KeptHit,
        KeptMiss,
        Lost,
        Gained,
        Unmeasured,
        NewHit,
        NewMiss,
        DeletedHit,
        DeletedMiss,
    ]
    .into_iter()
    .find(|c| c.as_str() == s)
    .ok_or_else(|| PyValueError::new_err(format!("unknown line category {s:?}")))
}

fn branch_category(s: &str) -> PyResult<BranchCategory> {
    use BranchCategory::*;
    [
        KeptHit, KeptMiss, Lost, Gained, New, Deleted, Flaky, Unmeasured,
    ]
    .into_iter()
    .find(|c| c.as_str() == s)
    .ok_or_else(|| PyValueError::new_err(format!("unknown branch category {s:?}")))
}

fn outcome(s: &str) -> PyResult<Outcome> {
    s.parse().map_err(PyValueError::new_err)
}

/// A cached tuple of pyclass objects built from a Rust slice.
fn cached<'a, T, P>(
    py: Python<'_>,
    cell: &PyOnceLock<Py<PyTuple>>,
    items: &'a [T],
    wrap: impl Fn(&'a T) -> P,
) -> PyResult<Py<PyTuple>>
where
    P: pyo3::PyClass + Into<pyo3::PyClassInitializer<P>>,
{
    let t = cell.get_or_try_init(py, || {
        let objs = items
            .iter()
            .map(|x| Py::new(py, wrap(x)))
            .collect::<PyResult<Vec<_>>>()?;
        Ok::<_, PyErr>(PyTuple::new(py, objs)?.unbind())
    })?;
    Ok(t.clone_ref(py))
}

fn cast_all<T, P>(obj: &Bound<'_, PyAny>, get: impl Fn(&P) -> T) -> PyResult<Vec<T>>
where
    P: pyo3::PyClass + Sync,
    P: pyo3::PyClass<Frozen = pyo3::pyclass::boolean_struct::True>,
{
    obj.try_iter()?
        .map(|x| Ok(get(x?.cast::<P>()?.get())))
        .collect()
}

#[pyclass(module = "sundgren.core.diff", name = "LineDelta", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyLineDelta {
    pub inner: LineDelta,
}

impl From<LineDelta> for PyLineDelta {
    fn from(inner: LineDelta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyLineDelta {
    #[new]
    #[pyo3(signature = (line, base_line, category, hits))]
    fn new(
        line: Option<LineNo>,
        base_line: Option<LineNo>,
        category: &str,
        hits: Pair,
    ) -> PyResult<Self> {
        Ok(LineDelta {
            line,
            base_line,
            category: line_category(category)?,
            hits,
        }
        .into())
    }

    #[getter]
    fn line(&self) -> Option<LineNo> {
        self.inner.line
    }

    #[getter]
    fn base_line(&self) -> Option<LineNo> {
        self.inner.base_line
    }

    #[getter]
    fn category(&self) -> &'static str {
        self.inner.category.as_str()
    }

    #[getter]
    fn hits(&self) -> Pair {
        self.inner.hits
    }
}
dataclass!(
    PyLineDelta,
    "LineDelta",
    ["line", "base_line", "category", "hits"]
);

#[pyclass(module = "sundgren.core.diff", name = "BranchDelta", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyBranchDelta {
    pub inner: BranchDelta,
}

impl From<BranchDelta> for PyBranchDelta {
    fn from(inner: BranchDelta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBranchDelta {
    #[new]
    #[pyo3(signature = (line, base_line, block, branch, category, taken))]
    fn new(
        line: Option<LineNo>,
        base_line: Option<LineNo>,
        block: String,
        branch: String,
        category: &str,
        taken: Pair,
    ) -> PyResult<Self> {
        Ok(BranchDelta {
            line,
            base_line,
            block,
            branch,
            category: branch_category(category)?,
            taken,
        }
        .into())
    }

    #[getter]
    fn line(&self) -> Option<LineNo> {
        self.inner.line
    }

    #[getter]
    fn base_line(&self) -> Option<LineNo> {
        self.inner.base_line
    }

    #[getter]
    fn block(&self) -> &str {
        &self.inner.block
    }

    #[getter]
    fn branch(&self) -> &str {
        &self.inner.branch
    }

    #[getter]
    fn category(&self) -> &'static str {
        self.inner.category.as_str()
    }

    #[getter]
    fn taken(&self) -> Pair {
        self.inner.taken
    }
}
dataclass!(
    PyBranchDelta,
    "BranchDelta",
    ["line", "base_line", "block", "branch", "category", "taken"]
);

#[pyclass(module = "sundgren.core.diff", name = "FlakyLine", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyFlakyLine {
    pub inner: FlakyLine,
}

impl From<FlakyLine> for PyFlakyLine {
    fn from(inner: FlakyLine) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFlakyLine {
    #[new]
    #[pyo3(signature = (line, base_line, hits, sides))]
    fn new(
        line: Option<LineNo>,
        base_line: Option<LineNo>,
        hits: Pair,
        sides: Vec<String>,
    ) -> PyResult<Self> {
        Ok(FlakyLine {
            line,
            base_line,
            hits,
            sides: sides.iter().map(|s| side(s)).collect::<PyResult<_>>()?,
        }
        .into())
    }

    #[getter]
    fn line(&self) -> Option<LineNo> {
        self.inner.line
    }

    #[getter]
    fn base_line(&self) -> Option<LineNo> {
        self.inner.base_line
    }

    #[getter]
    fn hits(&self) -> Pair {
        self.inner.hits
    }

    #[getter]
    fn sides<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, self.inner.sides.iter().map(|s| s.as_str()))
    }
}
dataclass!(
    PyFlakyLine,
    "FlakyLine",
    ["line", "base_line", "hits", "sides"]
);

#[pyclass(module = "sundgren.core.diff", name = "FileDelta", frozen, eq, hash)]
pub struct PyFileDelta {
    pub inner: FileDelta,
    lines: PyOnceLock<Py<PyTuple>>,
    branches: PyOnceLock<Py<PyTuple>>,
    functions: PyOnceLock<Py<PyTuple>>,
    flaky: PyOnceLock<Py<PyTuple>>,
}

impl PartialEq for PyFileDelta {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Hash for PyFileDelta {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl From<FileDelta> for PyFileDelta {
    fn from(inner: FileDelta) -> Self {
        Self {
            inner,
            lines: PyOnceLock::new(),
            branches: PyOnceLock::new(),
            functions: PyOnceLock::new(),
            flaky: PyOnceLock::new(),
        }
    }
}

#[pymethods]
impl PyFileDelta {
    #[new]
    #[pyo3(signature = (
        path, base_path, sha256, source_changed, inferred_side, lines, branches, functions,
        flaky, stats,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        path: &Bound<'_, PyAny>,
        base_path: &Bound<'_, PyAny>,
        sha256: (Option<String>, Option<String>),
        source_changed: bool,
        inferred_side: Option<String>,
        lines: &Bound<'_, PyAny>,
        branches: &Bound<'_, PyAny>,
        functions: &Bound<'_, PyAny>,
        flaky: &Bound<'_, PyAny>,
        stats: &PyStats,
    ) -> PyResult<Self> {
        Ok(FileDelta {
            path: pytext_from_py(path)?,
            base_path: opt_pytext_from_py(base_path)?,
            sha256,
            source_changed,
            inferred_side: inferred_side.as_deref().map(side).transpose()?,
            lines: cast_all(lines, |x: &PyLineDelta| x.inner.clone())?,
            branches: cast_all(branches, |x: &PyBranchDelta| x.inner.clone())?,
            functions: cast_all(functions, |x: &PyFunctionDelta| x.inner.clone())?,
            flaky: cast_all(flaky, |x: &PyFlakyLine| x.inner.clone())?,
            stats: stats.inner,
        }
        .into())
    }

    #[getter]
    fn path<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        pytext_into_py(py, &self.inner.path)
    }

    #[getter]
    fn base_path<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        opt_pytext_into_py(py, self.inner.base_path.as_ref())
    }

    #[getter]
    fn sha256(&self) -> (Option<String>, Option<String>) {
        self.inner.sha256.clone()
    }

    #[getter]
    fn source_changed(&self) -> bool {
        self.inner.source_changed
    }

    #[getter]
    fn inferred_side(&self) -> Option<&'static str> {
        self.inner.inferred_side.map(Side::as_str)
    }

    #[getter]
    fn lines(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.lines, &self.inner.lines, |x| {
            PyLineDelta::from(x.clone())
        })
    }

    #[getter]
    fn branches(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.branches, &self.inner.branches, |x| {
            PyBranchDelta::from(x.clone())
        })
    }

    #[getter]
    fn functions(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.functions, &self.inner.functions, |x| {
            PyFunctionDelta::from(x.clone())
        })
    }

    #[getter]
    fn flaky(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.flaky, &self.inner.flaky, |x| {
            PyFlakyLine::from(x.clone())
        })
    }

    #[getter]
    fn stats(&self) -> PyStats {
        self.inner.stats.into()
    }

    /// Anything beyond kept_* lines and branches.
    #[getter]
    fn changed(&self) -> bool {
        self.inner.changed()
    }
}
dataclass!(
    PyFileDelta,
    "FileDelta",
    [
        "path",
        "base_path",
        "sha256",
        "source_changed",
        "inferred_side",
        "lines",
        "branches",
        "functions",
        "flaky",
        "stats"
    ]
);

#[pyclass(module = "sundgren.core.diff", name = "OutcomePair", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyOutcomePair {
    pub inner: OutcomePair,
}

impl From<OutcomePair> for PyOutcomePair {
    fn from(inner: OutcomePair) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOutcomePair {
    #[new]
    #[pyo3(signature = (id, base, patched))]
    fn new(id: String, base: &str, patched: &str) -> PyResult<Self> {
        Ok(OutcomePair {
            id,
            base: outcome(base)?,
            patched: outcome(patched)?,
        }
        .into())
    }

    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    #[getter]
    fn base(&self) -> &'static str {
        self.inner.base.as_str()
    }

    #[getter]
    fn patched(&self) -> &'static str {
        self.inner.patched.as_str()
    }
}
dataclass!(PyOutcomePair, "OutcomePair", ["id", "base", "patched"]);

#[pyclass(module = "sundgren.core.diff", name = "Delta", frozen, eq, hash)]
pub struct PyDelta {
    pub inner: Delta,
    files: PyOnceLock<Py<PyTuple>>,
    tests: PyOnceLock<Py<PyTuple>>,
    warnings: PyOnceLock<Py<PyTuple>>,
    tree: PyOnceLock<Py<PyTreeNode>>,
}

impl PartialEq for PyDelta {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Hash for PyDelta {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl From<Delta> for PyDelta {
    fn from(inner: Delta) -> Self {
        Self {
            inner,
            files: PyOnceLock::new(),
            tests: PyOnceLock::new(),
            warnings: PyOnceLock::new(),
            tree: PyOnceLock::new(),
        }
    }
}

#[pymethods]
impl PyDelta {
    #[new]
    #[pyo3(signature = (files, tests, tree, external_files, warnings))]
    fn new(
        files: &Bound<'_, PyAny>,
        tests: &Bound<'_, PyAny>,
        tree: &PyTreeNode,
        external_files: i64,
        warnings: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Delta {
            files: cast_all(files, |x: &PyFileDelta| x.inner.clone())?,
            tests: cast_all(tests, |x: &PyOutcomePair| x.inner.clone())?,
            tree: tree.inner.clone(),
            external_files,
            warnings: cast_all(warnings, |x: &PyReportWarning| x.inner.clone())?,
        }
        .into())
    }

    #[getter]
    fn files(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.files, &self.inner.files, |x| {
            PyFileDelta::from(x.clone())
        })
    }

    #[getter]
    fn tests(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.tests, &self.inner.tests, |x| {
            PyOutcomePair::from(x.clone())
        })
    }

    #[getter]
    fn tree(&self, py: Python<'_>) -> PyResult<Py<PyTreeNode>> {
        let t = self.tree.get_or_try_init(py, || {
            Py::new(py, PyTreeNode::from(self.inner.tree.clone()))
        })?;
        Ok(t.clone_ref(py))
    }

    #[getter]
    fn external_files(&self) -> i64 {
        self.inner.external_files
    }

    #[getter]
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        cached(py, &self.warnings, &self.inner.warnings, |x| {
            PyReportWarning::from(x.clone())
        })
    }
}
dataclass!(
    PyDelta,
    "Delta",
    ["files", "tests", "tree", "external_files", "warnings"]
);

fn snapshots(obj: &Bound<'_, PyAny>) -> PyResult<Vec<Snapshot>> {
    cast_all(obj, |s: &PySnapshot| s.inner.clone())
}

fn outcomes(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<BTreeMap<String, Outcome>>> {
    let Some(obj) = obj.filter(|o| !o.is_none()) else {
        return Ok(None);
    };
    mapping_items(obj)?
        .into_iter()
        .map(|(k, v)| Ok((k.extract()?, outcome(&v.extract::<String>()?)?)))
        .collect::<PyResult<_>>()
        .map(Some)
}

/// Classify every line, branch and function; [0] of each side is canonical, the rest repeats.
#[pyfunction(name = "diff")]
#[pyo3(signature = (
    base, patched, linemaps, *,
    base_root = None, patched_root = None, base_outcomes = None, patched_outcomes = None,
))]
#[allow(clippy::too_many_arguments)]
fn diff_py(
    py: Python<'_>,
    base: &Bound<'_, PyAny>,
    patched: &Bound<'_, PyAny>,
    linemaps: &Bound<'_, PyAny>,
    base_root: Option<PathBuf>,
    patched_root: Option<PathBuf>,
    base_outcomes: Option<&Bound<'_, PyAny>>,
    patched_outcomes: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyDelta> {
    let base = snapshots(base)?;
    let patched = snapshots(patched)?;
    let linemaps = mapping_items(linemaps)?
        .into_iter()
        .map(|(k, v)| Ok((pytext_from_py(&k)?, v.cast::<PyLineMap>()?.get().inner.clone())))
        .collect::<PyResult<BTreeMap<PyText, LineMap>>>()?;
    let base_outcomes = outcomes(base_outcomes)?;
    let patched_outcomes = outcomes(patched_outcomes)?;
    let opts = DiffOptions {
        base_root: base_root.as_deref(),
        patched_root: patched_root.as_deref(),
        base_outcomes: base_outcomes.as_ref(),
        patched_outcomes: patched_outcomes.as_ref(),
    };
    let lookup = registry_lookup(py);
    match sundgren_core::diff::diff(&base, &patched, &linemaps, opts, &lookup) {
        Ok(delta) => Ok(delta.into()),
        Err(DiffError::NoSnapshots) => Err(PyValueError::new_err(DiffError::NoSnapshots.to_string())),
        Err(DiffError::Symbolizer(e)) => Err(into_pyerr(e)),
        Err(DiffError::Io(e)) => Err(e.into()),
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_diff")?;
    m.add_class::<PyLineDelta>()?;
    m.add_class::<PyBranchDelta>()?;
    m.add_class::<PyFlakyLine>()?;
    m.add_class::<PyFileDelta>()?;
    m.add_class::<PyOutcomePair>()?;
    m.add_class::<PyDelta>()?;
    m.add_function(wrap_pyfunction!(diff_py, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
