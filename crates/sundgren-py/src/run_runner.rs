//! Bindings for `sundgren._native.run_runner`.

use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyDict, PyType};
use sundgren_run::runner::{self, LOG_TAIL_LINES, RunArtifacts, RunError, TemplateRunner};

use crate::dataclass;
use crate::run_presets::{PyPreset, preset_into_py};
use crate::run_selectors::PySelection;

#[pyclass(
    module = "sundgren.run.runner",
    name = "RunArtifacts",
    frozen,
    eq,
    hash
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyRunArtifacts {
    pub inner: RunArtifacts,
}

#[pymethods]
impl PyRunArtifacts {
    #[new]
    #[pyo3(signature = (lcov, junit, returncode, log))]
    fn new(lcov: PathBuf, junit: Option<PathBuf>, returncode: i32, log: PathBuf) -> Self {
        RunArtifacts {
            lcov,
            junit,
            returncode,
            log,
        }
        .into()
    }

    #[getter]
    fn lcov(&self) -> PathBuf {
        self.inner.lcov.clone()
    }

    #[getter]
    fn junit(&self) -> Option<PathBuf> {
        self.inner.junit.clone()
    }

    #[getter]
    fn returncode(&self) -> i32 {
        self.inner.returncode
    }

    #[getter]
    fn log(&self) -> PathBuf {
        self.inner.log.clone()
    }
}
dataclass!(
    PyRunArtifacts,
    "RunArtifacts",
    ["lcov", "junit", "returncode", "log"]
);

impl From<RunArtifacts> for PyRunArtifacts {
    fn from(inner: RunArtifacts) -> Self {
        Self { inner }
    }
}

// Defined in Python so its constructor and str() match the prototype exactly.
const RUN_FAILURE: &std::ffi::CStr = c"
class RunFailure(Exception):
    \"\"\"lcov output missing after the commands ran.\"\"\"

    def __init__(self, message: str, log_tail: str) -> None:
        super().__init__(message)
        self.log_tail = log_tail

RunFailure.__module__ = 'sundgren.run.runner'
";

static RUN_FAILURE_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

fn run_failure_type(py: Python<'_>) -> PyResult<&Bound<'_, PyType>> {
    RUN_FAILURE_TYPE
        .get_or_try_init(py, || {
            let ns = PyDict::new(py);
            py.run(RUN_FAILURE, Some(&ns), None)?;
            Ok::<_, PyErr>(
                ns.get_item("RunFailure")?
                    .unwrap()
                    .cast_into::<PyType>()?
                    .unbind(),
            )
        })
        .map(|t| t.bind(py))
}

/// Python error for a `RunError`.
pub fn run_err(py: Python<'_>, err: RunError) -> PyErr {
    match err {
        RunError::Failure { message, log_tail } => match run_failure_type(py) {
            Ok(t) => match t.call1((message, log_tail)) {
                Ok(exc) => PyErr::from_value(exc),
                Err(e) => e,
            },
            Err(e) => e,
        },
        RunError::Value(msg) => PyValueError::new_err(msg),
        RunError::Io(e) => e.into(),
    }
}

#[pyclass(module = "sundgren.run.runner", name = "TemplateRunner", frozen)]
pub struct PyTemplateRunner {
    pub inner: TemplateRunner,
}

#[pymethods]
impl PyTemplateRunner {
    #[new]
    #[pyo3(signature = (preset, *, python, env_overrides = None))]
    fn new(
        preset: PyRef<'_, PyPreset>,
        python: String,
        env_overrides: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut env = Vec::new();
        if let Some(overrides) = env_overrides {
            for item in overrides.call_method0("items")?.try_iter()? {
                env.push(item?.extract::<(String, String)>()?);
            }
        }
        Ok(Self {
            inner: TemplateRunner::new(preset.inner.clone(), python, env),
        })
    }

    #[getter]
    fn preset<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        preset_into_py(py, self.inner.preset.clone())
    }

    #[getter]
    fn python(&self) -> &str {
        &self.inner.python
    }

    #[getter]
    fn env_overrides<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.inner.env_overrides {
            d.set_item(k, v)?;
        }
        Ok(d)
    }

    /// Commands with only {python} and {tests} expanded (deterministic, for meta).
    fn display_commands(&self, selection: PyRef<'_, PySelection>) -> PyResult<Vec<String>> {
        self.inner
            .display_commands(&selection.inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[pyo3(signature = (worktree, selection, out_dir, repeat = 1))]
    fn run(
        &self,
        py: Python<'_>,
        worktree: PathBuf,
        selection: PyRef<'_, PySelection>,
        out_dir: PathBuf,
        repeat: i64,
    ) -> PyResult<Vec<PyRunArtifacts>> {
        let selection = selection.inner.clone();
        let result = py.detach(|| self.inner.run(&worktree, &selection, &out_dir, repeat));
        match result {
            Ok(arts) => Ok(arts.into_iter().map(PyRunArtifacts::from).collect()),
            Err(e) => Err(run_err(py, e)),
        }
    }
}

#[pyfunction]
#[pyo3(signature = (commands, lcov, junit, *, shell))]
fn adhoc_preset<'py>(
    py: Python<'py>,
    commands: Vec<String>,
    lcov: String,
    junit: Option<String>,
    shell: bool,
) -> PyResult<Bound<'py, PyAny>> {
    preset_into_py(py, runner::adhoc_preset(commands, lcov, junit, shell))
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "run_runner")?;
    m.add("LOG_TAIL_LINES", LOG_TAIL_LINES)?;
    m.add_class::<PyRunArtifacts>()?;
    m.add_class::<PyTemplateRunner>()?;
    m.add("RunFailure", run_failure_type(py)?)?;
    m.add_function(wrap_pyfunction!(adhoc_preset, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
