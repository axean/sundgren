use std::collections::BTreeMap;
use std::path::Path;

use regex::Regex;

use super::*;
use crate::diff::tests::{fc, snap};
use crate::diff::{DiffOptions, diff};
use crate::linemap::LineMaps;
use crate::model::{FileCoverage, FunctionRecord, Outcome, Outcomes, warning};

// --- a minimal JSON Schema check, for the keywords delta.schema.json uses ---

fn schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/delta.schema.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn is_type(v: &Value, t: &str) -> bool {
    match t {
        "null" => v.is_null(),
        "boolean" => v.is_boolean(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "string" => v.is_string(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        _ => panic!("type {t}"),
    }
}

fn valid(root: &Value, s: &Value, v: &Value) -> bool {
    let Some(s) = s.as_object() else { return true };
    let all = |key: &str| arr(&s.get(key).cloned().unwrap_or(Value::Null)).to_vec();
    let n = |key: &str| s.get(key).and_then(Value::as_f64);
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        let name = r.strip_prefix("#/$defs/").expect("local ref");
        if !valid(root, &root["$defs"][name], v) {
            return false;
        }
    }
    let checks = [
        match &s.get("type") {
            Some(Value::String(t)) => is_type(v, t),
            Some(Value::Array(ts)) => ts.iter().any(|t| is_type(v, t.as_str().unwrap())),
            _ => true,
        },
        s.get("enum").is_none_or(|e| arr(e).contains(v)),
        s.get("const").is_none_or(|c| c == v),
        n("minimum").is_none_or(|m| v.as_f64().is_none_or(|x| x >= m)),
        n("maximum").is_none_or(|m| v.as_f64().is_none_or(|x| x <= m)),
        n("minLength").is_none_or(|m| v.as_str().is_none_or(|x| x.chars().count() as f64 >= m)),
        s.get("pattern").is_none_or(|p| {
            v.as_str()
                .is_none_or(|x| Regex::new(p.as_str().unwrap()).unwrap().is_match(x))
        }),
        all("allOf").iter().all(|sub| valid(root, sub, v)),
        s.get("anyOf")
            .is_none_or(|_| all("anyOf").iter().any(|sub| valid(root, sub, v))),
        s.get("oneOf").is_none_or(|_| {
            all("oneOf")
                .iter()
                .filter(|sub| valid(root, sub, v))
                .count()
                == 1
        }),
        s.get("if")
            .is_none_or(|c| !valid(root, c, v) || valid(root, &s["then"], v)),
    ];
    if checks.contains(&false) {
        return false;
    }
    if let Some(items) = v.as_array() {
        let prefix = all("prefixItems");
        let unique = s.get("uniqueItems") != Some(&Value::Bool(true))
            || items
                .iter()
                .enumerate()
                .all(|(i, x)| !items[..i].contains(x));
        if !unique
            || n("minItems").is_some_and(|m| (items.len() as f64) < m)
            || items
                .iter()
                .zip(&prefix)
                .any(|(x, sub)| !valid(root, sub, x))
            || s.get("items").is_some_and(|sub| {
                items[prefix.len().min(items.len())..]
                    .iter()
                    .any(|x| !valid(root, sub, x))
            })
        {
            return false;
        }
    }
    if let Some(obj) = v.as_object() {
        let props = s.get("properties").and_then(Value::as_object);
        let required = all("required")
            .iter()
            .all(|k| obj.contains_key(k.as_str().unwrap()));
        let closed = s.get("additionalProperties") == Some(&Value::Bool(false));
        for (k, x) in obj {
            match props.and_then(|p| p.get(k)) {
                Some(sub) if !valid(root, sub, x) => return false,
                None if closed => return false,
                _ => {}
            }
        }
        return required;
    }
    true
}

fn check(report: &Value) -> bool {
    let s = schema();
    valid(&s, &s, report)
}

// --- the spec's canonical example: skip test_divide, divide goes dark ---

const CALC: &str = "shed/calc.py";
const EXECUTABLE: [i64; 12] = [1, 2, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14];
const ARCS: [(i64, i64); 6] = [(5, 6), (5, 7), (10, 11), (10, 12), (12, 13), (12, 14)];
const SHA: &str = "abababababababababababababababababababababababababababababababab";

fn calc(dark: bool, hits: i128) -> FileCoverage {
    let lines: Vec<_> = EXECUTABLE
        .iter()
        .map(|&l| {
            (
                l,
                if dark && (5..=7).contains(&l) {
                    0
                } else {
                    hits
                },
            )
        })
        .collect();
    let ids: Vec<String> = ARCS
        .iter()
        .map(|(_, d)| format!("jump to line {d}"))
        .collect();
    let br: Vec<_> = ARCS
        .iter()
        .zip(&ids)
        .map(|(&(src, _), id)| {
            (
                src,
                "0",
                id.as_str(),
                if dark && src == 5 { None } else { Some(1) },
            )
        })
        .collect();
    let mut f = fc(CALC, &lines, &br);
    f.functions = [("add", 1, 2), ("divide", 4, 7), ("classify", 9, 14)]
        .map(|(name, start, end)| FunctionRecord {
            name: name.into(),
            start,
            end: Some(end),
            hits: Some(1),
        })
        .to_vec();
    f
}

fn init() -> FileCoverage {
    fc("shed/__init__.py", &[], &[])
}

fn outcomes(divide: Outcome) -> Outcomes {
    BTreeMap::from([
        ("tests.test_calc::test_add".to_string(), Outcome::Passed),
        ("tests.test_calc::test_divide".to_string(), divide),
    ])
}

fn canonical() -> Delta {
    let (b, p) = (outcomes(Outcome::Passed), outcomes(Outcome::Skipped));
    let opts = DiffOptions {
        base_outcomes: Some(&b),
        patched_outcomes: Some(&p),
        ..Default::default()
    };
    let base = [snap(&[init(), calc(false, 1)])];
    let patched = [snap(&[init(), calc(true, 1)])];
    diff(&base, &patched, &LineMaps::new(), opts).unwrap()
}

fn plain(base: &[FileCoverage], patched: &[FileCoverage]) -> Delta {
    crate::diff::tests::d(base, patched, &LineMaps::new())
}

fn diff_meta() -> Value {
    json!({
        "sundgren_version": "0.1.0",
        "created_utc": "2026-10-18T07:00:00Z",
        "mode": "diff",
        "base_ref": null, "patch": null, "preset": null, "commands": null,
        "selection": null, "repeat": null,
        "inputs": {"base_lcov_sha256": [SHA], "patched_lcov_sha256": [SHA]},
    })
}

fn patch_meta() -> Value {
    json!({"path": "skip_divide.patch", "sha256": SHA, "files": ["tests/test_calc.py"]})
}

fn run_meta() -> Value {
    with(
        diff_meta(),
        json!({
            "mode": "run",
            "base_ref": "a1b2c3d",
            "patch": patch_meta(),
            "preset": "pytest",
            "commands": ["python -m coverage run --branch -m pytest tests/test_calc.py"],
            "selection": {"tests": ["tests/test_calc.py"], "run_all": false, "reason": "touched"},
            "repeat": 1,
        }),
    )
}

fn with(mut meta: Value, over: Value) -> Value {
    meta.as_object_mut()
        .unwrap()
        .extend(over.as_object().unwrap().clone());
    meta
}

fn report(delta: &Delta, meta: Value, full: bool, counts: bool) -> Value {
    let r = build_report(delta, meta, full, counts, &[]);
    assert!(check(&r), "{}", dumps(&r));
    r
}

#[test]
fn meta_variants_validate() {
    for meta in [
        diff_meta(),
        with(diff_meta(), json!({"patch": patch_meta()})),
        run_meta(),
    ] {
        report(&canonical(), meta, false, false);
    }
    let mut no_inputs = diff_meta();
    no_inputs.as_object_mut().unwrap().remove("inputs");
    for meta in [
        with(diff_meta(), json!({"preset": "pytest"})),
        with(diff_meta(), json!({"repeat": 1})),
        with(run_meta(), json!({"preset": null})),
        with(run_meta(), json!({"patch": null})),
        with(diff_meta(), json!({"mode": "other"})),
        with(diff_meta(), json!({"created_utc": "yesterday"})),
        with(diff_meta(), json!({"extra": 1})),
        no_inputs,
    ] {
        assert!(
            !check(&build_report(&canonical(), meta.clone(), false, false, &[])),
            "{meta}"
        );
    }
}

#[test]
fn schema_rejects_unknown_categories_and_keys() {
    let good = report(&canonical(), diff_meta(), false, false);
    let mut bad = good.clone();
    bad["files"][0]["lines"][0]["category"] = json!("lost_somehow");
    assert!(!check(&bad));
    let mut bad = good.clone();
    bad["summary"]["lines"]["extra"] = json!(0);
    assert!(!check(&bad));
    let mut bad = good;
    bad["calls"] = json!([]);
    assert!(!check(&bad));
}

#[test]
fn canonical_example() {
    let r = report(&canonical(), run_meta(), false, false);
    assert_eq!(r["schema"], "sundgren.delta/1");
    assert_eq!(r["calls"], Value::Null);
    assert_eq!(r["meta"], run_meta());
    assert_eq!(
        r["tests"],
        json!([{"id": "tests.test_calc::test_divide", "base": "passed", "patched": "skipped"}])
    );
    assert_eq!(
        r["summary"],
        json!({
            "lines": {"lost": 3, "gained": 0, "new_hit": 0, "new_miss": 0, "deleted_hit": 0,
                      "deleted_miss": 0, "flaky": 0, "unmeasured": 0},
            "branches": {"lost": 2, "gained": 0, "new": 0, "deleted": 0},
            "functions": {"went_dark": 1, "lit_up": 0, "partial_loss": 0, "partial_gain": 0,
                          "mixed": 0, "added": 0, "removed": 0},
            "coverage": {"base_pct": 100.0, "patched_pct": 75.0},
            "external_files": 0,
        })
    );
    let lines: Vec<Value> = [5, 6, 7]
        .map(|l| json!({"line": l, "base_line": l, "category": "lost", "hits": [1, 0]}))
        .to_vec();
    let branches: Vec<Value> = [6, 7]
        .map(|d| {
            json!({"line": 5, "base_line": 5, "block": "0", "branch": format!("jump to line {d}"),
                             "category": "branch_lost", "taken": [1, null]})
        })
        .to_vec();
    assert_eq!(
        r["files"],
        json!([{
            "path": CALC, "base_path": CALC,
            "sha256": {"base": null, "patched": null},
            "source_changed": false, "inferred_side": null,
            "lines": lines, "branches": branches,
            "functions": [{"name": "divide", "start": 4, "end": 7, "status": "went_dark",
                           "lines": {"lost": 3, "gained": 0, "executable": 3}}],
            "flaky": [], "count_changed": [],
        }])
    );
    assert_eq!(r["warnings"], json!([]));
}

#[test]
fn tree_shape() {
    let tree = &report(&canonical(), diff_meta(), false, false)["tree"];
    assert_eq!(
        (&tree["name"], &tree["path"], &tree["kind"]),
        (&json!(""), &json!(""), &json!("dir"))
    );
    assert_eq!(tree["stats"]["lost"], 3);
    assert_eq!(
        (
            &tree["stats"]["executable_base"],
            &tree["stats"]["hit_patched"]
        ),
        (&json!(12), &json!(9))
    );
    let shed = &tree["children"][0];
    assert_eq!(arr(&tree["children"]).len(), 1);
    assert_eq!(
        (&shed["path"], &shed["kind"]),
        (&json!("shed"), &json!("dir"))
    );
    let kids: Vec<_> = arr(&shed["children"])
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap(),
                c["path"].as_str().unwrap(),
                c["kind"].as_str().unwrap(),
                c.get("children").is_none(),
            )
        })
        .collect();
    assert_eq!(
        kids,
        [
            ("__init__.py", "shed/__init__.py", "file", true),
            ("calc.py", CALC, "file", true)
        ]
    );
    assert_eq!(shed["children"][1]["stats"].as_object().unwrap().len(), 10);
}

