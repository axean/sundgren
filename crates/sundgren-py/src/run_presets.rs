//! Bindings for `sundgren._native.run_presets`.
//!
//! `_BUILTINS` stays a dict in `sundgren.run.presets`; every call reads it from there.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyList, PyString, PyTuple};
use sundgren_run::presets::{self, Preset, PresetError as RsPresetError, Table, TestsStyle, Value};

create_exception!(
    sundgren.run.presets,
    PresetError,
    PyValueError,
    "A presets file or table is malformed."
);

/// `Preset` as a frozen Python class with the dataclass's fields, equality and repr.
#[pyclass(frozen, name = "Preset", module = "sundgren.run.presets")]
pub struct PyPreset {
    pub inner: Preset,
}

#[pymethods]
impl PyPreset {
    #[new]
    #[pyo3(signature = (
        name, description, commands, env, lcov, junit, selector, tests_style,
        shell = false, hints = Vec::new(), source = "built-in".to_owned()
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        name: String,
        description: String,
        commands: Vec<String>,
        env: &Bound<'_, PyAny>,
        lcov: String,
        junit: Option<String>,
        selector: Option<String>,
        tests_style: &str,
        shell: bool,
        hints: Vec<String>,
        source: String,
    ) -> PyResult<Self> {
        let tests_style = TestsStyle::parse(tests_style).ok_or_else(|| {
            PyValueError::new_err(format!(
                "tests_style must be 'args' or 'filter', got {tests_style:?}"
            ))
        })?;
        let mut pairs = Vec::new();
        for item in env.call_method0("items")?.try_iter()? {
            pairs.push(item?.extract::<(String, String)>()?);
        }
        Ok(PyPreset {
            inner: Preset {
                name,
                description,
                commands,
                env: pairs,
                lcov,
                junit,
                selector,
                tests_style,
                shell,
                hints,
                source,
            },
        })
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }
    #[getter]
    fn description(&self) -> &str {
        &self.inner.description
    }
    #[getter]
    fn commands<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, &self.inner.commands)
    }
    #[getter]
    fn env<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.inner.env {
            d.set_item(k, v)?;
        }
        Ok(d)
    }
    #[getter]
    fn lcov(&self) -> &str {
        &self.inner.lcov
    }
    #[getter]
    fn junit(&self) -> Option<&str> {
        self.inner.junit.as_deref()
    }
    #[getter]
    fn selector(&self) -> Option<&str> {
        self.inner.selector.as_deref()
    }
    #[getter]
    fn tests_style(&self) -> &'static str {
        self.inner.tests_style.as_str()
    }
    #[getter]
    fn shell(&self) -> bool {
        self.inner.shell
    }
    #[getter]
    fn hints<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        PyTuple::new(py, &self.inner.hints)
    }
    #[getter]
    fn source(&self) -> &str {
        &self.inner.source
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let py = other.py();
        Ok(match other.cast::<PyPreset>() {
            Ok(o) => PyBool::new(py, self.inner == o.get().inner)
                .to_owned()
                .into_any()
                .unbind(),
            Err(_) => py.NotImplemented(),
        })
    }

    fn __hash__(&self) -> PyResult<isize> {
        // The dataclass hashes its field tuple, and `env` is a dict.
        Err(PyTypeError::new_err("unhashable type: 'dict'"))
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let mut parts = Vec::new();
        for field in FIELDS {
            let value = slf.getattr(field)?.repr()?;
            parts.push(format!("{field}={value}"));
        }
        Ok(format!("Preset({})", parts.join(", ")))
    }
}

const FIELDS: [&str; 11] = [
    "name",
    "description",
    "commands",
    "env",
    "lcov",
    "junit",
    "selector",
    "tests_style",
    "shell",
    "hints",
    "source",
];

pub fn preset_into_py(py: Python<'_>, preset: Preset) -> PyResult<Bound<'_, PyAny>> {
    Ok(Bound::new(py, PyPreset { inner: preset })?.into_any())
}

pub fn preset_from_py(obj: &Bound<'_, PyAny>) -> PyResult<Preset> {
    Ok(obj.cast::<PyPreset>()?.get().inner.clone())
}

/// Python error for a Rust `PresetError`.
pub fn preset_err(err: RsPresetError) -> PyErr {
    match err {
        RsPresetError::Invalid(msg) => PresetError::new_err(msg),
        RsPresetError::Host(e) => match e.downcast::<PyErr>() {
            Ok(e) => *e,
            Err(e) => match e.downcast::<std::io::Error>() {
                Ok(e) => (*e).into(),
                Err(e) => PyRuntimeError::new_err(e.to_string()),
            },
        },
    }
}

/// The live `_BUILTINS` dict of `sundgren.run.presets`.
fn builtins_dict(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    Ok(py
        .import("sundgren.run.presets")?
        .getattr("_BUILTINS")?
        .cast_into::<PyDict>()?)
}

/// `_BUILTINS` read at call time, as Rust presets.
pub fn read_builtins(py: Python<'_>) -> PyResult<BTreeMap<String, Preset>> {
    let mut out = BTreeMap::new();
    for (k, v) in builtins_dict(py)?.iter() {
        out.insert(k.extract()?, preset_from_py(&v)?);
    }
    Ok(out)
}

/// TOML decoding with Python's tomllib, so accepted input and errors are tomllib's.
pub struct TomllibLoader<'py>(pub Python<'py>);

