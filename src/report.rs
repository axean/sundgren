//! Delta -> sundgren.delta/1 JSON report, and the report -> terminal summary.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use crate::diff::{BranchCategory, Delta, FileDelta, LineCategory, LineDelta};
use crate::model::Warning;
use crate::rollup::FunctionStatus;

pub const SCHEMA_ID: &str = "sundgren.delta/1";
pub const TOP_FILES: usize = 20;

const LINE_KEYS: [&str; 8] = [
    "lost",
    "gained",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
    "flaky",
    "unmeasured",
];
const BRANCH_KEYS: [&str; 4] = ["lost", "gained", "new", "deleted"];
const FUNCTION_KEYS: [&str; 7] = [
    "went_dark",
    "lit_up",
    "partial_loss",
    "partial_gain",
    "mixed",
    "added",
    "removed",
];

/// The sundgren.delta/1 report; by default only changes, `full` keeps everything.
pub fn build_report(
    delta: &Delta,
    meta: Value,
    full: bool,
    counts: bool,
    extra_warnings: &[Warning],
) -> Value {
    let warnings: BTreeSet<&Warning> = delta.warnings.iter().chain(extra_warnings).collect();
    let tests: Vec<_> = delta
        .tests
        .iter()
        .filter(|t| full || t.base != t.patched)
        .collect();
    let files: Vec<Value> = delta
        .files
        .iter()
        .filter(|f| full || f.changed() || (counts && f.lines.iter().any(count_changed)))
        .map(|f| file(f, full, counts))
        .collect();
    json!({
        "schema": SCHEMA_ID,
        "meta": meta,
        "tests": tests,
        "summary": summary(delta),
        "tree": delta.tree,
        "files": files,
        "warnings": warnings,
        "calls": null,
    })
}

fn summary(delta: &Delta) -> Value {
    let all_lines = || delta.files.iter().flat_map(|f| &f.lines);
    let mut lines = Map::new();
    for key in LINE_KEYS {
        let n = match key {
            "flaky" => delta.files.iter().map(|f| f.flaky.len()).sum(),
            _ => {
                let cat = LineCategory::parse(key).expect("line category");
                all_lines().filter(|e| e.category == cat).count()
            }
        };
        lines.insert(key.into(), n.into());
    }
    let mut branches = Map::new();
    for key in BRANCH_KEYS {
        let cat = BranchCategory::parse(&format!("branch_{key}")).expect("branch category");
        let n = delta
            .files
            .iter()
            .flat_map(|f| &f.branches)
            .filter(|b| b.category == cat)
            .count();
        branches.insert(key.into(), n.into());
    }
    let mut functions = Map::new();
    for key in FUNCTION_KEYS {
        let status = FunctionStatus::parse(key).expect("function status");
        let n = delta
            .files
            .iter()
            .flat_map(|f| &f.functions)
            .filter(|f| f.status == status)
            .count();
        functions.insert(key.into(), n.into());
    }
    let s = &delta.tree.stats;
    json!({
        "lines": lines,
        "branches": branches,
        "functions": functions,
        "coverage": {
            "base_pct": pct(s.hit_base, s.executable_base),
            "patched_pct": pct(s.hit_patched, s.executable_patched),
        },
        "external_files": delta.external_files,
    })
}

/// `hit * 100 / executable` rounded to 1 decimal, or null when nothing is executable.
fn pct(hit: i64, executable: i64) -> Option<f64> {
    (executable != 0).then(|| round1((hit * 100) as f64 / executable as f64))
}

/// Round half-even on the exact binary value, as Python's `round(x, 1)`.
fn round1(x: f64) -> f64 {
    format!("{x:.1}").parse().expect("formatted float")
}

fn file(f: &FileDelta, full: bool, counts: bool) -> Value {
    let lines: Vec<_> = f
        .lines
        .iter()
        .filter(|e| full || !e.category.is_kept())
        .collect();
    let branches: Vec<_> = f
        .branches
        .iter()
        .filter(|b| full || !b.category.is_kept())
        .collect();
    let functions: Vec<Value> = f
        .functions
        .iter()
        .filter(|func| full || func.status != FunctionStatus::Unchanged)
        .map(|func| {
            json!({
                "name": func.name,
                "start": func.start,
                "end": func.end,
                "status": func.status,
                "lines": {"lost": func.lost, "gained": func.gained, "executable": func.executable},
            })
        })
        .collect();
    let count_changed_lines: Vec<Value> = f
        .lines
        .iter()
        .filter(|e| counts && count_changed(e))
        .map(|e| json!({"line": e.line, "base_line": e.base_line, "hits": e.hits}))
        .collect();
    json!({
        "path": f.path,
        "base_path": f.base_path,
        "sha256": {"base": f.sha256.0, "patched": f.sha256.1},
        "source_changed": f.source_changed,
        "inferred_side": f.inferred_side,
        "lines": lines,
        "branches": branches,
        "functions": functions,
        "flaky": f.flaky,
        "count_changed": count_changed_lines,
    })
}

fn count_changed(e: &LineDelta) -> bool {
    e.category == LineCategory::KeptHit && e.hits.0 != e.hits.1
}

/// JSON with sorted keys, 2-space indent and a trailing newline.
pub fn dumps(value: &Value) -> String {
    let mut out = serde_json::to_string_pretty(value).expect("JSON value serializes");
    out.push('\n');
    out
}

/// A scalar as the text report shows it.
fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        other => other.to_string(),
    }
}

