//! lcov tracefile parser and dumper. Port of `sundgren/core/ingest/lcov.py`.

use std::fmt;

use crate::model::Snapshot;

/// A known record that doesn't parse. Displays as the prototype's ValueError message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LcovError {
    /// 1-based line number in the tracefile.
    pub lineno: usize,
    pub tag: String,
    /// The stripped line.
    pub line: String,
}

impl fmt::Display for LcovError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for LcovError {}

/// Parse an lcov tracefile. Paths stay raw; duplicate SF sections are merged.
pub fn parse(text: &str) -> Result<Snapshot, LcovError> {
    parse_logged(text, &mut |_| {})
}

/// `parse`, sending debug messages (ignored records, dropped FNDA) to `log`.
pub fn parse_logged(_text: &str, _log: &mut dyn FnMut(String)) -> Result<Snapshot, LcovError> {
    todo!()
}

/// Serialize a Snapshot back to lcov (summary records recomputed).
pub fn dump(_snapshot: &Snapshot) -> String {
    todo!()
}
