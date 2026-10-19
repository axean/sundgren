//! sundgren.delta/1 report -> terminal summary. Port of core/report/text_report.py.
//!
//! Renders from the report value, not the Delta, so a reloaded report renders the same.

use std::fmt;

use crate::pytext::PyText;
use crate::report_json::{BRANCH_KEYS, FUNCTION_KEYS, Json, LINE_KEYS};

pub const TOP_FILES: usize = 20;

const FILE_LINE_KEYS: [&str; 7] = [
    "lost",
    "gained",
    "new_hit",
    "new_miss",
    "deleted_hit",
    "deleted_miss",
    "unmeasured",
];
const FILE_BRANCH_KEYS: [&str; 6] = ["lost", "gained", "new", "deleted", "flaky", "unmeasured"];

/// The report lacks a key or has a value of the wrong type (Python: KeyError/TypeError).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError(pub String);

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

type Res<T> = Result<T, RenderError>;

fn key<'a>(v: &'a Json, k: &str) -> Res<&'a Json> {
    v.get(k)
        .ok_or_else(|| RenderError(format!("missing key {k:?}")))
}

fn arr<'a>(v: &'a Json, k: &str) -> Res<&'a [Json]> {
    key(v, k)?
        .as_array()
        .ok_or_else(|| RenderError(format!("{k:?} is not a list")))
}

fn text<'a>(v: &'a Json, k: &str) -> Res<&'a PyText> {
    key(v, k)?
        .as_text()
        .ok_or_else(|| RenderError(format!("{k:?} is not a str")))
}

/// Output lines.
struct Out(Vec<PyText>);

impl Out {
    fn push(&mut self, s: impl Into<PyText>) {
        self.0.push(s.into());
    }
}

fn cat(parts: &[&PyText]) -> PyText {
    let mut out = PyText::new();
    for p in parts {
        out.push_text(p);
    }
    out
}

fn t(s: &str) -> PyText {
    s.into()
}

/// Test changes, summary counts, top files with their functions, warnings, hints.
pub fn render_text(report: &Json, hints: &[PyText]) -> Res<PyText> {
    let mut out = Out(Vec::new());
    let mut changed_tests = Vec::new();
    for test in arr(report, "tests")? {
        if key(test, "base")? != key(test, "patched")? {
            changed_tests.push(test);
        }
    }
    if !changed_tests.is_empty() {
        out.push("Tests with a changed outcome");
        for test in changed_tests {
            out.push(cat(&[
                &t("  "),
                &key(test, "id")?.py_str(),
                &t(": "),
                &key(test, "base")?.py_str(),
                &t(" -> "),
                &key(test, "patched")?.py_str(),
            ]));
        }
        out.push("");
    }

    let s = key(report, "summary")?;
    let cov = key(s, "coverage")?;
    let mut line_keys: Vec<&str> = LINE_KEYS.to_vec();
    line_keys.extend(["flaky", "unmeasured"]);
    out.push("Summary");
    out.push(cat(&[
        &t("  lines      "),
        &counts(key(s, "lines")?, &line_keys)?,
    ]));
    out.push(cat(&[
        &t("  branches   "),
        &counts(key(s, "branches")?, &BRANCH_KEYS)?,
    ]));
    out.push(cat(&[
        &t("  functions  "),
        &counts(key(s, "functions")?, &FUNCTION_KEYS)?,
    ]));
    out.push(format!(
        "  coverage   {} -> {}",
        pct(key(cov, "base_pct")?)?,
        pct(key(cov, "patched_pct")?)?
    ));
    let external = key(s, "external_files")?;
    if external.is_truthy() {
        out.push(cat(&[
            &t("  external files (not in tree): "),
            &external.py_str(),
        ]));
    }
    out.push("");

    let mut files = Vec::new();
    for f in arr(report, "files")? {
        if changed(f)? {
            files.push(f);
        }
    }
    if files.is_empty() {
        out.push("No coverage changes.");
    } else {
        let mut keyed = Vec::new();
        for f in files {
            let weight = line_count(f, "lost")? + line_count(f, "gained")?;
            keyed.push((weight, text(f, "path")?, f));
        }
        keyed.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        out.push(format!("Files (top {TOP_FILES} by lost + gained lines)"));
        for (_, path, f) in keyed.iter().take(TOP_FILES) {
            out.push(cat(&[&t("  "), path, &t(": "), &file_counts(f)?]));
            for func in arr(f, "functions")? {
                let status = key(func, "status")?;
                if status.as_text().is_some_and(|s| s == "unchanged") {
                    continue;
                }
                let n = key(func, "lines")?;
                out.push(cat(&[
                    &t("    "),
                    &key(func, "name")?.py_str(),
                    &t(" ("),
                    &key(func, "start")?.py_str(),
                    &t("-"),
                    &key(func, "end")?.py_str(),
                    &t("): "),
                    &status.py_str(),
                    &t(", lost "),
                    &key(n, "lost")?.py_str(),
                    &t(", gained "),
                    &key(n, "gained")?.py_str(),
                ]));
            }
        }
        if keyed.len() > TOP_FILES {
            out.push(format!("  ... and {} more", keyed.len() - TOP_FILES));
        }
    }

    let warnings = arr(report, "warnings")?;
    if !warnings.is_empty() {
        out.push("");
        out.push("Warnings");
        for w in warnings {
            out.push(cat(&[
                &t("  ["),
                &key(w, "code")?.py_str(),
                &t("] "),
                &key(w, "message")?.py_str(),
            ]));
        }
    }
    if !hints.is_empty() {
        let mut inferred = false;
        for f in arr(report, "files")? {
            if key(f, "inferred_side")?.is_truthy() {
                inferred = true;
                break;
            }
        }
        if !warnings.is_empty() || inferred {
            out.push("");
            out.push("Hints");
            for h in hints {
                out.push(cat(&[&t("  - "), h]));
            }
        }
    }

    let mut result = PyText::new();
    for (i, line) in out.0.iter().enumerate() {
        if i > 0 {
            result.push_str("\n");
        }
        result.push_text(line);
    }
    result.push_str("\n");
    Ok(result)
}

