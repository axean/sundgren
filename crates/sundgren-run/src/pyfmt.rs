//! Python's `repr()` of a str, for error messages that quote names with `{name!r}`.

/// `repr(s)` for a Python str.
///
/// Non-printable characters follow `str.isprintable` for controls, separators, format
/// characters and private use; unassigned code points are kept as they are.
pub fn repr_str(s: &str) -> String {
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
            c if !is_printable(c) => {
                let n = c as u32;
                if n < 0x100 {
                    out.push_str(&format!("\\x{n:02x}"));
                } else if n < 0x10000 {
                    out.push_str(&format!("\\u{n:04x}"));
                } else {
                    out.push_str(&format!("\\U{n:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn is_printable(c: char) -> bool {
    let n = c as u32;
    if c == ' ' {
        return true;
    }
    !(c.is_control()
        // Zs, Zl, Zp
        || matches!(n, 0xA0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
        // Cf
        || matches!(
            n,
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
        )
        // Co
        || matches!(n, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD))
}

#[cfg(test)]
mod tests {
    use super::repr_str;

    #[test]
    fn matches_python_repr() {
        assert_eq!(repr_str("pytest"), "'pytest'");
        assert_eq!(repr_str("it's"), "\"it's\"");
        assert_eq!(repr_str("a'\""), "'a\\'\"'");
        assert_eq!(repr_str("a\\b\n\x01\x7f"), "'a\\\\b\\n\\x01\\x7f'");
        assert_eq!(repr_str("é\u{a0}\u{200b}😀"), "'é\\xa0\\u200b😀'");
    }
}
