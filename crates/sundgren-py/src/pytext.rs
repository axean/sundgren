//! `PyText` <-> Python `str`, lossless for lone surrogates (via "surrogatepass").

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use sundgren_core::pytext::PyText;

pub(crate) fn pytext_from_py(obj: &Bound<'_, PyAny>) -> PyResult<PyText> {
    let Ok(s) = obj.cast::<PyString>() else {
        let ty = obj.get_type().name()?;
        return Err(PyTypeError::new_err(format!("expected str, got {ty}")));
    };
    if let Ok(text) = s.to_str() {
        return Ok(PyText::from(text));
    }
    let bytes = s.call_method1("encode", ("utf-8", "surrogatepass"))?;
    let bytes = bytes.cast::<PyBytes>()?.as_bytes().to_vec();
    Ok(PyText::from_bytes_unchecked(bytes))
}

pub(crate) fn pytext_into_py<'py>(
    py: Python<'py>,
    text: &PyText,
) -> PyResult<Bound<'py, PyString>> {
    match text.as_str() {
        Some(s) => Ok(PyString::new(py, s)),
        None => PyString::from_encoded_object(
            PyBytes::new(py, text.as_bytes()).as_any(),
            Some(c"utf-8"),
            Some(c"surrogatepass"),
        ),
    }
}

pub(crate) fn opt_pytext_from_py(obj: &Bound<'_, PyAny>) -> PyResult<Option<PyText>> {
    if obj.is_none() {
        Ok(None)
    } else {
        pytext_from_py(obj).map(Some)
    }
}

pub(crate) fn opt_pytext_into_py<'py>(
    py: Python<'py>,
    text: Option<&PyText>,
) -> PyResult<Bound<'py, PyAny>> {
    match text {
        Some(t) => Ok(pytext_into_py(py, t)?.into_any()),
        None => Ok(py.None().into_bound(py)),
    }
}
