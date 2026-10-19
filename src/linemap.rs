//! Patch -> per-file line maps between the base and patched trees.

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::model::LineNo;
use crate::patch::{FilePatch, Patch};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineMap {
    /// None for a new file.
    pub old_path: Option<String>,
    /// None for a deleted file.
    pub new_path: Option<String>,
    /// Explicit pairs for context lines inside hunks.
    pub old_to_new: BTreeMap<LineNo, LineNo>,
    /// Deleted base lines, sorted.
    pub deleted: Vec<LineNo>,
    /// Added patched lines, sorted.
    pub added: Vec<LineNo>,
}

/// Line maps keyed by both old and new path.
pub type LineMaps = BTreeMap<String, LineMap>;

impl LineMap {
    pub fn identity(path: &str) -> Self {
        LineMap {
            old_path: Some(path.to_string()),
            new_path: Some(path.to_string()),
            old_to_new: BTreeMap::new(),
            deleted: Vec::new(),
            added: Vec::new(),
        }
    }

    /// Patched line for a base line, or None if it was deleted.
    pub fn to_new(&self, old: LineNo) -> Option<LineNo> {
        if self.deleted.binary_search(&old).is_ok() {
            return None;
        }
        if let Some(&new) = self.old_to_new.get(&old) {
            return Some(new);
        }
        Some(shift(old, &self.deleted, &self.added))
    }

    /// Base line for a patched line, or None if it was added.
    pub fn to_old(&self, new: LineNo) -> Option<LineNo> {
        if self.added.binary_search(&new).is_ok() {
            return None;
        }
        Some(shift(new, &self.added, &self.deleted))
    }

    pub fn is_identity(&self) -> bool {
        self.deleted.is_empty() && self.added.is_empty() && self.old_path == self.new_path
    }
}

fn shift(line: LineNo, removed: &[LineNo], inserted: &[LineNo]) -> LineNo {
    // Unchanged lines keep their order: rank among unchanged lines on one side
    // equals rank on the other. Solve n - #inserted(<= n) == rank by fixed point.
    let rank = line - removed.partition_point(|&x| x < line) as LineNo;
    let mut shift = 0;
    loop {
        let n = inserted.partition_point(|&x| x <= rank + shift) as LineNo;
        if n == shift {
            return rank + shift;
        }
        shift = n;
    }
}

/// LineMap for every file the patch touches, keyed by both its old and new path. On a clash
/// (rename chains, swaps) the map whose new path is the key wins.
pub fn build_linemaps(patch: &Patch) -> LineMaps {
    let maps: Vec<LineMap> = patch.files.iter().map(linemap).collect();
    let mut by_path = LineMaps::new();
    for lm in &maps {
        if let Some(p) = &lm.old_path {
            by_path.insert(p.clone(), lm.clone());
        }
    }
    for lm in maps {
        if let Some(p) = &lm.new_path {
            by_path.insert(p.clone(), lm.clone());
        }
    }
    by_path
}

fn linemap(fp: &FilePatch) -> LineMap {
    let mut old_to_new = BTreeMap::new();
    let mut deleted = Vec::new();
    let mut added = Vec::new();
    let mut delta = 0;
    for hunk in &fp.hunks {
        // New positions follow from old ones and earlier hunks, as git apply
        // places them; new_start is only cross-checked by the parser.
        let mut old = hunk.old_start + LineNo::from(hunk.old_count == 0);
        let mut new = old + delta;
        for line in &hunk.lines {
            if line.starts_with('-') {
                deleted.push(old);
                old += 1;
            } else if line.starts_with('+') {
                added.push(new);
                new += 1;
            } else {
                old_to_new.insert(old, new);
                old += 1;
                new += 1;
            }
        }
        delta += hunk.new_count - hunk.old_count;
    }
    deleted.sort_unstable();
    deleted.dedup();
    added.sort_unstable();
    added.dedup();
    LineMap {
        old_path: fp.old_path.clone(),
        new_path: fp.new_path.clone(),
        old_to_new,
        deleted,
        added,
    }
}

/// The LineMap touching `path`, or the identity map.
pub fn linemap_for<'a>(linemaps: &'a LineMaps, path: &str) -> Cow<'a, LineMap> {
    match linemaps.get(path) {
        Some(lm) => Cow::Borrowed(lm),
        None => Cow::Owned(LineMap::identity(path)),
    }
}

#[cfg(test)]
mod tests;