#[test]
fn full_keeps_everything() {
    let r = report(&canonical(), diff_meta(), true, false);
    let paths: Vec<_> = arr(&r["files"])
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["shed/__init__.py", CALC]);
    let calc = &r["files"][1];
    assert_eq!(arr(&calc["lines"]).len(), EXECUTABLE.len());
    assert_eq!(arr(&calc["branches"]).len(), ARCS.len());
    let statuses: Vec<_> = arr(&calc["functions"])
        .iter()
        .map(|f| f["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["unchanged", "went_dark", "unchanged"]);
    assert_eq!(arr(&r["tests"]).len(), 2);
    // Filtering never changes the summary or the tree.
    let default = report(&canonical(), diff_meta(), false, false);
    assert_eq!(
        (&r["summary"], &r["tree"]),
        (&default["summary"], &default["tree"])
    );
}

#[test]
fn unchanged_run_lists_nothing() {
    let d = plain(&[init(), calc(false, 1)], &[init(), calc(false, 1)]);
    let r = report(&d, diff_meta(), false, false);
    assert_eq!((&r["files"], &r["tests"]), (&json!([]), &json!([])));
    let cov = &r["summary"]["coverage"];
    assert_eq!(cov["base_pct"], cov["patched_pct"]);
}

#[test]
fn counts_fills_count_changed() {
    let d = plain(&[calc(false, 1)], &[calc(false, 2)]);
    assert_eq!(report(&d, diff_meta(), false, false)["files"], json!([]));
    let r = report(&d, diff_meta(), false, true);
    let want: Vec<Value> = EXECUTABLE
        .iter()
        .map(|l| json!({"line": l, "base_line": l, "hits": [1, 2]}))
        .collect();
    assert_eq!(r["files"][0]["count_changed"], json!(want));
    assert_eq!(r["files"][0]["lines"], json!([]));
    let r = report(&d, diff_meta(), true, false);
    assert_eq!(r["files"][0]["count_changed"], json!([]));
}

#[test]
fn coverage_pct_rounding_and_null() {
    let d = plain(&[fc("a.py", &[(1, 1), (2, 1), (3, 0)], &[])], &[init()]);
    let cov = &report(&d, diff_meta(), false, false)["summary"]["coverage"];
    assert_eq!(*cov, json!({"base_pct": 66.7, "patched_pct": null}));
    assert_eq!(round1(0.25), 0.2);
    assert_eq!(round1(0.35), 0.3); // 0.34999...
}

#[test]
fn warnings_merged_sorted_deduplicated() {
    let d = plain(&[], &[fc("x.py", &[(1, 1)], &[])]);
    let extra = [
        warning("patch", "z"),
        warning("dirty_tree", "uncommitted"),
        warning("patch", "z"),
    ];
    let r = build_report(&d, diff_meta(), false, false, &extra);
    let codes: Vec<_> = arr(&r["warnings"])
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["dirty_tree", "patch", "unmeasured"]);
}

