//! Bindings for `sundgren._native.core_linemap`.

use std::collections::{BTreeMap, BTreeSet};

use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyDict, PyFrozenSet};
use sundgren_core::linemap::{self, LineMap};
use sundgren_core::model::LineNo;

use crate::core_model::mapping_items;
use crate::core_patch::PyPatch;
use crate::dataclass;
use crate::pytext::{opt_pytext_from_py, opt_pytext_into_py, pytext_from_py, pytext_into_py};

/// Not hashable, like the dataclass (it holds a dict).
#[pyclass(frozen, eq, module = "sundgren.core.linemap", name = "LineMap")]
pub struct PyLineMap {
    pub inner: LineMap,
    old_to_new: PyOnceLock<Py<PyDict>>,
    deleted_old: PyOnceLock<Py<PyFrozenSet>>,
    added_new: PyOnceLock<Py<PyFrozenSet>>,
}

impl PartialEq for PyLineMap {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl From<LineMap> for PyLineMap {
    fn from(inner: LineMap) -> Self {
        Self {
            inner,
            old_to_new: PyOnceLock::new(),
            deleted_old: PyOnceLock::new(),
            added_new: PyOnceLock::new(),
        }
    }
}

fn line_set(obj: &Bound<'_, PyAny>) -> PyResult<BTreeSet<LineNo>> {
    obj.try_iter()?.map(|x| x?.extract()).collect()
}

#[pymethods]
impl PyLineMap {
    #[new]
    #[pyo3(signature = (old_path, new_path, old_to_new, deleted_old, added_new))]
    fn new(
        old_path: &Bound<'_, PyAny>,
        new_path: &Bound<'_, PyAny>,
        old_to_new: &Bound<'_, PyAny>,
        deleted_old: &Bound<'_, PyAny>,
        added_new: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let pairs = mapping_items(old_to_new)?
            .into_iter()
            .map(|(k, v)| Ok((k.extract()?, v.extract()?)))
            .collect::<PyResult<BTreeMap<LineNo, LineNo>>>()?;
        Ok(LineMap::new(
            opt_pytext_from_py(old_path)?,
            opt_pytext_from_py(new_path)?,
            pairs,
            line_set(deleted_old)?,
            line_set(added_new)?,
        )
        .into())
    }

    #[getter]
    fn old_path<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        opt_pytext_into_py(py, self.inner.old_path.as_ref())
    }

    #[getter]
    fn new_path<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        opt_pytext_into_py(py, self.inner.new_path.as_ref())
    }

    #[getter]
    fn old_to_new(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = self.old_to_new.get_or_try_init(py, || {
            Ok::<_, PyErr>(self.inner.old_to_new().into_pyobject(py)?.unbind())
        })?;
        Ok(d.clone_ref(py))
    }

    #[getter]
    fn deleted_old(&self, py: Python<'_>) -> PyResult<Py<PyFrozenSet>> {
        let s = self.deleted_old.get_or_try_init(py, || {
            Ok::<_, PyErr>(PyFrozenSet::new(py, self.inner.deleted_old())?.unbind())
        })?;
        Ok(s.clone_ref(py))
    }

    #[getter]
    fn added_new(&self, py: Python<'_>) -> PyResult<Py<PyFrozenSet>> {
        let s = self.added_new.get_or_try_init(py, || {
            Ok::<_, PyErr>(PyFrozenSet::new(py, self.inner.added_new())?.unbind())
        })?;
        Ok(s.clone_ref(py))
    }

    /// Patched line for a base line, or None if it was deleted.
    fn to_new(&self, old: LineNo) -> Option<LineNo> {
        self.inner.to_new(old)
    }

    /// Base line for a patched line, or None if it was added.
    fn to_old(&self, new: LineNo) -> Option<LineNo> {
        self.inner.to_old(new)
    }

    #[getter]
    fn is_identity(&self) -> bool {
        self.inner.is_identity()
    }
}

dataclass!(
    PyLineMap,
    "LineMap",
    [
        "old_path",
        "new_path",
        "old_to_new",
        "deleted_old",
        "added_new"
    ]
);

#[pyfunction]
fn identity(path: &Bound<'_, PyAny>) -> PyResult<PyLineMap> {
    Ok(linemap::identity(pytext_from_py(path)?).into())
}

/// LineMap for every file the patch touches, keyed by both its old and new path.
#[pyfunction]
fn build_linemaps<'py>(py: Python<'py>, patch: PyRef<'_, PyPatch>) -> PyResult<Bound<'py, PyDict>> {
    let maps = linemap::file_linemaps(&patch.inner);
    let keys = linemap::linemap_keys(&maps);
    // One object per file, under both of its paths.
    let objs = maps
        .into_iter()
        .map(|lm| Bound::new(py, PyLineMap::from(lm)))
        .collect::<PyResult<Vec<_>>>()?;
    let out = PyDict::new(py);
    for (path, i) in keys {
        out.set_item(pytext_into_py(py, &path)?, &objs[i])?;
    }
    Ok(out)
}

/// The LineMap touching `path`, or the identity map.
#[pyfunction]
fn linemap_for<'py>(
    linemaps: &Bound<'py, PyAny>,
    path: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let found = linemaps.call_method1("get", (path,))?;
    if found.is_truthy()? {
        return Ok(found);
    }
    Ok(Bound::new(path.py(), identity(path)?)?.into_any())
}

/// The Rust LineMaps behind a Python `{path: LineMap}` mapping. For core_diff.
#[allow(dead_code)]
pub(crate) fn linemaps_from_py(
    obj: &Bound<'_, PyAny>,
) -> PyResult<BTreeMap<sundgren_core::pytext::PyText, LineMap>> {
    mapping_items(obj)?
        .into_iter()
        .map(|(k, v)| {
            Ok((
                pytext_from_py(&k)?,
                v.cast::<PyLineMap>()?.get().inner.clone(),
            ))
        })
        .collect()
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_linemap")?;
    m.add_class::<PyLineMap>()?;
    m.add_function(wrap_pyfunction!(identity, &m)?)?;
    m.add_function(wrap_pyfunction!(build_linemaps, &m)?)?;
    m.add_function(wrap_pyfunction!(linemap_for, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
