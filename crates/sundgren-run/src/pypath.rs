//! `str()` of a Python `pathlib.Path`, for messages that print paths.

use std::path::{Component, Path};

/// `str(Path(p))`: repeated separators and `.` components dropped, no trailing slash.
pub fn py_str(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut absolute = false;
    for c in path.components() {
        match c {
            Component::RootDir => absolute = true,
            Component::CurDir => {}
            other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    match (absolute, parts.is_empty()) {
        (true, _) => format!("/{}", parts.join("/")),
        (false, true) => ".".into(),
        (false, false) => parts.join("/"),
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
