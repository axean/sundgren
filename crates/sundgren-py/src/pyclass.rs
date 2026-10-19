//! Shared helpers that make frozen pyclasses behave like the prototype's frozen dataclasses.
//!
//! A class gets `__dataclass_fields__` (so `dataclasses.asdict/astuple/fields/replace/
//! is_dataclass` work), the dataclass repr, `__reduce__` (copy, pickle) and `__match_args__`.
//! The constructor must take every field as a keyword, in field order.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

/// `dataclasses.make_dataclass(name, fields).__dataclass_fields__`, for a `#[classattr]`.
pub fn dataclass_fields(py: Python<'_>, name: &str, fields: &[&str]) -> PyResult<Py<PyDict>> {
    let dc = py.import("dataclasses")?;
    let cls = dc.call_method1("make_dataclass", (name, fields.to_vec()))?;
    Ok(cls
        .getattr("__dataclass_fields__")?
        .cast_into::<PyDict>()?
        .unbind())
}

/// Field names, in order, for `__match_args__`.
pub fn match_args<'py>(py: Python<'py>, fields: &[&str]) -> PyResult<Bound<'py, PyTuple>> {
    PyTuple::new(py, fields)
}

fn field_names<'py>(obj: &Bound<'py, PyAny>) -> PyResult<Vec<String>> {
    let fields = obj.get_type().getattr("__dataclass_fields__")?;
    fields.cast_into::<PyDict>()?.keys().extract()
}

/// The dataclass repr: `Name(a=1, b='x')`.
pub fn dataclass_repr(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let mut parts = Vec::new();
    for name in field_names(obj)? {
        let value = obj.getattr(name.as_str())?.repr()?;
        parts.push(format!("{name}={value}"));
    }
    let qualname = obj.get_type().qualname()?;
    Ok(format!("{qualname}({})", parts.join(", ")))
}

/// `(type(obj), (field values...))`, for `__reduce__`.
pub fn dataclass_reduce<'py>(obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyTuple>> {
    let py = obj.py();
    let mut values = Vec::new();
    for name in field_names(obj)? {
        values.push(obj.getattr(name.as_str())?);
    }
    PyTuple::new(
        py,
        [
            obj.get_type().into_any(),
            PyTuple::new(py, values)?.into_any(),
        ],
    )
}

/// Adds the dataclass methods to a pyclass, in a second `#[pymethods]` block:
/// `dataclass!(PyBranchKey, "BranchKey", ["line", "block", "branch"]);`
#[macro_export]
macro_rules! dataclass {
    ($ty:ty, $name:literal, [$($field:literal),* $(,)?]) => {
        #[::pyo3::pymethods]
        impl $ty {
            #[classattr]
            fn __dataclass_fields__(
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::types::PyDict>> {
                $crate::pyclass::dataclass_fields(py, $name, &[$($field),*])
            }

            #[classattr]
            fn __match_args__(
                py: ::pyo3::Python<'_>,
            ) -> ::pyo3::PyResult<::pyo3::Py<::pyo3::types::PyTuple>> {
                Ok($crate::pyclass::match_args(py, &[$($field),*])?.unbind())
            }

            fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<String> {
                $crate::pyclass::dataclass_repr(slf.as_any())
            }

            fn __reduce__<'py>(
                slf: &::pyo3::Bound<'py, Self>,
            ) -> ::pyo3::PyResult<::pyo3::Bound<'py, ::pyo3::types::PyTuple>> {
                $crate::pyclass::dataclass_reduce(slf.as_any())
            }
        }
    };
}
