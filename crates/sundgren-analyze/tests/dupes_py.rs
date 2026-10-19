//! End to end: Python frontend (L1) -> join with real coverage.py lcov -> duplicate functions.

use std::path::PathBuf;

use sundgren_analyze::{Confidence, DuplicateOptions, duplicate_functions};
use sundgren_core::lcov;
use sundgren_frontend_python::{Request, parse_files};
use sundgren_ir::{File, NodeId, sha256_hex};
use sundgren_join::{Granularity, Measured, NodeStatus, join};

const FILES: [&str; 3] = ["dupes/__init__.py", "dupes/geometry.py", "dupes/util.py"];

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dupes_py")
}

fn ids(v: &[NodeId]) -> Vec<&str> {
    v.iter().map(NodeId::as_str).collect()
}

#[test]
#[ignore = "needs L1 from the Python frontend (W2)"]
fn duplicates_in_a_real_package() {
    let root = fixture();
    let requests: Vec<Request> = FILES
        .iter()
        .map(|p| Request {
            file: root.join(p),
            path: (*p).into(),
        })
        .collect();
    let files: Vec<File> = parse_files("python3".as_ref(), &requests)
        .expect("python3 runs the frontend")
        .into_iter()
        .map(|r| r.expect("parses"))
        .collect();
    assert!(files.iter().all(File::has_l1));
    let snapshot =
        lcov::parse(&std::fs::read_to_string(root.join("coverage.lcov")).unwrap()).unwrap();
    let runs = [snapshot];
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
    assert!(overlay.warnings.is_empty(), "{:?}", overlay.warnings);

    let out = duplicate_functions(&files, &overlay, DuplicateOptions::default());
    assert_eq!(out.len(), 2, "{out:#?}");

    // clamp is copied verbatim into util.py, and again as `limit` with other local names.
    let clamp = &out[0];
    assert_eq!(
        ids(&clamp.nodes),
        [
            "dupes/geometry.py::clamp",
            "dupes/util.py::clamp",
            "dupes/util.py::limit"
        ]
    );
    assert!(clamp.message.contains("identical up to local names"));
    let status: Vec<_> = clamp.evidence.nodes.iter().map(|e| e.status).collect();
    assert_eq!(
        status,
        [
            Some(NodeStatus::Covered),
            Some(NodeStatus::Uncovered),
            Some(NodeStatus::Partial)
        ]
    );
    assert_eq!(clamp.confidence, Confidence::Medium);

    // shout and yell are identical, and no test reaches either.
    let shout = &out[1];
    assert_eq!(
        ids(&shout.nodes),
        ["dupes/util.py::shout", "dupes/util.py::yell"]
    );
    assert!(shout.message.contains("are identical."));
    assert!(shout.message.ends_with("No copy is covered by any test."));
    assert_eq!(shout.confidence, Confidence::Low);
}
