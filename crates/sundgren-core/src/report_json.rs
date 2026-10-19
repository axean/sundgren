//! Delta -> sundgren.delta/1 JSON. Port of core/report/json_report.py.
//!
//! `Json` is the report as a value tree. Strings are `PyText`, because paths and messages may
//! hold lone surrogates. `dumps` writes exactly what `json.dumps(sort_keys=True, indent=2)` does.

use std::fmt::Write as _;

use crate::pytext::PyText;

pub const SCHEMA_ID: &str = "sundgren.delta/1";

pub const LINE_KEYS: [&str; 6] = [
    "lost",
    "gained",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
];
pub const BRANCH_KEYS: [&str; 4] = ["lost", "gained", "new", "deleted"];
pub const FUNCTION_KEYS: [&str; 7] = [
    "went_dark",
    "lit_up",
    "partial_loss",
    "partial_gain",
    "mixed",
    "added",
    "removed",
];

/// A JSON value as Python's `json` module sees it. Objects keep insertion order.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(PyText),
    Array(Vec<Json>),
    Object(Vec<(PyText, Json)>),
}

impl Json {
    pub fn object<K: Into<PyText>>(items: impl IntoIterator<Item = (K, Json)>) -> Self {
        Json::Object(items.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// Value under `key` in an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&PyText> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Python truthiness.
    pub fn is_truthy(&self) -> bool {
        match self {
            Json::Null => false,
            Json::Bool(b) => *b,
            Json::Int(i) => *i != 0,
            Json::Float(f) => *f != 0.0,
            Json::Str(s) => !s.is_empty(),
            Json::Array(a) => !a.is_empty(),
            Json::Object(o) => !o.is_empty(),
        }
    }

    /// `str(value)` in Python, for the text report.
    pub fn py_str(&self) -> PyText {
        match self {
            Json::Null => "None".into(),
            Json::Bool(true) => "True".into(),
            Json::Bool(false) => "False".into(),
            Json::Int(i) => i.to_string().into(),
            Json::Float(f) => float_repr(*f).into(),
            Json::Str(s) => s.clone(),
            other => {
                let mut out = String::new();
                write_value(&mut out, other, None, 0);
                out.into()
            }
        }
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Json::Str(s.into())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Self {
        Json::Str(s.into())
    }
}

impl From<PyText> for Json {
    fn from(s: PyText) -> Self {
        Json::Str(s)
    }
}

impl From<&PyText> for Json {
    fn from(s: &PyText) -> Self {
        Json::Str(s.clone())
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Json::Bool(b)
    }
}

impl From<i64> for Json {
    fn from(i: i64) -> Self {
        Json::Int(i.into())
    }
}

impl From<i128> for Json {
    fn from(i: i128) -> Self {
        Json::Int(i)
    }
}

impl From<f64> for Json {
    fn from(f: f64) -> Self {
        Json::Float(f)
    }
}

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(v: Option<T>) -> Self {
        v.map_or(Json::Null, Into::into)
    }
}

impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(v: Vec<T>) -> Self {
        Json::Array(v.into_iter().map(Into::into).collect())
    }
}

/// `json.dumps(report, sort_keys=True, indent=2) + "\n"`.
pub fn dumps(report: &Json) -> String {
    let mut out = String::new();
    write_value(&mut out, report, Some(2), 0);
    out.push('\n');
    out
}

/// `json.dumps(value, sort_keys=True, indent=indent)`, without the trailing newline.
pub fn to_json_string(value: &Json, indent: Option<usize>) -> String {
    let mut out = String::new();
    write_value(&mut out, value, indent, 0);
    out
}

fn newline(out: &mut String, indent: Option<usize>, level: usize) {
    if let Some(n) = indent {
        out.push('\n');
        out.extend(std::iter::repeat_n(' ', n * level));
    }
}

fn write_value(out: &mut String, value: &Json, indent: Option<usize>, level: usize) {
    let item_sep = if indent.is_some() { "," } else { ", " };
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Int(i) => write!(out, "{i}").expect("write to String"),
        Json::Float(f) => out.push_str(&float_json(*f)),
        Json::Str(s) => write_str(out, s),
        Json::Array(items) if items.is_empty() => out.push_str("[]"),
        Json::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(item_sep);
                }
                newline(out, indent, level + 1);
                write_value(out, item, indent, level + 1);
            }
            newline(out, indent, level);
            out.push(']');
        }
        Json::Object(items) if items.is_empty() => out.push_str("{}"),
        Json::Object(items) => {
            let mut sorted: Vec<&(PyText, Json)> = items.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            out.push('{');
            for (i, (k, v)) in sorted.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(item_sep);
                }
                newline(out, indent, level + 1);
                write_str(out, k);
                out.push_str(": ");
                write_value(out, v, indent, level + 1);
            }
            newline(out, indent, level);
            out.push('}');
        }
    }
}

