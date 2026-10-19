//! A Python `str` that may hold lone surrogates, as surrogateescape decoding produces.
//!
//! Stored as generalized UTF-8: what `s.encode("utf-8", "surrogatepass")` gives. Byte order is
//! code point order, so equality, ordering and hashing match Python's `str`.

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PyText(Vec<u8>);

impl PyText {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap generalized UTF-8, or `None` if `bytes` isn't well-formed.
    pub fn from_generalized_utf8(bytes: Vec<u8>) -> Option<Self> {
        let mut i = 0;
        while i < bytes.len() {
            i += decode_at(&bytes, i)?.1;
        }
        Some(Self(bytes))
    }

    /// Wrap bytes known to be generalized UTF-8, e.g. a slice of a `PyText` cut at an ASCII byte.
    pub fn from_bytes_unchecked(bytes: Vec<u8>) -> Self {
        debug_assert!(Self::from_generalized_utf8(bytes.clone()).is_some());
        Self(bytes)
    }

    /// `bytes.decode("utf-8", "surrogateescape")`.
    pub fn from_surrogateescape_bytes(bytes: &[u8]) -> Self {
        let mut out = Vec::with_capacity(bytes.len());
        let mut rest = bytes;
        loop {
            match std::str::from_utf8(rest) {
                Ok(s) => {
                    out.extend_from_slice(s.as_bytes());
                    return Self(out);
                }
                Err(e) => {
                    let (good, bad) = rest.split_at(e.valid_up_to());
                    out.extend_from_slice(good);
                    let n = e.error_len().unwrap_or(bad.len());
                    for &b in &bad[..n] {
                        push_code_point(&mut out, 0xDC00 + u32::from(b));
                    }
                    rest = &bad[n..];
                }
            }
        }
    }

    /// `s.encode("utf-8", "surrogateescape")`: U+DC80..U+DCFF become raw bytes. `None` for any
    /// other lone surrogate, where Python raises `UnicodeEncodeError`.
    pub fn encode_surrogateescape(&self) -> Option<Vec<u8>> {
        if self.as_str().is_some() {
            return Some(self.0.clone());
        }
        let mut out = Vec::with_capacity(self.0.len());
        let mut i = 0;
        while i < self.0.len() {
            let (cp, n) = decode_at(&self.0, i).expect("well-formed");
            match cp {
                0xDC80..=0xDCFF => out.push((cp - 0xDC00) as u8),
                0xD800..=0xDFFF => return None,
                _ => out.extend_from_slice(&self.0[i..i + n]),
            }
            i += n;
        }
        Some(out)
    }

    /// File system bytes, as `os.fsencode` gives on POSIX. Same as `encode_surrogateescape`.
    pub fn to_os_bytes(&self) -> Option<Vec<u8>> {
        self.encode_surrogateescape()
    }

    /// A path for the file system; `None` where Python's `os.fsencode` would raise.
    #[cfg(unix)]
    pub fn to_os_path(&self) -> Option<PathBuf> {
        use std::os::unix::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_vec(
            self.encode_surrogateescape()?,
        )))
    }

    #[cfg(not(unix))]
    pub fn to_os_path(&self) -> Option<PathBuf> {
        self.as_str().map(PathBuf::from)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// The text as `&str`, unless it holds surrogates.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }

    /// Surrogates become U+FFFD.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Code points, surrogates included.
    pub fn code_points(&self) -> impl Iterator<Item = u32> + '_ {
        let mut i = 0;
        std::iter::from_fn(move || {
            let (cp, n) = decode_at(&self.0, i)?;
            i += n;
            Some(cp)
        })
    }

    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.0.starts_with(prefix)
    }

    /// Split on an ASCII byte, like `str.split(sep)`.
    pub fn split_ascii(&self, sep: u8) -> impl Iterator<Item = PyText> + '_ {
        assert!(sep.is_ascii());
        self.0
            .split(move |&b| b == sep)
            .map(|piece| Self(piece.to_vec()))
    }

    pub fn push_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }

    pub fn push_text(&mut self, t: &PyText) {
        self.0.extend_from_slice(&t.0);
    }
}

/// Decode one code point at `i`, surrogates allowed; `None` at the end or on bad input.
fn decode_at(b: &[u8], i: usize) -> Option<(u32, usize)> {
    let b0 = *b.get(i)?;
    let cont = |k: usize| -> Option<u32> {
        let c = *b.get(i + k)?;
        (c & 0xC0 == 0x80).then_some(u32::from(c & 0x3F))
    };
    match b0 {
        0x00..=0x7F => Some((u32::from(b0), 1)),
        0xC2..=0xDF => Some(((u32::from(b0 & 0x1F) << 6) | cont(1)?, 2)),
        0xE0..=0xEF => {
            let cp = (u32::from(b0 & 0x0F) << 12) | (cont(1)? << 6) | cont(2)?;
            (cp >= 0x800).then_some((cp, 3))
        }
        0xF0..=0xF4 => {
            let cp = (u32::from(b0 & 0x07) << 18) | (cont(1)? << 12) | (cont(2)? << 6) | cont(3)?;
            (0x10000..=0x10FFFF).contains(&cp).then_some((cp, 4))
        }
        _ => None,
    }
}

fn push_code_point(out: &mut Vec<u8>, cp: u32) {
    match cp {
        0..=0x7F => out.push(cp as u8),
        0x80..=0x7FF => out.extend_from_slice(&[0xC0 | (cp >> 6) as u8, 0x80 | (cp & 0x3F) as u8]),
        0x800..=0xFFFF => out.extend_from_slice(&[
            0xE0 | (cp >> 12) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]),
        _ => out.extend_from_slice(&[
            0xF0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3F) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ]),
    }
}

impl From<String> for PyText {
    fn from(s: String) -> Self {
        Self(s.into_bytes())
    }
}

impl From<&str> for PyText {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl PartialEq<str> for PyText {
    fn eq(&self, other: &str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<&str> for PyText {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<String> for PyText {
    fn eq(&self, other: &String) -> bool {
        self.0 == other.as_bytes()
    }
}

/// Lossy: surrogates show as U+FFFD.
impl fmt::Display for PyText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

impl fmt::Debug for PyText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_str() {
            Some(s) => fmt::Debug::fmt(s, f),
            None => write!(f, "PyText({:?})", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrogateescape_round_trip() {
        let raw = b"caf\xc3\xa9/\xff\x80x";
        let t = PyText::from_surrogateescape_bytes(raw);
        assert!(t.as_str().is_none());
        let cps: Vec<u32> = t.code_points().collect();
        assert_eq!(cps, [99, 97, 102, 0xE9, 47, 0xDCFF, 0xDC80, 120]);
        assert_eq!(t.encode_surrogateescape().unwrap(), raw);
        assert!(PyText::from_generalized_utf8(t.as_bytes().to_vec()).is_some());
    }

    #[test]
    fn order_is_code_point_order() {
        let e = PyText::from("é");
        let s = PyText::from_surrogateescape_bytes(b"\x80");
        let p = PyText::from("\u{E000}");
        assert!(e < s && s < p);
    }

    #[test]
    fn other_surrogates_do_not_encode() {
        let t = PyText::from_generalized_utf8(vec![0xED, 0xA0, 0x80]).unwrap();
        assert_eq!(t.encode_surrogateescape(), None);
        assert!(PyText::from_generalized_utf8(vec![0xC0, 0x80]).is_none());
    }
}