#[test]
fn dumps_format() {
    let r = report(&canonical(), run_meta(), false, false);
    let text = dumps(&r);
    assert!(text.ends_with("}\n") && !text.ends_with("\n\n"));
    assert!(text.starts_with("{\n  \"calls\": null,\n  \"files\": [\n"));
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), r);
}

#[test]
fn identical_inputs_in_different_dirs_dump_identically() {
    let only = fc("shed/only_here.py", &[(1, 1)], &[]);
    let mut outs = Vec::new();
    let tmp = tempfile::tempdir().unwrap();
    for name in ["one", "two"] {
        let root = tmp.path().join(name);
        for (side, text) in [("base", "x = 1\n"), ("patched", "x = 2\n")] {
            for rel in [CALC, "shed/only_here.py"] {
                let p = root.join(side).join(rel);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(p, text).unwrap();
            }
        }
        let (b, p) = (outcomes(Outcome::Passed), outcomes(Outcome::Skipped));
        let (br, pr) = (root.join("base"), root.join("patched"));
        let opts = DiffOptions {
            base_root: Some(&br),
            patched_root: Some(&pr),
            base_outcomes: Some(&b),
            patched_outcomes: Some(&p),
        };
        let base = [snap(&[calc(false, 1), only.clone()])];
        let d = diff(&base, &[snap(&[calc(true, 1)])], &LineMaps::new(), opts).unwrap();
        outs.push(dumps(&report(&d, run_meta(), true, false)));
    }
    assert_eq!(outs[0], outs[1]);
    assert!(!outs[0].contains(tmp.path().to_str().unwrap()));
    assert!(outs[0].contains("\"inferred_side\": \"patched\""));
}