fn counts(values: &Json, keys: &[&str]) -> Res<PyText> {
    let mut out = PyText::new();
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&k.replace('_', " "));
        out.push_str(" ");
        out.push_text(&key(values, k)?.py_str());
    }
    Ok(out)
}

fn pct(value: &Json) -> Res<String> {
    match value {
        Json::Null => Ok("n/a".into()),
        Json::Float(f) => Ok(format!("{f:.1}%")),
        Json::Int(i) => Ok(format!("{i}.0%")),
        _ => Err(RenderError("coverage is not a number".into())),
    }
}

fn category_is(v: &Json, k: &str, pred: impl Fn(&PyText) -> bool) -> Res<bool> {
    text(v, k).map(pred)
}

fn changed(f: &Json) -> Res<bool> {
    for e in arr(f, "lines")? {
        if !category_is(e, "category", |c| c.starts_with(b"kept_"))? {
            return Ok(true);
        }
    }
    for b in arr(f, "branches")? {
        if !category_is(b, "category", |c| c.starts_with(b"branch_kept_"))? {
            return Ok(true);
        }
    }
    for func in arr(f, "functions")? {
        if key(func, "status")?
            .as_text()
            .is_none_or(|s| s != "unchanged")
        {
            return Ok(true);
        }
    }
    Ok(key(f, "flaky")?.is_truthy())
}

fn line_count(f: &Json, category: &str) -> Res<usize> {
    let mut n = 0;
    for e in arr(f, "lines")? {
        if key(e, "category")?.as_text().is_some_and(|c| c == category) {
            n += 1;
        }
    }
    Ok(n)
}

fn file_counts(f: &Json) -> Res<PyText> {
    let lines = arr(f, "lines")?;
    let branches = arr(f, "branches")?;
    let count_lines = |k: &str| -> Res<usize> {
        let mut n = 0;
        for e in lines {
            n += usize::from(text(e, "category")? == k);
        }
        Ok(n)
    };
    let count_branches = |k: &str| -> Res<usize> {
        let mut n = 0;
        for b in branches {
            let c = text(b, "category")?;
            let bare = c
                .as_bytes()
                .strip_prefix(b"branch_")
                .unwrap_or(c.as_bytes());
            n += usize::from(bare == k.as_bytes());
        }
        Ok(n)
    };
    let mut parts: Vec<PyText> = Vec::new();
    for k in FILE_LINE_KEYS {
        let n = count_lines(k)?;
        if n > 0 {
            parts.push(format!("{} {n}", k.replace('_', " ")).into());
        }
    }
    let flaky = key(f, "flaky")?;
    if flaky.is_truthy() {
        let n = flaky.as_array().map_or(0, <[Json]>::len);
        parts.push(format!("flaky {n}").into());
    }
    let mut branch_parts = Vec::new();
    for k in FILE_BRANCH_KEYS {
        let n = count_branches(k)?;
        if n > 0 {
            branch_parts.push(format!("{k} {n}"));
        }
    }
    if !branch_parts.is_empty() {
        parts.push(format!("branches {}", branch_parts.join(", ")).into());
    }
    let inferred = key(f, "inferred_side")?;
    if inferred.is_truthy() {
        parts.push(cat(&[&inferred.py_str(), &t(" side inferred")]));
    }
    if parts.is_empty() {
        return Ok("functions only".into());
    }
    let mut out = PyText::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str("; ");
        }
        out.push_text(p);
    }
    Ok(out)
}
