//! Join the Python frontend's IR for tests/fixtures/shed_py with its recorded lcov runs.

use std::path::PathBuf;

use sundgren_core::lcov;
use sundgren_core::model::Snapshot;
use sundgren_ir::{File, NodeId, sha256_hex};
use sundgren_join::{Granularity, MatchedBy, Measured, NodeStatus, Overlay, join, join_file};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ir(name: &str) -> File {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/shed_py/{name}.ir.json"));
    File::from_json(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn source(path: &str) -> Vec<u8> {
    std::fs::read(repo().join("tests/fixtures/shed_py").join(path)).unwrap()
}

fn run(scenario: &str, side: &str) -> Snapshot {
    let p = repo().join(format!(
        "tests/fixtures/shed_py_runs/{scenario}/{side}.lcov"
    ));
    lcov::parse(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn calc(runs: &[Snapshot]) -> (File, Overlay) {
    let ir = ir("calc");
    let sha = sha256_hex(&source("shed/calc.py"));
    let o = join_file(
        &ir,
        Measured {
            runs,
            source_sha256: &sha,
            granularity: Granularity::Line,
        },
    );
    (ir, o)
}

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn fixtures_match_the_source() {
    ir("calc").validate(&source("shed/calc.py")).unwrap();
    ir("__init__")
        .validate(&source("shed/__init__.py"))
        .unwrap();
}

#[test]
fn skip_divide_patched_side() {
    let (_, o) = calc(&[run("skip_divide", "patched")]);
    assert!(o.warnings.is_empty());
    let divide = &o.facts[&id("shed/calc.py::divide")];
    // The def line runs at import; the body (6-8) never runs.
    assert_eq!(divide.status, NodeStatus::Uncovered);
    assert_eq!(divide.executable, [6, 7, 8].into());
    assert!(divide.hit.is_empty());
    assert_eq!(divide.granularity, Granularity::Line);
    let f = divide.function.as_ref().unwrap();
    assert_eq!(
        (f.name.as_str(), f.hits, f.by),
        ("divide", Some(0), MatchedBy::Name)
    );
    // Branches on line 6 attach to divide, the innermost node, with taken '-'.
    let br: Vec<_> = divide
        .branches
        .iter()
        .map(|b| (b.line, b.branch.as_str(), b.taken))
        .collect();
    assert_eq!(
        br,
        [(6, "jump to line 7", None), (6, "jump to line 8", None)]
    );

    let add = &o.facts[&id("shed/calc.py::add")];
    assert_eq!((add.status, add.executable.len()), (NodeStatus::Covered, 1));
    let classify = &o.facts[&id("shed/calc.py::classify")];
    assert_eq!(classify.status, NodeStatus::Covered);
    assert_eq!(classify.branches.len(), 4);

    let module = &o.facts[&id("shed/calc.py")];
    assert_eq!(module.status, NodeStatus::Partial);
    assert_eq!(module.missed, [6, 7, 8].into());
    assert!(module.branches.is_empty());
    assert!(
        o.facts
            .values()
            .all(|f| f.shared.is_empty() && f.tests.is_none())
    );
}

#[test]
fn skip_divide_base_side_is_covered() {
    let (ir, o) = calc(&[run("skip_divide", "base")]);
    assert_eq!(o.facts.len(), ir.nodes.len());
    assert!(o.facts.values().all(|f| f.status == NodeStatus::Covered));
}

#[test]
fn repeats_that_disagree_are_flaky() {
    let (_, o) = calc(&[run("skip_divide", "base"), run("skip_divide", "patched")]);
    let divide = &o.facts[&id("shed/calc.py::divide")];
    assert_eq!(divide.status, NodeStatus::Flaky);
    assert_eq!(divide.flaky, [6, 7, 8].into());
    assert_eq!(
        o.facts[&id("shed/calc.py::add")].status,
        NodeStatus::Covered
    );
}

#[test]
fn source_mismatch_refuses() {
    let ir = ir("calc");
    let runs = [run("skip_divide", "base")];
    let o = join_file(
        &ir,
        Measured {
            runs: &runs,
            source_sha256: &sha256_hex(b"something else"),
            granularity: Granularity::Line,
        },
    );
    assert!(o.facts.is_empty());
    let w: Vec<_> = o.warnings.iter().map(|w| w.code.as_str()).collect();
    assert_eq!(w, ["ir_source_mismatch"]);
}

#[test]
fn empty_module_and_whole_package() {
    let (calc_ir, init_ir) = (ir("calc"), ir("__init__"));
    let runs = [run("skip_divide", "patched")];
    let (a, b) = (
        sha256_hex(&source("shed/calc.py")),
        sha256_hex(&source("shed/__init__.py")),
    );
    let m = |sha| Measured {
        runs: &runs,
        source_sha256: sha,
        granularity: Granularity::Line,
    };
    let o = join([(&calc_ir, m(&a)), (&init_ir, m(&b))]);
    assert_eq!(o.facts.len(), calc_ir.nodes.len() + 1);
    assert_eq!(
        o.facts[&id("shed/__init__.py")].status,
        NodeStatus::Unmeasured
    );
}

#[test]
fn file_missing_from_coverage_is_unmeasured() {
    let (_, o) = calc(&[Snapshot::default()]);
    assert!(
        o.facts
            .values()
            .all(|f| f.status == NodeStatus::Unmeasured && f.function.is_none())
    );
}

#[test]
fn nested_names_match_by_overlap_and_shared_lines_are_flagged() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/nested");
    let ir = File::from_json(&std::fs::read_to_string(dir.join("m.ir.json")).unwrap()).unwrap();
    let src = std::fs::read(dir.join("m.py")).unwrap();
    ir.validate(&src).unwrap();
    let runs = [lcov::parse(&std::fs::read_to_string(dir.join("m.lcov")).unwrap()).unwrap()];
    let o = join_file(
        &ir,
        Measured {
            runs: &runs,
            source_sha256: &sha256_hex(&src),
            granularity: Granularity::Line,
        },
    );
    // coverage.py writes `outer.inner`; the IR has `outer.<locals>.inner`.
    let inner = &o.facts[&id("m.py::outer.<locals>.inner")];
    let f = inner.function.as_ref().unwrap();
    assert_eq!((f.name.as_str(), f.by), ("outer.inner", MatchedBy::Overlap));
    let outer = &o.facts[&id("m.py::outer")];
    assert_eq!(outer.function.as_ref().unwrap().by, MatchedBy::Name);
    assert_eq!(outer.executable, [2, 3, 4, 5].into());
    assert!(outer.shared.is_empty());
    // Two lambdas on line 4: line coverage can't tell them apart.
    for l in [
        "m.py::outer.<locals>.<lambda>",
        "m.py::outer.<locals>.<lambda>#1",
    ] {
        let fact = &o.facts[&id(l)];
        assert_eq!(fact.shared, [4].into());
        assert!(fact.function.is_none());
    }
}
