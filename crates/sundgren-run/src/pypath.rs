//! `str()` of a Python `pathlib.Path`, for messages that print paths.

use std::path::{Component, Path, PathBuf};

/// `Path(p)` normalization: repeated separators and `.` components dropped.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        if c != Component::CurDir {
            out.push(c);
        }
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

/// `str(Path(p))`, for messages.
pub fn py_str(path: &Path) -> String {
    normalize(path).to_string_lossy().into_owned()
}

/// `Path(p).absolute()`: joined to the cwd when relative, then normalized.
pub fn absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(normalize(path))
    } else {
        Ok(normalize(&std::env::current_dir()?.join(path)))
    }
}

#[cfg(test)]
mod tests {
    use super::py_str;
    use std::path::Path;

    #[test]
    fn like_pathlib() {
        for (raw, want) in [
            ("./pyproject.toml", "pyproject.toml"),
            (".", "."),
            ("", "."),
            ("a//b/./c/", "a/b/c"),
            ("/a/../b", "/a/../b"),
            ("/", "/"),
        ] {
            assert_eq!(py_str(Path::new(raw)), want, "{raw}");
        }
    }
}
