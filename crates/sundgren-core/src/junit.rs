//! JUnit XML -> {test_id: outcome}. Port of `sundgren/core/junit.py`.

use std::collections::BTreeMap;
use std::fmt;

use crate::model::Outcome;

/// Malformed XML or not JUnit. Displays as the prototype's ValueError message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunitError(pub String);

impl fmt::Display for JunitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JunitError {}

/// Outcome per test id (`classname::name`, or `name` without a classname), sorted by id.
pub fn parse_junit(text: &str) -> Result<BTreeMap<String, Outcome>, JunitError> {
    parse_junit_logged(text, &mut |_| {})
}

/// `parse_junit`, sending debug messages (testcases without a name) to `log`.
pub fn parse_junit_logged(
    _text: &str,
    _log: &mut dyn FnMut(String),
) -> Result<BTreeMap<String, Outcome>, JunitError> {
    todo!()
}
