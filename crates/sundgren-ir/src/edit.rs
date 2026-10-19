//! Text edits on byte spans.

use std::fmt;
use std::ops::Range;

/// Replace `range` (bytes of the original source) with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range<usize>,
    pub text: Vec<u8>,
}

/// Edits to one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEdit {
    /// Repo-relative, `/`-separated.
    pub path: String,
    pub edits: Vec<TextEdit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    OutOfRange(Range<usize>),
    Overlap(Range<usize>, Range<usize>),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::OutOfRange(r) => write!(f, "edit {r:?} is out of range"),
            EditError::Overlap(a, b) => write!(f, "edits {a:?} and {b:?} overlap"),
        }
    }
}

impl std::error::Error for EditError {}

/// Apply edits given in original-source offsets, in any order. Edits must not overlap; two
/// insertions at the same offset apply in the given order.
pub fn apply_edits(source: &[u8], edits: &[TextEdit]) -> Result<Vec<u8>, EditError> {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.range.start, e.range.end));
    let mut out = Vec::with_capacity(source.len());
    let mut at = 0;
    let mut prev = 0..0;
    for e in sorted {
        let r = &e.range;
        if r.start > r.end || r.end > source.len() {
            return Err(EditError::OutOfRange(r.clone()));
        }
        if r.start < at {
            return Err(EditError::Overlap(prev, r.clone()));
        }
        out.extend_from_slice(&source[at..r.start]);
        out.extend_from_slice(&e.text);
        at = r.end;
        prev = r.clone();
    }
    out.extend_from_slice(&source[at..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(range: Range<usize>, text: &str) -> TextEdit {
        TextEdit {
            range,
            text: text.into(),
        }
    }

    #[test]
    fn applies_in_original_offsets() {
        let got = apply_edits(
            b"abcdef",
            &[edit(4..6, "X"), edit(0..1, "YY"), edit(2..2, "_")],
        );
        assert_eq!(got.unwrap(), b"YYb_cdX");
    }

    #[test]
    fn rejects_overlap_and_out_of_range() {
        assert!(matches!(
            apply_edits(b"abc", &[edit(0..2, ""), edit(1..3, "")]),
            Err(EditError::Overlap(..))
        ));
        assert!(matches!(
            apply_edits(b"abc", &[edit(2..4, "")]),
            Err(EditError::OutOfRange(_))
        ));
    }
}
