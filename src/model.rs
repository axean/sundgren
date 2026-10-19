//! Coverage data model shared by every layer.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};

use crate::str_enum;

/// 1-based line number.
pub type LineNo = i64;

/// Hit count. Wide and signed: lcov counts are taken as written, including overflowed
/// counters (2^64 - 1) and negative ones.
pub type Count = i128;

str_enum!(
    /// Test outcome, as in the JUnit map and the report.
    Outcome {
        Passed = "passed",
        Failed = "failed",
        Skipped = "skipped",
        Xfailed = "xfailed",
        Xpassed = "xpassed",
        Error = "error",
        NotRun = "not_run",
    }
);

/// Test id to outcome.
pub type Outcomes = BTreeMap<String, Outcome>;

/// Ordered by (line, block, branch).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BranchKey {
    pub line: LineNo,
    /// lcov block id, kept as a string (may carry an 'e' prefix).
    pub block: String,
    pub branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionRecord {
    pub name: String,
    pub start: LineNo,
    /// None when the lcov version has no end line.
    pub end: Option<LineNo>,
    /// From FNDA.
    pub hits: Option<Count>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileCoverage {
    /// Normalized, repo-relative, posix.
    pub path: String,
    /// Executable line -> hit count.
    pub lines: BTreeMap<LineNo, Count>,
    /// None = '-' (block never reached).
    pub branches: BTreeMap<BranchKey, Option<Count>>,
    pub functions: Vec<FunctionRecord>,
    /// Path couldn't be made repo-relative.
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub files: BTreeMap<String, FileCoverage>,
    /// From lcov TN:, informational.
    pub test_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub struct Warning {
    /// Stable machine id, e.g. "unmeasured", "inferred_file", "dirty_tree".
    pub code: String,
    pub message: String,
}

pub fn warning(code: &str, message: impl Into<String>) -> Warning {
    Warning {
        code: code.to_string(),
        message: message.into(),
    }
}

/// Sum of two taken counts, where None ('-') is the identity.
pub fn add_taken(a: Option<Count>, b: Option<Count>) -> Option<Count> {
    match (a, b) {
        (None, b) => b,
        (a, None) => a,
        (Some(a), Some(b)) => Some(a + b),
    }
}

/// Merge two records for the same file: sum DA and BRDA, merge FNDA by name.
pub fn merge_file_coverage(a: &FileCoverage, b: &FileCoverage) -> FileCoverage {
    let mut lines = a.lines.clone();
    for (&ln, &count) in &b.lines {
        *lines.entry(ln).or_insert(0) += count;
    }
    let mut branches = a.branches.clone();
    for (key, &taken) in &b.branches {
        branches
            .entry(key.clone())
            .and_modify(|t| *t = add_taken(*t, taken))
            .or_insert(taken);
    }
    let mut funcs: HashMap<&str, FunctionRecord> = HashMap::new();
    for f in a.functions.iter().chain(&b.functions) {
        match funcs.get_mut(f.name.as_str()) {
            None => {
                funcs.insert(&f.name, f.clone());
            }
            Some(old) => {
                old.end = old.end.or(f.end);
                old.hits = add_taken(old.hits, f.hits);
            }
        }
    }
    let mut functions: Vec<FunctionRecord> = funcs.into_values().collect();
    functions.sort_by(|x, y| (x.start, &x.name).cmp(&(y.start, &y.name)));
    FileCoverage {
        path: a.path.clone(),
        lines,
        branches,
        functions,
        external: a.external && b.external,
    }
}

/// Plain-JSON view of a Snapshot (for `sundgren ingest`).
pub fn snapshot_json(snapshot: &Snapshot) -> Value {
    let files: Vec<Value> = snapshot
        .files
        .values()
        .map(|fc| {
            let lines: serde_json::Map<String, Value> = fc
                .lines
                .iter()
                .map(|(ln, c)| (ln.to_string(), json!(c)))
                .collect();
            let branches: Vec<Value> = fc
                .branches
                .iter()
                .map(|(k, t)| {
                    json!({"line": k.line, "block": k.block, "branch": k.branch, "taken": t})
                })
                .collect();
            let functions: Vec<Value> = fc
                .functions
                .iter()
                .map(|f| json!({"name": f.name, "start": f.start, "end": f.end, "hits": f.hits}))
                .collect();
            json!({
                "path": fc.path,
                "external": fc.external,
                "lines": lines,
                "branches": branches,
                "functions": functions,
            })
        })
        .collect();
    json!({"test_names": snapshot.test_names, "files": files})
}

#[cfg(test)]
mod tests;