/// A JSON string with `ensure_ascii`: everything outside ' '..='~' is escaped.
fn write_str(out: &mut String, s: &PyText) {
    out.push('"');
    for cp in s.code_points() {
        match cp {
            0x22 => out.push_str("\\\""),
            0x5C => out.push_str("\\\\"),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x08 => out.push_str("\\b"),
            0x0C => out.push_str("\\f"),
            0x20..=0x7E => out.push(char::from(cp as u8)),
            0x10000.. => {
                let v = cp - 0x10000;
                let (hi, lo) = (0xD800 | (v >> 10), 0xDC00 | (v & 0x3FF));
                write!(out, "\\u{hi:04x}\\u{lo:04x}").expect("write to String");
            }
            _ => write!(out, "\\u{cp:04x}").expect("write to String"),
        }
    }
    out.push('"');
}

fn float_json(f: f64) -> String {
    if f.is_nan() {
        "NaN".into()
    } else if f.is_infinite() {
        if f > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else {
        float_repr(f)
    }
}

/// Python's `repr(float)`: shortest round-trip digits, exponent outside 1e-4..1e16.
pub fn float_repr(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.into();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    // `{:e}` gives the shortest round-trip digits: "-1.2345e6".
    let sci = format!("{f:e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent digits");
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let n = digits.len() as i32;
    let body = if (-4..16).contains(&exp) {
        if exp < 0 {
            format!("0.{}{digits}", "0".repeat((-exp - 1) as usize))
        } else if exp + 1 >= n {
            format!("{digits}{}.0", "0".repeat((exp + 1 - n) as usize))
        } else {
            let (a, b) = digits.split_at((exp + 1) as usize);
            format!("{a}.{b}")
        }
    } else {
        let (a, b) = digits.split_at(1);
        let frac = if b.is_empty() {
            String::new()
        } else {
            format!(".{b}")
        };
        let esign = if exp < 0 { '-' } else { '+' };
        format!("{a}{frac}e{esign}{:02}", exp.abs())
    };
    format!("{sign}{body}")
}

/// Python's `round(x, 1)`.
pub fn round1(x: f64) -> f64 {
    format!("{x:.1}").parse().expect("formatted float")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_repr_matches_python() {
        let cases = [
            (100.0, "100.0"),
            (66.7, "66.7"),
            (0.1, "0.1"),
            (1e16, "1e+16"),
            (1.5e16, "1.5e+16"),
            (123456789012345.6, "123456789012345.6"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (-2.5e-7, "-2.5e-07"),
            (1e300, "1e+300"),
            (-0.0, "-0.0"),
        ];
        for (f, want) in cases {
            assert_eq!(float_repr(f), want, "{f}");
        }
    }

    #[test]
    fn round1_is_half_even_on_exact_ties() {
        assert_eq!(round1(0.25), 0.2);
        assert_eq!(round1(0.35), 0.3); // 0.34999...
        assert_eq!(round1(2.0 / 3.0 * 100.0), 66.7);
    }

    #[test]
    fn dumps_layout() {
        let v = Json::object([
            ("b", Json::Array(vec![1i64.into(), Json::Null])),
            ("a", Json::object::<&str>([])),
            ("c", Json::Array(vec![])),
            ("d", "é\u{1F600}\"\x7f".into()),
        ]);
        assert_eq!(
            dumps(&v),
            "{\n  \"a\": {},\n  \"b\": [\n    1,\n    null\n  ],\n  \"c\": [],\n  \
             \"d\": \"\\u00e9\\ud83d\\ude00\\\"\\u007f\"\n}\n"
        );
    }
}
