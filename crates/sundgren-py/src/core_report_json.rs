//! Bindings for `sundgren._native.core_report_json`.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use sundgren_core::report_json::{self, Json};

use crate::core_diff::PyDelta;
use crate::core_model::PyReportWarning;
use crate::pytext::{pytext_from_py, pytext_into_py};

const MAX_DEPTH: usize = 500;

/// Plain JSON data (dict with str keys, list, tuple, str, int, float, bool, None) as `Json`.
/// `None` for anything else, including subclasses, which callers hand back to Python's `json`.
pub(crate) fn py_to_json(obj: &Bound<'_, PyAny>) -> PyResult<Option<Json>> {
    to_json(obj, 0)
}

fn to_json(obj: &Bound<'_, PyAny>, depth: usize) -> PyResult<Option<Json>> {
    if depth > MAX_DEPTH {
        return Ok(None);
    }
    Ok(Some(if obj.is_none() {
        Json::Null
    } else if obj.is_exact_instance_of::<PyBool>() {
        Json::Bool(obj.extract()?)
    } else if obj.is_exact_instance_of::<PyInt>() {
        match obj.extract::<i128>() {
            Ok(i) => Json::Int(i),
            Err(_) => return Ok(None),
        }
    } else if obj.is_exact_instance_of::<PyFloat>() {
        Json::Float(obj.extract()?)
    } else if obj.is_exact_instance_of::<PyString>() {
        Json::Str(pytext_from_py(obj)?)
    } else if obj.is_exact_instance_of::<PyList>() || obj.is_exact_instance_of::<PyTuple>() {
        let mut items = Vec::new();
        for item in obj.try_iter()? {
            match to_json(&item?, depth + 1)? {
                Some(v) => items.push(v),
                None => return Ok(None),
            }
        }
        Json::Array(items)
    } else if let Ok(d) = obj.cast_exact::<PyDict>() {
        let mut items = Vec::with_capacity(d.len());
        for (k, v) in d.iter() {
            if !k.is_exact_instance_of::<PyString>() {
                return Ok(None);
            }
            match to_json(&v, depth + 1)? {
                Some(v) => items.push((pytext_from_py(&k)?, v)),
                None => return Ok(None),
            }
        }
        Json::Object(items)
    } else {
        return Ok(None);
    }))
}

/// `Json` as plain Python data: dict, list, str, int, float, bool, None.
pub(crate) fn json_into_py<'py>(py: Python<'py>, value: &Json) -> PyResult<Bound<'py, PyAny>> {
    Ok(match value {
        Json::Null => py.None().into_bound(py),
        Json::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
        Json::Int(i) => i.into_pyobject(py)?.into_any(),
        Json::Float(f) => PyFloat::new(py, *f).into_any(),
        Json::Str(s) => pytext_into_py(py, s)?.into_any(),
        Json::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_into_py(py, item)?)?;
            }
            list.into_any()
        }
        Json::Object(items) => {
            let dict = PyDict::new(py);
            for (k, v) in items {
                dict.set_item(pytext_into_py(py, k)?, json_into_py(py, v)?)?;
            }
            dict.into_any()
        }
    })
}

/// The sundgren.delta/1 dict; by default only changes, `full` keeps everything.
#[pyfunction]
#[pyo3(signature = (delta, meta, *, full = false, counts = false, extra_warnings = None))]
fn build_report<'py>(
    py: Python<'py>,
    delta: &Bound<'py, PyAny>,
    meta: &Bound<'py, PyAny>,
    full: bool,
    counts: bool,
    extra_warnings: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let delta = delta.cast::<PyDelta>()?;
    let extra = match extra_warnings {
        Some(w) => w
            .try_iter()?
            .map(|w| Ok(w?.cast::<PyReportWarning>()?.get().inner.clone()))
            .collect::<PyResult<Vec<_>>>()?,
        None => Vec::new(),
    };
    let report = report_json::build_report(&delta.get().inner, Json::Null, full, counts, &extra);
    let out = json_into_py(py, &report)?;
    // `dict(meta)`: the caller's values as they are, not converted.
    let meta = py.import("builtins")?.getattr("dict")?.call1((meta,))?;
    out.set_item("meta", meta)?;
    Ok(out)
}

/// `json.dumps(report, sort_keys=True, indent=2)` plus a trailing newline.
#[pyfunction]
fn dumps(py: Python<'_>, report: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Some(value) = py_to_json(report)? {
        return Ok(report_json::dumps(&value));
    }
    let kwargs = PyDict::new(py);
    kwargs.set_item("sort_keys", true)?;
    kwargs.set_item("indent", 2)?;
    let text: String = py
        .import("json")?
        .call_method("dumps", (report,), Some(&kwargs))?
        .extract()?;
    Ok(text + "\n")
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "core_report_json")?;
    m.add("SCHEMA_ID", report_json::SCHEMA_ID)?;
    m.add("LINE_KEYS", PyTuple::new(m.py(), report_json::LINE_KEYS)?)?;
    m.add(
        "BRANCH_KEYS",
        PyTuple::new(m.py(), report_json::BRANCH_KEYS)?,
    )?;
    m.add(
        "FUNCTION_KEYS",
        PyTuple::new(m.py(), report_json::FUNCTION_KEYS)?,
    )?;
    m.add_function(wrap_pyfunction!(build_report, &m)?)?;
    m.add_function(wrap_pyfunction!(dumps, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
