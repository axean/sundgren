//! Function spans: Symbolizer trait, lcov FN fallback. Port of core/symbols.py.
//!
//! The registry by extension stays a Python dict; callers pass the symbolizer for a path.

use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;
use std::path::Path;

use crate::model::{FileCoverage, LineNo};
use crate::pytext::PyText;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// Qualified, e.g. "Class.method".
    pub name: String,
    pub start: LineNo,
    pub end: LineNo,
    /// First body line; lines before it (signature) don't count.
    pub body_start: Option<LineNo>,
}

impl Span {
    pub fn body_lines(&self) -> RangeInclusive<LineNo> {
        self.body_start.unwrap_or(self.start)..=self.end
    }
}

/// Why a symbolizer gave no spans. `Syntax` falls back to lcov FN; `Other` propagates.
#[derive(Debug)]
pub enum SymbolizerError {
    Syntax,
    Other(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for SymbolizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => f.write_str("syntax error"),
            Self::Other(e) => e.fmt(f),
        }
    }
}

impl Error for SymbolizerError {}

pub trait Symbolizer {
    fn functions(&self, source: &str) -> Result<Vec<Span>, SymbolizerError>;
}

/// Spans from lcov FN records. Missing end lines run to the next function or last line.
///
/// The FN line is the signature, which tools report hit on import or entry, so the body starts
/// one line later. A one-line function has no separate body.
pub struct LcovFnSymbolizer;

impl LcovFnSymbolizer {
    pub fn spans(&self, fc: &FileCoverage) -> Vec<Span> {
        let mut funcs: Vec<_> = fc.functions.iter().collect();
        funcs.sort_by(|a, b| (a.start, &a.name).cmp(&(b.start, &b.name)));
        let last_line = fc.lines.keys().next_back().copied().unwrap_or(0);
        funcs
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let end = f.end.unwrap_or_else(|| {
                    match funcs[i + 1..].iter().find(|g| g.start > f.start) {
                        Some(g) => g.start - 1,
                        None => last_line.max(f.start),
                    }
                });
                let end = end.max(f.start);
                Span {
                    name: f.name.clone(),
                    start: f.start,
                    end,
                    body_start: (end > f.start).then_some(f.start + 1),
                }
            })
            .collect()
    }
}

/// Spans for one side of a file: the given symbolizer, then lcov FN, else None.
pub fn resolve_spans(
    path: &PyText,
    root: Option<&Path>,
    fc: Option<&FileCoverage>,
    symbolizer: Option<&dyn Symbolizer>,
) -> Result<Option<Vec<Span>>, SymbolizerError> {
    if let (Some(symbolizer), Some(root)) = (symbolizer, root)
        && let Some(source) = read_source(root, path)
    {
        match symbolizer.functions(&source) {
            Ok(mut spans) => {
                spans.sort_by(span_order);
                return Ok(Some(spans));
            }
            Err(SymbolizerError::Syntax) => {}
            Err(e) => return Err(e),
        }
    }
    if let Some(fc) = fc
        && !fc.functions.is_empty()
    {
        let mut spans = LcovFnSymbolizer.spans(fc);
        spans.sort_by(span_order);
        return Ok(Some(spans));
    }
    Ok(None)
}

/// `(root / path).read_text(encoding="utf-8")`: strict UTF-8, universal newlines.
fn read_source(root: &Path, path: &PyText) -> Option<String> {
    let bytes = std::fs::read(root.join(path.to_os_path()?)).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    if !text.contains('\r') {
        return Some(text);
    }
    Some(text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn span_order(a: &Span, b: &Span) -> std::cmp::Ordering {
    (a.start, a.end, &a.name).cmp(&(b.start, b.end, &b.name))
}

/// First span per qualified name (duplicates, e.g. redefinitions, keep the earliest), in order.
pub fn spans_by_name(spans: &[Span]) -> Vec<(&str, &Span)> {
    let mut seen = std::collections::HashSet::new();
    spans
        .iter()
        .filter(|s| seen.insert(s.name.as_str()))
        .map(|s| (s.name.as_str(), s))
        .collect()
}
