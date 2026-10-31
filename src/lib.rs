//! sundgren: git history as a graph, how much work branches share, and how it reads code.

pub mod cli;
pub mod contain;
pub mod git;
pub mod graph;
pub mod similarity;
pub mod syntax;
#[cfg(test)]
mod test_support;

use std::fmt;

pub const EXIT_OK: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_USAGE: i32 = 2;

/// A failure printed as `sundgren: {msg}` that exits with `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: i32,
    pub msg: String,
}

impl Error {
    pub fn usage(msg: impl Into<String>) -> Self {
        Error {
            code: EXIT_USAGE,
            msg: msg.into(),
        }
    }

    pub fn failure(msg: impl Into<String>) -> Self {
        Error {
            code: EXIT_FAILURE,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::failure(e.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
