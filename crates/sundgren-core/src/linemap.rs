//! Patch -> per-file line maps between the base and patched trees. Port of core/linemap.py.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use crate::patch::{FilePatch, LineNo, Patch};
use crate::pytext::PyText;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineMap {
    /// None for a new file.
    pub old_path: Option<PyText>,
    /// None for a deleted file.
    pub new_path: Option<PyText>,
    old_to_new: BTreeMap<LineNo, LineNo>,
    deleted: Vec<LineNo>,
    added: Vec<LineNo>,
}

impl LineMap {
    /// `old_to_new` holds explicit pairs for lines inside hunks; it may be empty.
    pub fn new(
        old_path: Option<PyText>,
        new_path: Option<PyText>,
        old_to_new: BTreeMap<LineNo, LineNo>,
        deleted_old: BTreeSet<LineNo>,
        added_new: BTreeSet<LineNo>,
    ) -> Self {
        Self {
            old_path,
            new_path,
            old_to_new,
            deleted: deleted_old.into_iter().collect(),
            added: added_new.into_iter().collect(),
        }
    }

    pub fn old_to_new(&self) -> &BTreeMap<LineNo, LineNo> {
        &self.old_to_new
    }

    /// Deleted base lines, sorted.
    pub fn deleted_old(&self) -> &[LineNo] {
        &self.deleted
    }

    /// Added patched lines, sorted.
    pub fn added_new(&self) -> &[LineNo] {
        &self.added
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

pub fn identity(path: PyText) -> LineMap {
    LineMap::new(
        Some(path.clone()),
        Some(path),
        BTreeMap::new(),
        BTreeSet::new(),
        BTreeSet::new(),
    )
}

/// LineMap for every file the patch touches, keyed by both its old and new path.
pub fn build_linemaps(patch: &Patch) -> BTreeMap<PyText, LineMap> {
    let maps: Vec<LineMap> = patch.files.iter().map(linemap).collect();
    let mut by_path = BTreeMap::new();
    for lm in &maps {
        if let Some(p) = &lm.old_path {
            by_path.insert(p.clone(), lm.clone());
        }
    }
    // New paths are unique within a diff, so on a clash (rename chains, swaps)
    // the map that produces the path wins and patched-side lookups stay exact.
    for lm in &maps {
        if let Some(p) = &lm.new_path {
            by_path.insert(p.clone(), lm.clone());
        }
    }
    by_path
}

fn linemap(fp: &FilePatch) -> LineMap {
    let mut old_to_new = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    let mut added = BTreeSet::new();
    let mut delta = 0;
    for hunk in &fp.hunks {
        // New positions follow from old ones and earlier hunks, as git apply
        // places them; new_start is only cross-checked by the parser.
        let mut old = hunk.old_start + LineNo::from(hunk.old_count == 0);
        let mut new = old + delta;
        for line in &hunk.lines {
            if line.starts_with(b"-") {
                deleted.insert(old);
                old += 1;
            } else if line.starts_with(b"+") {
                added.insert(new);
                new += 1;
            } else {
                old_to_new.insert(old, new);
                old += 1;
                new += 1;
            }
        }
        delta += hunk.new_count - hunk.old_count;
    }
    LineMap::new(
        fp.old_path.clone(),
        fp.new_path.clone(),
        old_to_new,
        deleted,
        added,
    )
}

/// The LineMap touching `path`, or the identity map.
pub fn linemap_for<'a>(linemaps: &'a BTreeMap<PyText, LineMap>, path: &PyText) -> Cow<'a, LineMap> {
    match linemaps.get(path) {
        Some(lm) => Cow::Borrowed(lm),
        None => Cow::Owned(identity(path.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::parse_patch;

    #[test]
    fn shifts_around_hunks() {
        let text = "--- a/f\n+++ b/f\n@@ -2 +2,2 @@\n-b\n+B\n+C\n@@ -5,0 +7 @@\n+x\n";
        let maps = build_linemaps(&parse_patch(&PyText::from(text)).unwrap());
        let lm = linemap_for(&maps, &PyText::from("f"));
        assert_eq!((1..=7).map(|l| lm.to_new(l)).collect::<Vec<_>>(), [
            Some(1),
            None,
            Some(4),
            Some(5),
            Some(6),
            Some(8),
            Some(9)
        ]);
        assert_eq!(lm.to_old(7), None);
        assert_eq!(lm.to_old(8), Some(6));
        assert!(linemap_for(&maps, &PyText::from("g")).is_identity());
    }
}
