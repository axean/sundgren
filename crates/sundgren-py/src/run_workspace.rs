//! Bindings for `sundgren._native.run_workspace`.

use std::path::PathBuf;
use std::sync::Mutex;

use pyo3::create_exception;
use pyo3::exceptions::{PyAttributeError, PyException, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use sundgren_core::pytext::PyText;
use sundgren_run::workspace::{self, Workspace, WorkspaceError};

use crate::pytext::pytext_into_py;

create_exception!(
    sundgren.run.workspace,
    PatchApplyError,
    PyException,
    "git apply failed; str(exc) is git's message."
);
create_exception!(
    sundgren.run.workspace,
    GitError,
    PyRuntimeError,
    "A git command other than `git apply` failed; str(exc) is git's message."
);
create_exception!(
    sundgren.run.workspace,
    BadRefError,
    GitError,
    "The base ref doesn't name a commit."
);

/// Python error for a `WorkspaceError`.
pub fn workspace_err(err: WorkspaceError) -> PyErr {
    match err {
        WorkspaceError::PatchApply(m) => PatchApplyError::new_err(m),
        WorkspaceError::Git(m) => GitError::new_err(m),
        WorkspaceError::BadRef(m) => BadRefError::new_err(m),
        WorkspaceError::Io(e) => e.into(),
    }
}

/// Context manager: base and patched worktrees, removed on exit unless keep.
#[pyclass(module = "sundgren.run.workspace", name = "Workspace", frozen)]
pub struct PyWorkspace {
    inner: Mutex<Workspace>,
}

fn unset(name: &str) -> PyErr {
    PyAttributeError::new_err(format!("'Workspace' object has no attribute '{name}'"))
}

impl PyWorkspace {
    fn get<T>(&self, name: &str, f: impl FnOnce(&Workspace) -> Option<T>) -> PyResult<T> {
        f(&self.inner.lock().unwrap()).ok_or_else(|| unset(name))
    }
}

#[pymethods]
impl PyWorkspace {
    #[new]
    #[pyo3(signature = (repo, base_ref, patch, *, workdir, keep))]
    fn new(
        repo: PathBuf,
        base_ref: String,
        patch: PathBuf,
        workdir: Option<PathBuf>,
        keep: bool,
    ) -> Self {
        Self {
            inner: Mutex::new(Workspace::new(repo, base_ref, patch, workdir, keep)),
        }
    }

    fn __enter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Self>> {
        let py = slf.py();
        let ws = slf.get();
        let result = py.detach(|| ws.inner.lock().unwrap().enter());
        result.map_err(workspace_err)?;
        Ok(slf)
    }

    #[pyo3(signature = (*_args))]
    fn __exit__(&self, py: Python<'_>, _args: &Bound<'_, PyTuple>) {
        py.detach(|| self.inner.lock().unwrap().exit());
    }

    #[getter]
    fn repo(&self) -> PathBuf {
        self.inner.lock().unwrap().repo.clone()
    }

    #[getter]
    fn base_ref(&self) -> String {
        self.inner.lock().unwrap().base_ref.clone()
    }

    #[getter]
    fn patch(&self) -> PathBuf {
        self.inner.lock().unwrap().patch.clone()
    }

    #[getter]
    fn keep(&self) -> bool {
        self.inner.lock().unwrap().keep
    }

    #[getter]
    fn workdir(&self) -> PyResult<PathBuf> {
        self.get("workdir", |w| w.workdir.clone())
    }

    #[getter]
    fn base(&self) -> PyResult<PathBuf> {
        self.get("base", |w| w.base.clone())
    }

    #[getter]
    fn patched(&self) -> PyResult<PathBuf> {
        self.get("patched", |w| w.patched.clone())
    }

    #[getter]
    fn base_sha(&self) -> PyResult<String> {
        self.get("base_sha", |w| w.base_sha.clone())
    }

    /// `git diff --cached` of patched/, decoded with surrogateescape.
    #[getter]
    fn applied_patch<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let raw = self.get("applied_patch", |w| w.applied_patch.clone())?;
        Ok(pytext_into_py(py, &PyText::from_surrogateescape_bytes(&raw))?.into_any())
    }

    #[getter]
    fn offset_paths<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, self.get("offset_paths", |w| w.offset_paths.clone())?)
    }
}

#[pyfunction]
fn repo_root(path: PathBuf) -> PyResult<PathBuf> {
    workspace::repo_root(&path).map_err(workspace_err)
}

/// Tracked changes only; untracked files don't affect the base ref.
#[pyfunction]
fn is_dirty(repo: PathBuf) -> PyResult<bool> {
    workspace::is_dirty(&repo).map_err(workspace_err)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "run_workspace")?;
    m.add("PatchApplyError", py.get_type::<PatchApplyError>())?;
    m.add("GitError", py.get_type::<GitError>())?;
    m.add("BadRefError", py.get_type::<BadRefError>())?;
    m.add_class::<PyWorkspace>()?;
    m.add_function(wrap_pyfunction!(repo_root, &m)?)?;
    m.add_function(wrap_pyfunction!(is_dirty, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
