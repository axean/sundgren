//! Bindings for `sundgren._native.run_template`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use sundgren_run::template::{self, Command, PLACEHOLDERS, Values};

/// Read the placeholder values out of a Python mapping with `.get`, like the prototype.
pub fn values_from_py(values: &Bound<'_, PyAny>) -> PyResult<Values> {
    let mut out = Values::new();
    for name in PLACEHOLDERS {
        let v = values.call_method1("get", (name,))?;
        if !v.is_none() {
            out.insert(name.to_owned(), v.extract()?);
        }
    }
    Ok(out)
}

/// argv list or shell string as a Python object.
pub fn command_into_py<'py>(py: Python<'py>, command: Command) -> PyResult<Bound<'py, PyAny>> {
    Ok(match command {
        Command::Argv(argv) => argv.into_pyobject(py)?.into_any(),
        Command::Shell(s) => s.into_pyobject(py)?.into_any(),
    })
}

/// Python's `shutil.which`, looked up at call time.
pub fn py_which(py: Python<'_>, name: &str) -> PyResult<Option<String>> {
    py.import("shutil")?
        .getattr("which")?
        .call1((name,))?
        .extract()
}

#[pyfunction]
fn expand_value(template: &str, values: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(template::expand_value(template, &values_from_py(values)?))
}

#[pyfunction]
#[pyo3(signature = (command, values, tests, *, shell))]
fn expand_command<'py>(
    py: Python<'py>,
    command: &str,
    values: &Bound<'py, PyAny>,
    tests: Vec<String>,
    shell: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let cmd = template::expand_command(command, &values_from_py(values)?, &tests, shell)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    command_into_py(py, cmd)
}

#[pyfunction]
fn default_python(py: Python<'_>) -> PyResult<String> {
    let err = std::cell::RefCell::new(None);
    let found = template::default_python(|name| {
        py_which(py, name).unwrap_or_else(|e| {
            err.borrow_mut().get_or_insert(e);
            None
        })
    });
    match err.into_inner() {
        Some(e) => Err(e),
        None => Ok(found),
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "run_template")?;
    m.add("PLACEHOLDERS", PyTuple::new(parent.py(), PLACEHOLDERS)?)?;
    m.add_function(wrap_pyfunction!(expand_value, &m)?)?;
    m.add_function(wrap_pyfunction!(expand_command, &m)?)?;
    m.add_function(wrap_pyfunction!(default_python, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
