//! sundgren: coverage delta between a base tree and a patched tree.

pub mod cancel;
pub mod cli;
pub mod diff;
pub mod junit;
pub mod lcov;
pub mod linemap;
pub mod model;
pub mod normalize;
pub mod patch;
pub mod report;
pub mod rollup;
pub mod run;

use std::fmt;
use std::path::Path;

use sha2::{Digest, Sha256};

pub const EXIT_OK: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_PATCH: i32 = 3;

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

/// Lowercase hex sha256 of a file.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let digest = Sha256::digest(std::fs::read(path)?);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// An enum that reads and serializes as a fixed string per variant.
macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident = $s:literal),* $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name { $($var),* }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$var => $s),* }
            }

            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some(Self::$var),)* _ => None }
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }
    };
}
pub(crate) use str_enum;
