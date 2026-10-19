//! Bindings for `sundgren._native.core_patch`.

use pyo3::exceptions::{PyAssertionError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};
use sundgren_core::patch::{FilePatch, Hunk, LineNo, Patch, PatchError, parse_patch};

use crate::dataclass;
use crate::pytext::{opt_pytext_from_py, opt_pytext_into_py, pytext_from_py, pytext_into_py};

#[pyclass(frozen, eq, hash, module = "sundgren.core.patch", name = "Hunk")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyHunk {
    pub inner: Hunk,
}

#[pymethods]
impl PyHunk {
    #[new]
    fn new(
        old_start: LineNo,
        old_count: LineNo,
        new_start: LineNo,
        new_count: LineNo,
        lines: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let lines = lines
            .try_iter()?
            .map(|l| pytext_from_py(&l?))
            .collect::<PyResult<_>>()?;
        Ok(Self {
            inner: Hunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines,
            },
        })
    }

    #[getter]
    fn old_start(&self) -> LineNo {
        self.inner.old_start
    }

    #[getter]
    fn old_count(&self) -> LineNo {
        self.inner.old_count
    }

    #[getter]
    fn new_start(&self) -> LineNo {
        self.inner.new_start
    }

    #[getter]
    fn new_count(&self) -> LineNo {
        self.inner.new_count
    }

    #[getter]
    fn lines<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let lines = self
            .inner
            .lines
            .iter()
            .map(|l| pytext_into_py(py, l))
            .collect::<PyResult<Vec<_>>>()?;
        PyTuple::new(py, lines)
    }
}

#[pyclass(frozen, eq, hash, module = "sundgren.core.patch", name = "FilePatch")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyFilePatch {
    pub inner: FilePatch,
}

dataclass!(
    PyHunk,
    "Hunk",
    ["old_start", "old_count", "new_start", "new_count", "lines"]
);

#[pymethods]
impl PyFilePatch {
    #[new]
    #[pyo3(signature = (old_path, new_path, hunks, binary = false))]
    fn new(
        old_path: &Bound<'_, PyAny>,
        new_path: &Bound<'_, PyAny>,
        hunks: &Bound<'_, PyAny>,
        binary: bool,
    ) -> PyResult<Self> {
        let hunks = hunks
            .try_iter()?
            .map(|h| Ok(h?.extract::<PyRef<'_, PyHunk>>()?.inner.clone()))
            .collect::<PyResult<_>>()?;
        Ok(Self {
            inner: FilePatch {
                old_path: opt_pytext_from_py(old_path)?,
                new_path: opt_pytext_from_py(new_path)?,
                hunks,
                binary,
            },
        })
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
    fn hunks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let hunks = self
            .inner
            .hunks
            .iter()
            .map(|h| Bound::new(py, PyHunk { inner: h.clone() }))
            .collect::<PyResult<Vec<_>>>()?;
        PyTuple::new(py, hunks)
    }

    #[getter]
    fn binary(&self) -> bool {
        self.inner.binary
    }

    #[getter]
    fn path<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        let fp = &self.inner;
        match fp.new_path.as_ref().or(fp.old_path.as_ref()) {
            Some(p) => pytext_into_py(py, p),
            None => Err(PyAssertionError::new_err(())),
        }
    }
}

#[pyclass(frozen, eq, hash, module = "sundgren.core.patch", name = "Patch")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyPatch {
    pub inner: Patch,
}

dataclass!(
    PyFilePatch,
    "FilePatch",
    ["old_path", "new_path", "hunks", "binary"]
);

#[pymethods]
impl PyPatch {
    #[new]
    #[pyo3(signature = (files, warnings = None))]
    fn new(files: &Bound<'_, PyAny>, warnings: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let files = files
            .try_iter()?
            .map(|f| Ok(f?.extract::<PyRef<'_, PyFilePatch>>()?.inner.clone()))
            .collect::<PyResult<_>>()?;
        let warnings = match warnings {
            Some(w) => w
                .try_iter()?
                .map(|w| pytext_from_py(&w?))
                .collect::<PyResult<_>>()?,
            None => Vec::new(),
        };
        Ok(Self {
            inner: Patch { files, warnings },
        })
    }

    #[getter]
    fn files<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let files = self
            .inner
            .files
            .iter()
            .map(|f| Bound::new(py, PyFilePatch { inner: f.clone() }))
            .collect::<PyResult<Vec<_>>>()?;
        PyTuple::new(py, files)
    }

    #[getter]
    fn warnings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        texts_into_py(py, &self.inner.warnings)
    }

    /// Sorted old and new paths of every file in the patch.
    fn touched_paths<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        texts_into_py(py, &self.inner.touched_paths())
    }
}

dataclass!(PyPatch, "Patch", ["files", "warnings"]);

fn texts_into_py<'py>(
    py: Python<'py>,
    texts: &[sundgren_core::pytext::PyText],
) -> PyResult<Bound<'py, PyTuple>> {
    let items = texts
        .iter()
        .map(|t| pytext_into_py(py, t))
        .collect::<PyResult<Vec<_>>>()?;
    PyTuple::new(py, items)
}

/// Wrap a Rust `Patch` as the Python `Patch` class.
pub(crate) fn patch_into_py(py: Python<'_>, patch: Patch) -> PyResult<Bound<'_, PyPatch>> {
    Bound::new(py, PyPatch { inner: patch })
}

/// `PatchError` as the `ValueError` the prototype raised, message included.
pub(crate) fn patch_error_into_py(py: Python<'_>, err: PatchError) -> PyErr {
    let msg = match err {
        PatchError::At { line, msg } => {
            let mut text = sundgren_core::pytext::PyText::from(format!("line {line}: "));
            text.push_text(&msg);
            pytext_into_py(py, &text).map(Bound::into_any)
        }
        PatchError::BadQuotedPath { line, raw } => (|| {
            let repr = pytext_into_py(py, &raw)?.repr()?;
            PyString::new(py, &format!("line {line}: bad quoted path ")).add(repr)
        })(),
    };
    match msg {
        Ok(msg) => PyValueError::new_err(msg.unbind()),
        Err(e) => e,
    }
}

/// Parse a git-style unified diff; raise ValueError on malformed input.
#[pyfunction(name = "parse_patch")]
fn py_parse_patch<'py>(py: Python<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyPatch>> {
    let text = pytext_from_py(text)?;
    let patch = parse_patch(&text).map_err(|e| patch_error_into_py(py, e))?;
    patch_into_py(py, patch)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_patch")?;
    m.add_class::<PyHunk>()?;
    m.add_class::<PyFilePatch>()?;
    m.add_class::<PyPatch>()?;
    m.add_function(wrap_pyfunction!(py_parse_patch, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}

impl From<Hunk> for PyHunk {
    fn from(inner: Hunk) -> Self {
        Self { inner }
    }
}

impl From<FilePatch> for PyFilePatch {
    fn from(inner: FilePatch) -> Self {
        Self { inner }
    }
}

impl From<Patch> for PyPatch {
    fn from(inner: Patch) -> Self {
        Self { inner }
    }
}
