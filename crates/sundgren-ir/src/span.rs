//! Positions, spans and the byte <-> line/column index.

use std::ops::{Range, RangeInclusive};

use serde::{Deserialize, Serialize};

/// A position in the original file bytes. `line` and `col` are 1-based; `col` counts bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pos {
    pub byte: usize,
    pub line: u32,
    pub col: u32,
}

/// A half-open byte range `[start, end)` with the line/column of both ends.
///
/// A non-empty span never ends at column 1, so `end.line` is the last line it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

impl Span {
    pub fn bytes(&self) -> Range<usize> {
        self.start.byte..self.end.byte
    }

    pub fn lines(&self) -> RangeInclusive<u32> {
        self.start.line..=self.end.line
    }

    pub fn contains(&self, other: &Span) -> bool {
        self.start.byte <= other.start.byte && other.end.byte <= self.end.byte
    }

    /// The spanned text. Panics if the span is out of range for `source`.
    pub fn text<'a>(&self, source: &'a [u8]) -> &'a [u8] {
        &source[self.bytes()]
    }
}

/// What ends a line. Languages differ: Python also breaks lines at a lone `\r`, Rust does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Newlines {
    /// `\n` only (`\r\n` ends in `\n`, so it counts once).
    Lf,
    /// `\n`, `\r\n` and a lone `\r`.
    Universal,
}

/// Maps byte offsets to [`Pos`] and back for one source.
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of the first byte of each line.
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(source: &[u8], newlines: Newlines) -> Self {
        let mut starts = vec![0];
        let mut i = 0;
        while i < source.len() {
            match source[i] {
                b'\n' => starts.push(i + 1),
                b'\r' if newlines == Newlines::Universal => {
                    if source.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    }
                    starts.push(i + 1);
                }
                _ => {}
            }
            i += 1;
        }
        Self {
            starts,
            len: source.len(),
        }
    }

    /// Number of lines. A trailing newline starts a last, empty line.
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// The position of `byte`, which may equal the source length. None past the end.
    pub fn pos(&self, byte: usize) -> Option<Pos> {
        if byte > self.len {
            return None;
        }
        let line = self.starts.partition_point(|&s| s <= byte) - 1;
        Some(Pos {
            byte,
            line: u32::try_from(line + 1).ok()?,
            col: u32::try_from(byte - self.starts[line] + 1).ok()?,
        })
    }

    pub fn span(&self, start: usize, end: usize) -> Option<Span> {
        if start > end {
            return None;
        }
        Some(Span {
            start: self.pos(start)?,
            end: self.pos(end)?,
        })
    }

    /// The byte offset of a 1-based line and byte column. None if it is not on that line
    /// (the line's end, before its newline, is still on it).
    pub fn byte(&self, line: u32, col: u32) -> Option<usize> {
        let l = usize::try_from(line).ok()?.checked_sub(1)?;
        let start = *self.starts.get(l)?;
        let next = self.starts.get(l + 1).copied().unwrap_or(self.len + 1);
        let byte = start + usize::try_from(col).ok()?.checked_sub(1)?;
        (byte < next).then_some(byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(byte: usize, line: u32, col: u32) -> Pos {
        Pos { byte, line, col }
    }

    #[test]
    fn lf_ignores_lone_cr() {
        let ix = LineIndex::new(b"a\rb\r\nc\n", Newlines::Lf);
        assert_eq!(ix.line_count(), 3);
        assert_eq!(ix.pos(2), Some(p(2, 1, 3)));
        assert_eq!(ix.pos(5), Some(p(5, 2, 1)));
        assert_eq!(ix.pos(7), Some(p(7, 3, 1)));
        assert_eq!(ix.pos(8), None);
    }

    #[test]
    fn universal_breaks_at_cr_crlf_and_lf() {
        let ix = LineIndex::new(b"a\rb\r\nc\nd", Newlines::Universal);
        assert_eq!(ix.line_count(), 4);
        assert_eq!(ix.pos(2), Some(p(2, 2, 1)));
        // The `\n` of `\r\n` is still on line 2.
        assert_eq!(ix.pos(4), Some(p(4, 2, 3)));
        assert_eq!(ix.pos(5), Some(p(5, 3, 1)));
        assert_eq!(ix.pos(7), Some(p(7, 4, 1)));
    }

    #[test]
    fn columns_count_bytes() {
        let src = "é = 1\nx".as_bytes();
        let ix = LineIndex::new(src, Newlines::Lf);
        assert_eq!(ix.pos(3), Some(p(3, 1, 4)));
        assert_eq!(ix.byte(1, 4), Some(3));
    }

    #[test]
    fn byte_roundtrips_and_rejects_off_line_columns() {
        let src = b"ab\ncd";
        let ix = LineIndex::new(src, Newlines::Lf);
        for b in 0..=src.len() {
            let pos = ix.pos(b).unwrap();
            assert_eq!(ix.byte(pos.line, pos.col), Some(b));
        }
        assert_eq!(ix.byte(1, 3), Some(2)); // the newline itself
        assert_eq!(ix.byte(1, 4), None);
        assert_eq!(ix.byte(2, 4), None);
        assert_eq!(ix.byte(0, 1), None);
        assert_eq!(ix.byte(3, 1), None);
    }

    #[test]
    fn empty_source() {
        let ix = LineIndex::new(b"", Newlines::Universal);
        assert_eq!(ix.line_count(), 1);
        assert_eq!(ix.span(0, 0).unwrap().lines(), 1..=1);
    }
}