// --- text report ---

const HINTS: &[&str] = &["Set [run] source = <your package>."];

fn hints() -> Vec<String> {
    HINTS.iter().map(|h| h.to_string()).collect()
}

fn skip_divide(full: bool, extra: &[Warning]) -> Value {
    let funcs = [("add", 1, 2), ("divide", 4, 7)]
        .map(|(name, start, end)| FunctionRecord {
            name: name.into(),
            start,
            end: Some(end),
            hits: Some(1),
        })
        .to_vec();
    let mut base = fc(CALC, &[(1, 1), (2, 1), (4, 1), (5, 1), (6, 1), (7, 1)], &[]);
    let mut patched = fc(CALC, &[(1, 1), (2, 1), (4, 1), (5, 0), (6, 0), (7, 0)], &[]);
    base.functions = funcs.clone();
    patched.functions = funcs;
    let o = |divide| {
        BTreeMap::from([
            ("t::test_add".to_string(), Outcome::Passed),
            ("t::test_divide".to_string(), divide),
        ])
    };
    let (b, p) = (o(Outcome::Passed), o(Outcome::Skipped));
    let opts = DiffOptions {
        base_outcomes: Some(&b),
        patched_outcomes: Some(&p),
        ..Default::default()
    };
    let d = diff(
        &[snap(&[base])],
        &[snap(&[patched])],
        &LineMaps::new(),
        opts,
    )
    .unwrap();
    build_report(&d, json!({"mode": "diff"}), full, false, extra)
}