fn arr(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Test changes, summary counts, top files with their functions, warnings, hints.
pub fn render_text(report: &Value, hints: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    let changed_tests: Vec<&Value> = arr(&report["tests"])
        .iter()
        .filter(|t| t["base"] != t["patched"])
        .collect();
    if !changed_tests.is_empty() {
        out.push("Tests with a changed outcome".into());
        for t in changed_tests {
            out.push(format!(
                "  {}: {} -> {}",
                show(&t["id"]),
                show(&t["base"]),
                show(&t["patched"])
            ));
        }
        out.push(String::new());
    }

    let s = &report["summary"];
    let counts = |v: &Value, keys: &[&str]| -> String {
        let parts: Vec<String> = keys
            .iter()
            .map(|k| format!("{} {}", k.replace('_', " "), show(&v[*k])))
            .collect();
        parts.join(", ")
    };
    let pct = |v: &Value| match v.as_f64() {
        Some(f) => format!("{f:.1}%"),
        None => "n/a".into(),
    };
    out.push("Summary".into());
    out.push(format!("  lines      {}", counts(&s["lines"], &LINE_KEYS)));
    out.push(format!(
        "  branches   {}",
        counts(&s["branches"], &BRANCH_KEYS)
    ));
    out.push(format!(
        "  functions  {}",
        counts(&s["functions"], &FUNCTION_KEYS)
    ));
    out.push(format!(
        "  coverage   {} -> {}",
        pct(&s["coverage"]["base_pct"]),
        pct(&s["coverage"]["patched_pct"])
    ));
    if truthy(&s["external_files"]) {
        out.push(format!(
            "  external files (not in tree): {}",
            show(&s["external_files"])
        ));
    }
    out.push(String::new());

    let mut files: Vec<(usize, &str, &Value)> = arr(&report["files"])
        .iter()
        .filter(|f| changed(f))
        .map(|f| {
            let weight = line_count(f, "lost") + line_count(f, "gained");
            (weight, f["path"].as_str().unwrap_or(""), f)
        })
        .collect();
    if files.is_empty() {
        out.push("No coverage changes.".into());
    } else {
        files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        out.push(format!("Files (top {TOP_FILES} by lost + gained lines)"));
        for (_, path, f) in files.iter().take(TOP_FILES) {
            out.push(format!("  {path}: {}", file_counts(f)));
            for func in arr(&f["functions"]) {
                if func["status"] == "unchanged" {
                    continue;
                }
                out.push(format!(
                    "    {} ({}-{}): {}, lost {}, gained {}",
                    show(&func["name"]),
                    show(&func["start"]),
                    show(&func["end"]),
                    show(&func["status"]),
                    show(&func["lines"]["lost"]),
                    show(&func["lines"]["gained"]),
                ));
            }
        }
        if files.len() > TOP_FILES {
            out.push(format!("  ... and {} more", files.len() - TOP_FILES));
        }
    }

    let warnings = arr(&report["warnings"]);
    if !warnings.is_empty() {
        out.push(String::new());
        out.push("Warnings".into());
        for w in warnings {
            out.push(format!("  [{}] {}", show(&w["code"]), show(&w["message"])));
        }
    }
    let inferred = arr(&report["files"])
        .iter()
        .any(|f| truthy(&f["inferred_side"]));
    if !hints.is_empty() && (!warnings.is_empty() || inferred) {
        out.push(String::new());
        out.push("Hints".into());
        out.extend(hints.iter().map(|h| format!("  - {h}")));
    }
    out.join("\n") + "\n"
}

fn category(v: &Value) -> &str {
    v["category"].as_str().unwrap_or("")
}

fn changed(f: &Value) -> bool {
    arr(&f["lines"])
        .iter()
        .any(|e| !category(e).starts_with("kept_"))
        || arr(&f["branches"])
            .iter()
            .any(|b| !category(b).starts_with("branch_kept_"))
        || arr(&f["functions"])
            .iter()
            .any(|func| func["status"] != "unchanged")
        || truthy(&f["flaky"])
}

fn line_count(f: &Value, cat: &str) -> usize {
    arr(&f["lines"])
        .iter()
        .filter(|e| category(e) == cat)
        .count()
}

fn file_counts(f: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    for k in [
        "lost",
        "gained",
        "new_hit",
        "new_miss",
        "deleted_hit",
        "deleted_miss",
        "unmeasured",
    ] {
        let n = line_count(f, k);
        if n > 0 {
            parts.push(format!("{} {n}", k.replace('_', " ")));
        }
    }
    let flaky = arr(&f["flaky"]).len();
    if flaky > 0 {
        parts.push(format!("flaky {flaky}"));
    }
    let mut branch_parts = Vec::new();
    for k in ["lost", "gained", "new", "deleted", "flaky", "unmeasured"] {
        let n = arr(&f["branches"])
            .iter()
            .filter(|b| category(b).strip_prefix("branch_") == Some(k))
            .count();
        if n > 0 {
            branch_parts.push(format!("{k} {n}"));
        }
    }
    if !branch_parts.is_empty() {
        parts.push(format!("branches {}", branch_parts.join(", ")));
    }
    if truthy(&f["inferred_side"]) {
        parts.push(format!("{} side inferred", show(&f["inferred_side"])));
    }
    if parts.is_empty() {
        return "functions only".into();
    }
    parts.join("; ")
}

#[cfg(test)]
mod tests;
