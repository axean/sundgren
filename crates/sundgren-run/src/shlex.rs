//! Python's `shlex.split`, `shlex.quote` and `shlex.join` (POSIX mode).

use std::fmt;

/// `shlex.split` failure; the message is Python's `ValueError` text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShlexError(pub &'static str);

impl fmt::Display for ShlexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ShlexError {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Space,
    Word,
    Quote(char),
    Escape,
}

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// `shlex.split(s)`: whitespace split, quotes and backslash escapes, no comments.
pub fn split(s: &str) -> Result<Vec<String>, ShlexError> {
    let mut out = Vec::new();
    let mut chars = s.chars();
    loop {
        let mut token = String::new();
        let mut quoted = false;
        let mut state = State::Space;
        let mut escaped_from = State::Word;
        let mut done = false;
        loop {
            let next = chars.next();
            match state {
                State::Space => match next {
                    None => {
                        done = true;
                        break;
                    }
                    Some(c) if is_space(c) => {}
                    Some('\\') => {
                        escaped_from = State::Word;
                        state = State::Escape;
                    }
                    Some(c @ ('\'' | '"')) => state = State::Quote(c),
                    Some(c) => {
                        token.push(c);
                        state = State::Word;
                    }
                },
                State::Quote(q) => {
                    quoted = true;
                    match next {
                        None => return Err(ShlexError("No closing quotation")),
                        Some(c) if c == q => state = State::Word,
                        Some('\\') if q == '"' => {
                            escaped_from = state;
                            state = State::Escape;
                        }
                        Some(c) => token.push(c),
                    }
                }
                State::Escape => match next {
                    None => return Err(ShlexError("No escaped character")),
                    Some(c) => {
                        // Inside double quotes only `\"` and `\\` are escapes.
                        if matches!(escaped_from, State::Quote(q) if c != '\\' && c != q) {
                            token.push('\\');
                        }
                        token.push(c);
                        state = escaped_from;
                    }
                },
                State::Word => match next {
                    None => {
                        done = true;
                        break;
                    }
                    Some(c) if is_space(c) => break,
                    Some(c @ ('\'' | '"')) => state = State::Quote(c),
                    Some('\\') => {
                        escaped_from = State::Word;
                        state = State::Escape;
                    }
                    Some(c) => token.push(c),
                },
            }
        }
        if quoted || !token.is_empty() {
            out.push(token);
        }
        if done {
            return Ok(out);
        }
    }
}

fn is_safe(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c)
}

/// `shlex.quote(s)`.
pub fn quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_owned();
    }
    if s.chars().all(is_safe) {
        return s.to_owned();
    }
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

/// `shlex.join(argv)`.
pub fn join<S: AsRef<str>>(argv: &[S]) -> String {
    argv.iter()
        .map(|a| quote(a.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Vec<String> {
        split(s).unwrap()
    }

    #[test]
    fn splits_like_python() {
        assert_eq!(ok("a  b\tc\n"), ["a", "b", "c"]);
        assert_eq!(
            ok(r#"echo "a {out}" '${HOME}'"#),
            ["echo", "a {out}", "${HOME}"]
        );
        assert_eq!(ok(r#"a"b c"d"#), ["ab cd"]);
        assert_eq!(ok("'' x"), ["", "x"]);
        assert_eq!(ok(r#""a\"b\\c\d""#), [r#"a"b\c\d"#]);
        assert_eq!(ok(r"'a\b'"), [r"a\b"]);
        assert_eq!(ok(r"a\ b \x"), ["a b", "x"]);
        assert_eq!(ok("a #b"), ["a", "#b"]);
        assert!(ok("  ").is_empty());
        assert_eq!(split("'a").unwrap_err().to_string(), "No closing quotation");
        assert_eq!(
            split("a\\").unwrap_err().to_string(),
            "No escaped character"
        );
    }

    #[test]
    fn quotes_like_python() {
        assert_eq!(quote(""), "''");
        assert_eq!(quote("a/b.py"), "a/b.py");
        assert_eq!(quote("a b"), "'a b'");
        assert_eq!(quote("it's"), "'it'\"'\"'s'");
        assert_eq!(quote("é"), "'é'");
        assert_eq!(join(&["a b", "c"]), "'a b' c");
    }
}