#[test]
fn canonical_text() {
    assert_eq!(
        render_text(&skip_divide(false, &[]), &hints()),
        "Tests with a changed outcome\n\
             \x20 t::test_divide: passed -> skipped\n\
             \n\
             Summary\n\
             \x20 lines      lost 3, gained 0, new hit 0, new miss 0, deleted hit 0, deleted miss 0, \
             flaky 0, unmeasured 0\n\
             \x20 branches   lost 0, gained 0, new 0, deleted 0\n\
             \x20 functions  went dark 1, lit up 0, partial loss 0, partial gain 0, mixed 0, added 0, \
             removed 0\n\
             \x20 coverage   100.0% -> 50.0%\n\
             \n\
             Files (top 20 by lost + gained lines)\n\
             \x20 shed/calc.py: lost 3\n\
             \x20   divide (4-7): went_dark, lost 3, gained 0\n"
    );
    // --full adds unchanged tests, functions and lines; the text still shows only changes.
    assert_eq!(
        render_text(&skip_divide(true, &[]), &[]),
        render_text(&skip_divide(false, &[]), &[])
    );
}

#[test]
fn warnings_then_hints() {
    let r = skip_divide(false, &[warning("dirty_tree", "uncommitted changes")]);
    let text = render_text(&r, &hints());
    assert!(text.ends_with(
            "\nWarnings\n  [dirty_tree] uncommitted changes\n\nHints\n  - Set [run] source = <your package>.\n"
        ));
    let order: Vec<_> = ["Tests with", "Summary", "Files (top", "Warnings", "Hints"]
        .iter()
        .map(|h| text.find(h).unwrap())
        .collect();
    assert!(order.is_sorted());
}

#[test]
fn hints_only_when_relevant() {
    assert!(!render_text(&skip_divide(false, &[]), &hints()).contains("Hints"));
    let mut r = skip_divide(false, &[]);
    r["files"][0]["inferred_side"] = json!("patched");
    assert!(render_text(&r, &hints()).contains("Hints"));
    assert!(!render_text(&skip_divide(false, &[warning("x", "y")]), &[]).contains("Hints"));

    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("pkg")).unwrap();
    std::fs::write(tmp.path().join("pkg/__init__.py"), "").unwrap();
    let opts = DiffOptions {
        base_root: Some(tmp.path()),
        ..Default::default()
    };
    let d = diff(
        &[snap(&[])],
        &[snap(&[fc("pkg/__init__.py", &[], &[])])],
        &LineMaps::new(),
        opts,
    )
    .unwrap();
    let r = build_report(&d, json!({}), true, false, &[]);
    assert!(!render_text(&r, &hints()).contains("Hints"));
}

#[test]
fn top_files_limit_and_order() {
    let base: Vec<_> = (0..25)
        .map(|i| {
            fc(
                &format!("pkg/m{i:02}.py"),
                &(1..=i + 1).map(|l| (l, 1)).collect::<Vec<_>>(),
                &[],
            )
        })
        .collect();
    let patched: Vec<_> = base
        .iter()
        .map(|f| {
            fc(
                &f.path,
                &f.lines.keys().map(|&l| (l, 0)).collect::<Vec<_>>(),
                &[],
            )
        })
        .collect();
    let text = render_text(
        &build_report(&plain(&base, &patched), json!({}), false, false, &[]),
        &[],
    );
    let listed: Vec<_> = text
        .lines()
        .filter(|l| l.starts_with("  pkg/"))
        .map(|l| l.split(':').next().unwrap().trim().to_string())
        .collect();
    let want: Vec<_> = (5..25).rev().map(|i| format!("pkg/m{i:02}.py")).collect();
    assert_eq!(listed, want);
    assert!(text.contains("  ... and 5 more\n"));
}

#[test]
fn nothing_changed_and_external() {
    let a = [fc("a.py", &[], &[])];
    let text = render_text(
        &build_report(&plain(&a, &a), json!({}), false, false, &[]),
        &hints(),
    );
    assert!(!text.contains("Tests with"));
    assert!(text.contains("  coverage   n/a -> n/a\n"));
    assert!(text.ends_with("No coverage changes.\n"));

    let mut ext = fc("/usr/lib/x.py", &[(1, 1)], &[]);
    ext.external = true;
    let d = plain(&[ext.clone()], &[ext]);
    let text = render_text(&build_report(&d, json!({}), false, false, &[]), &[]);
    assert!(text.contains("  external files (not in tree): 1\n"));
}
