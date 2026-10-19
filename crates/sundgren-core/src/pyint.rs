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

/// Close to `str.isprintable()` for one char. Unassigned code points (Cn) count as printable.
fn is_py_printable(c: char) -> bool {
    let u = c as u32;
    let format = matches!(
        u,
        0xAD | 0x600..=0x605
            | 0x61C
            | 0x6DD
            | 0x70F
            | 0x890..=0x891
            | 0x8E2
            | 0x180E
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x2066..=0x206F
            | 0xFEFF
            | 0xFFF9..=0xFFFB
            | 0x110BD
            | 0x110CD
            | 0x13430..=0x1343F
            | 0x1BCA0..=0x1BCA3
            | 0x1D173..=0x1D17A
            | 0xE0001
            | 0xE0020..=0xE007F
    );
    let private = matches!(u, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD);
    let nonchar = matches!(u, 0xFDD0..=0xFDEF) || (u & 0xFFFE) == 0xFFFE;
    !(c.is_control() || (c.is_whitespace() && c != ' ') || format || private || nonchar)
}

/// `repr(s)`. Exact except for unassigned code points, which Python escapes.
pub fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_py_printable(c) => out.push(c),
            c if (c as u32) < 0x100 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    out.push(quote);
    out
}

/// `repr(list_of_str)`.
pub fn py_repr_list<S: AsRef<str>>(items: &[S]) -> String {
    let parts: Vec<String> = items.iter().map(|s| py_repr(s.as_ref())).collect();
    format!("[{}]", parts.join(", "))
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
    fn repr() {
        assert_eq!(py_repr("a"), "'a'");
        assert_eq!(py_repr("it's"), "\"it's\"");
        assert_eq!(py_repr("'\""), "'\\'\"'");
        assert_eq!(
            py_repr("a\\b\t\x00\x7f\u{85}\u{a0} é\u{2028}\u{feff}\u{e000}"),
            "'a\\\\b\\t\\x00\\x7f\\x85\\xa0 é\\u2028\\ufeff\\ue000'"
        );
        assert_eq!(py_repr("\u{10ffff}😀"), "'\\U0010ffff😀'");
        assert_eq!(py_repr_list(&["a", "b"]), "['a', 'b']");
    }

    #[test]
    fn strip() {
        assert_eq!(py_strip("\u{85} a b\u{2028}"), "a b");
        assert_eq!(py_strip("\u{feff}x"), "\u{feff}x");
    }
}
