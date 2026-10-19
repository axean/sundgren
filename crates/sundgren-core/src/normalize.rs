//! Path normalization: raw SF paths -> repo-relative posix paths. Port of
//! `sundgren/core/normalize.py`.

use crate::model::Snapshot;

/// `os.path.realpath(path)` (non-strict, POSIX): symlinks resolved, missing tails kept.
pub fn realpath(_path: &str) -> String {
    todo!()
}

/// Return (path, external). Strips the longest matching root; external if unresolvable.
pub fn normalize_path<S: AsRef<str>>(_raw: &str, _roots: &[S]) -> (String, bool) {
    todo!()
}

/// Re-key a Snapshot by normalized path, merging records that collide.
pub fn normalize_snapshot<S: AsRef<str>>(_snapshot: &Snapshot, _roots: &[S]) -> Snapshot {
    todo!()
}
