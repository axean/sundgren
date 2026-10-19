//! Just enough Python lexing to find a `def` header's end and its parameters.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Byte {
    Code,
    Str,
    Comment,
}

/// Classifies each byte of `src` as code, string literal or comment.
pub fn classify(src: &[u8]) -> Vec<Byte> {
    let mut mask = vec![Byte::Code; src.len()];
    let mut i = 0;
    while i < src.len() {
        match src[i] {
            b'#' => {
                while i < src.len() && src[i] != b'\n' && src[i] != b'\r' {
                    mask[i] = Byte::Comment;
                    i += 1;
                }
            }
            b'\\' => i += 2,
            q @ (b'\'' | b'"') => {
                let raw = prefix(src, i).contains(&b'r') || prefix(src, i).contains(&b'R');
                let triple = src.get(i + 1) == Some(&q) && src.get(i + 2) == Some(&q);
                let open = if triple { 3 } else { 1 };
                let start = i;
                i += open;
                loop {
                    match src.get(i) {
                        None => break,
                        Some(b'\\') if !raw => i += 2,
                        Some(&c) if c == q => {
                            if !triple {
                                i += 1;
                                break;
                            }
                            if src.get(i + 1) == Some(&q) && src.get(i + 2) == Some(&q) {
                                i += 3;
                                break;
                            }
                            i += 1;
                        }
                        Some(_) => i += 1,
                    }
                }
                let end = i.min(src.len());
                mask[start..end].iter_mut().for_each(|m| *m = Byte::Str);
            }
            _ => i += 1,
        }
    }
    mask
}

/// String prefix letters (r, b, f, u, t in any case) right before a quote.
fn prefix(src: &[u8], quote: usize) -> &[u8] {
    let mut s = quote;
    while s > 0 && b"rRbBfFuUtT".contains(&src[s - 1]) {
        s -= 1;
    }
    &src[s..quote]
}

/// Index just past the `:` that ends the `def` header starting at `start`.
pub fn header_end(src: &[u8], start: usize) -> Option<usize> {
    let mask = classify(&src[start..]);
    let mut depth = 0i32;
    let mut opened = false;
    for (j, &b) in src[start..].iter().enumerate() {
        if mask[j] != Byte::Code {
            continue;
        }
        match b {
            b'(' | b'[' | b'{' => {
                opened |= b == b'(' && depth == 0;
                depth += 1;
            }
            b')' | b']' | b'}' => depth -= 1,
            b':' if depth == 0 && opened => return Some(start + j + 1),
            _ => {}
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Positional,
    VarArgs,
    KwOnly,
    KwArgs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub kind: ParamKind,
}

/// Parameters of the header `src` (from `def` or `async` to its `:`).
pub fn params(header: &[u8]) -> Option<Vec<Param>> {
    let mask = classify(header);
    // Comments become spaces; strings stay (they're only in defaults and annotations).
    let clean: Vec<u8> = header
        .iter()
        .zip(&mask)
        .map(|(&b, &m)| if m == Byte::Comment { b' ' } else { b })
        .collect();
    let code = |i: usize| mask[i] == Byte::Code;
    let open = (0..clean.len()).find(|&i| code(i) && clean[i] == b'(')?;
    let mut depth = 0i32;
    let mut pieces = Vec::new();
    let mut from = open + 1;
    let mut close = None;
    for i in open..clean.len() {
        if !code(i) {
            continue;
        }
        match clean[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            b',' if depth == 1 => {
                pieces.push(&clean[from..i]);
                from = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(&clean[from..close?]);
    let mut out = Vec::new();
    let mut kw_only = false;
    for p in pieces {
        let p = std::str::from_utf8(p).ok()?.trim();
        if p.is_empty() || p == "/" {
            continue;
        }
        if p == "*" {
            kw_only = true;
            continue;
        }
        let (kind, rest) = if let Some(r) = p.strip_prefix("**") {
            (ParamKind::KwArgs, r)
        } else if let Some(r) = p.strip_prefix('*') {
            kw_only = true;
            (ParamKind::VarArgs, r)
        } else if kw_only {
            (ParamKind::KwOnly, p)
        } else {
            (ParamKind::Positional, p)
        };
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            return None;
        }
        out.push(Param { name, kind });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_header_colon() {
        let src = b"def f(a: dict[str, int] = {'x': 1}, b=\"):\") -> int:  # c\n    return a\n";
        let end = header_end(src, 0).unwrap();
        assert_eq!(&src[end - 7..end], b"-> int:");
    }

    #[test]
    fn parses_parameter_kinds() {
        let ps = params(b"def f(a, /, b: int = 1, *args, c, d=(1, 2), **kw):").unwrap();
        let got: Vec<_> = ps.iter().map(|p| (p.name.as_str(), p.kind)).collect();
        use ParamKind::*;
        assert_eq!(
            got,
            [
                ("a", Positional),
                ("b", Positional),
                ("args", VarArgs),
                ("c", KwOnly),
                ("d", KwOnly),
                ("kw", KwArgs)
            ]
        );
        let ps = params(b"async def g(\n    x,  # first\n    *, y,\n):").unwrap();
        let got: Vec<_> = ps.iter().map(|p| (p.name.as_str(), p.kind)).collect();
        assert_eq!(got, [("x", Positional), ("y", KwOnly)]);
        assert_eq!(params(b"def h():").unwrap(), []);
    }
}
