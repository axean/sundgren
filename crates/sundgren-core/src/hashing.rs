//! sha256 helpers. Port of `sundgren/core/hashing.py`.

use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Lowercase hex sha256.
pub fn sha256_bytes(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn sha256_file(path: &Path) -> io::Result<String> {
    Ok(sha256_bytes(&std::fs::read(path)?))
}

/// sha256 of a file, or None when the path is unknown or missing.
pub fn sha256_file_or_none(path: Option<&Path>) -> io::Result<Option<String>> {
    match path {
        Some(p) if p.is_file() => sha256_file(p).map(Some),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digest() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_is_none() {
        assert_eq!(sha256_file_or_none(None).unwrap(), None);
        assert_eq!(
            sha256_file_or_none(Some(Path::new("/nonexistent/x"))).unwrap(),
            None
        );
        assert_eq!(sha256_file_or_none(Some(Path::new("/"))).unwrap(), None);
        assert!(sha256_file(Path::new("/nonexistent/x")).is_err());
    }
}