impl presets::TomlLoader for TomllibLoader<'_> {
    fn load(&self, data: &[u8], file: &str) -> Result<Table, RsPresetError> {
        let py = self.0;
        let host = |e: PyErr| RsPresetError::Host(Box::new(e));
        let tomllib = py.import("tomllib").map_err(host)?;
        let decode_error = tomllib.getattr("TOMLDecodeError").map_err(host)?;
        let parsed = PyBytes::new(py, data)
            .call_method1("decode", ("utf-8",))
            .and_then(|text| tomllib.call_method1("loads", (text,)));
        let doc = match parsed {
            Ok(doc) => doc,
            Err(e)
                if e.is_instance_of::<pyo3::exceptions::PyUnicodeDecodeError>(py)
                    || e.is_instance(py, &decode_error) =>
            {
                return Err(RsPresetError::Invalid(format!("{file}: {}", e.value(py))));
            }
            Err(e) => return Err(host(e)),
        };
        table_from_py(&doc).map_err(host)
    }
}

fn table_from_py(obj: &Bound<'_, PyAny>) -> PyResult<Table> {
    let mut out = Table::new();
    for (k, v) in obj.cast::<PyDict>()?.iter() {
        out.push((k.extract()?, value_from_py(&v)?));
    }
    Ok(out)
}

fn value_from_py(v: &Bound<'_, PyAny>) -> PyResult<Value> {
    Ok(if let Ok(s) = v.cast::<PyString>() {
        Value::Str(s.to_str()?.to_owned())
    } else if let Ok(b) = v.cast::<PyBool>() {
        Value::Bool(b.is_true())
    } else if let Ok(l) = v.cast::<PyList>() {
        Value::Array(l.iter().map(|x| value_from_py(&x)).collect::<PyResult<_>>()?)
    } else if v.cast::<PyDict>().is_ok() {
        Value::Table(table_from_py(v)?)
    } else {
        Value::Other
    })
}

fn presets_into_py(py: Python<'_>, map: BTreeMap<String, Preset>) -> PyResult<Bound<'_, PyDict>> {
    let d = PyDict::new(py);
    for (k, v) in map {
        d.set_item(k, preset_into_py(py, v)?)?;
    }
    Ok(d)
}

/// Register presets from a presets.toml shipped by a lang pack.
#[pyfunction]
fn register_builtin_presets(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<()> {
    let file: String = source.str()?.extract()?;
    let data: Vec<u8> = source.call_method0("read_bytes")?.extract()?;
    let doc = presets::TomlLoader::load(&TomllibLoader(py), &data, &file).map_err(preset_err)?;
    let mut builtins = read_builtins(py)?;
    let added = presets::register_builtin(&mut builtins, &doc, &file).map_err(preset_err)?;
    let dict = builtins_dict(py)?;
    for p in added {
        dict.set_item(p.name.clone(), preset_into_py(py, p)?)?;
    }
    Ok(())
}

#[pyfunction]
fn builtin_presets(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let dict = builtins_dict(py)?;
    let mut items: Vec<(String, Bound<'_, PyAny>)> = dict
        .iter()
        .map(|(k, v)| Ok((k.extract()?, v)))
        .collect::<PyResult<_>>()?;
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let out = PyDict::new(py);
    for (k, v) in items {
        out.set_item(k, v)?;
    }
    Ok(out)
}

/// Presets from sundgren.toml or [tool.sundgren.presets.*] in pyproject.toml.
#[pyfunction]
fn user_presets(py: Python<'_>, repo: PathBuf) -> PyResult<Bound<'_, PyDict>> {
    let map = presets::user_presets(&repo, &TomllibLoader(py)).map_err(preset_err)?;
    presets_into_py(py, map)
}

/// Built-ins overridden by user presets of the same name.
#[pyfunction]
fn all_presets(py: Python<'_>, repo: PathBuf) -> PyResult<Bound<'_, PyDict>> {
    let builtins = read_builtins(py)?;
    let map = presets::all_presets(&builtins, &repo, &TomllibLoader(py)).map_err(preset_err)?;
    presets_into_py(py, map)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "run_presets")?;
    m.add_class::<PyPreset>()?;
    m.add("PresetError", py.get_type::<PresetError>())?;
    m.add_function(wrap_pyfunction!(register_builtin_presets, &m)?)?;
    m.add_function(wrap_pyfunction!(builtin_presets, &m)?)?;
    m.add_function(wrap_pyfunction!(user_presets, &m)?)?;
    m.add_function(wrap_pyfunction!(all_presets, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
