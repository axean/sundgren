//! End to end: duplicates in a real package get edits, and the patched package behaves the same.

use std::path::{Path, PathBuf};
use std::process::Command;

use sundgren_analyze::{DuplicateOptions, duplicate_functions};
use sundgren_core::lcov;
use sundgren_frontend_python::{Options, Request, parse_files_with};
use sundgren_ir::{File, apply_edits, sha256_hex};
use sundgren_join::{Granularity, Measured, join};
use sundgren_lang_python::add_duplicate_edits;

const FILES: [&str; 3] = ["dupes/__init__.py", "dupes/geometry.py", "dupes/util.py"];

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sundgren-analyze/tests/fixtures/dupes_py")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn patched_duplicates_behave_the_same() {
    let root = fixture();
    let requests: Vec<Request> = FILES
        .iter()
        .map(|p| Request {
            file: root.join(p),
            path: (*p).into(),
        })
        .collect();
    let files: Vec<File> = parse_files_with("python3".as_ref(), &requests, Options { l1: true })
        .unwrap()
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    let runs =
        [lcov::parse(&std::fs::read_to_string(root.join("coverage.lcov")).unwrap()).unwrap()];
    let shas: Vec<String> = FILES
        .iter()
        .map(|p| sha256_hex(&std::fs::read(root.join(p)).unwrap()))
        .collect();
    let overlay = join(files.iter().zip(&shas).map(|(f, sha)| {
        (
            f,
            Measured {
                runs: &runs,
                source_sha256: sha,
                granularity: Granularity::Line,
            },
        )
    }));
    let mut out = duplicate_functions(&files, &overlay, DuplicateOptions::default());
    add_duplicate_edits(&mut out, &files, &root);
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|s| s.edits.is_some()), "{out:#?}");

    let work = std::env::temp_dir().join(format!("sundgren-dupes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    copy_tree(&root, &work);
    // Every edit's range refers to the original file, so a file's edits apply in one go.
    let mut by_file: std::collections::BTreeMap<&str, Vec<_>> = Default::default();
    for fe in out.iter().flat_map(|s| s.edits.as_ref().unwrap()) {
        by_file
            .entry(&fe.path)
            .or_default()
            .extend(fe.edits.iter().cloned());
    }
    for (path, edits) in by_file {
        let path = work.join(path);
        let patched = apply_edits(&std::fs::read(&path).unwrap(), &edits).unwrap();
        std::fs::write(&path, patched).unwrap();
    }
    let util = std::fs::read_to_string(work.join("dupes/util.py")).unwrap();
    assert!(util.contains("from .geometry import clamp\n"), "{util}");
    assert!(
        util.contains("def limit(x, lo, hi):\n    from .geometry import clamp\n"),
        "{util}"
    );
    assert!(util.contains("yell = shout"), "{util}");

    // The patched package gives the same answers, keyword calls included.
    let check = r#"
from dupes import util, geometry
assert util.clamp(5, 0, 3) == 3 and util.clamp(-1, 0, 3) == 0
assert util.limit(x=-1, lo=0, hi=3) == 0 and util.limit(7, 0, 3) == 3
assert util.yell("a b") == util.shout("a b") == "A! B!"
assert geometry.clamp(value=2, low=0, high=3) == 2
"#;
    let status = Command::new("python3")
        .args(["-c", check])
        .current_dir(&work)
        .status()
        .unwrap();
    std::fs::remove_dir_all(&work).unwrap();
    assert!(status.success());
}
