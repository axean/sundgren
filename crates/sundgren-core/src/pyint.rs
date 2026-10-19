//! Python's `int(str)` and `str.strip()` rules, for input the prototype read with them.

use std::fmt;

/// `int(s)` failed: not a base-10 integer, or outside i128.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyIntError(pub String);

impl fmt::Display for PyIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid literal for int() with base 10: {:?}", self.0)
    }
}

impl std::error::Error for PyIntError {}

/// `str.isspace()`: Unicode White_Space plus the ASCII separators U+001C..U+001F.
pub fn is_py_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `s.strip()`.
pub fn py_strip(s: &str) -> &str {
    s.trim_matches(is_py_space)
}

/// `int(s)`: surrounding whitespace, an optional sign, ASCII digits with single `_` between
/// them. Non-ASCII decimal digits are not accepted (Python takes them).
pub fn py_int(s: &str) -> Result<i128, PyIntError> {
    let err = || PyIntError(s.to_string());
    let t = py_strip(s);
    let (neg, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') {
        return Err(err());
    }
    let mut value: i128 = 0;
    let mut prev_underscore = false;
    for b in digits.bytes() {
        match b {
            b'_' if !prev_underscore => prev_underscore = true,
            b'0'..=b'9' => {
                prev_underscore = false;
                let d = i128::from(b - b'0');
                value = value
                    .checked_mul(10)
                    .and_then(|v| {
                        if neg {
                            v.checked_sub(d)
                        } else {
                            v.checked_add(d)
                        }
                    })
                    .ok_or_else(err)?;
            }
            _ => return Err(err()),
        }
    }
    Ok(value)
}

/// `int(s)` narrowed to i64 (line numbers).
pub fn py_int_i64(s: &str) -> Result<i64, PyIntError> {
    i64::try_from(py_int(s)?).map_err(|_| PyIntError(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_python_grammar() {
        assert_eq!(py_int("12"), Ok(12));
        assert_eq!(py_int(" -3\t"), Ok(-3));
        assert_eq!(py_int("+0"), Ok(0));
        assert_eq!(py_int("007"), Ok(7));
        assert_eq!(py_int("1_000"), Ok(1000));
        assert_eq!(py_int("\u{a0}5\u{1f}"), Ok(5));
        assert_eq!(py_int("18446744073709551615"), Ok(18446744073709551615));
        assert_eq!(py_int(&i128::MIN.to_string()), Ok(i128::MIN));
    }

    #[test]
    fn rejects_what_python_rejects() {
        for s in [
            "", " ", "-", "_1", "1_", "1__0", "1 2", "0x10", "1.0", "--1", "- 1",
        ] {
            assert!(py_int(s).is_err(), "{s:?}");
        }
        assert!(py_int(&format!("{}0", i128::MAX)).is_err());
        assert!(py_int_i64("9223372036854775808").is_err());
    }

    #[test]
    fn strip() {
        assert_eq!(py_strip("\u{85} a b\u{2028}"), "a b");
        assert_eq!(py_strip("\u{feff}x"), "\u{feff}x");
    }
}
