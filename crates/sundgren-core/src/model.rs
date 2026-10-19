//! Coverage data model shared by every layer. Port of `sundgren/core/model.py`.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::str::FromStr;

use serde_json::{Value, json};

/// 1-based line number. Signed, because lcov input is read with Python's `int()` rules.
pub type LineNo = i64;

/// Hit count. Wide and signed: lcov counts are taken as written, including overflowed
/// counters (2^64 - 1) and negative ones.
pub type Count = i128;

/// Test outcome, as in the JUnit map and the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Outcome {
    Passed,
    Failed,
    Skipped,
    Xfailed,
    Xpassed,
    Error,
    NotRun,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Passed => "passed",
            Outcome::Failed => "failed",
            Outcome::Skipped => "skipped",
            Outcome::Xfailed => "xfailed",
            Outcome::Xpassed => "xpassed",
            Outcome::Error => "error",
            Outcome::NotRun => "not_run",
        }
    }
}

impl FromStr for Outcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s {
            "passed" => Outcome::Passed,
            "failed" => Outcome::Failed,
            "skipped" => Outcome::Skipped,
            "xfailed" => Outcome::Xfailed,
            "xpassed" => Outcome::Xpassed,
            "error" => Outcome::Error,
            "not_run" => Outcome::NotRun,
            _ => return Err(format!("unknown outcome {s:?}")),
        })
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineStatus {
    Absent,
    Miss,
    Hit,
}

impl LineStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LineStatus::Absent => "absent",
            LineStatus::Miss => "miss",
            LineStatus::Hit => "hit",
        }
    }
}

/// Ordered by (line, block, branch), like the dataclass with `order=True`.
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

impl FileCoverage {
    pub fn status(&self, line: LineNo) -> LineStatus {
        match self.lines.get(&line) {
            None => LineStatus::Absent,
            Some(&c) if c > 0 => LineStatus::Hit,
            Some(_) => LineStatus::Miss,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub files: BTreeMap<String, FileCoverage>,
    /// From lcov TN:, informational.
    pub test_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReportWarning {
    /// Stable machine id, e.g. "unmeasured", "inferred_file", "dirty_tree".
    pub code: String,
    pub message: String,
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
    for f in &a.functions {
        funcs.insert(&f.name, f.clone());
    }
    for f in &b.functions {
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
pub fn snapshot_to_dict(snapshot: &Snapshot) -> Value {
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
mod tests {
    use super::*;

    fn fc(lines: &[(LineNo, Count)], funcs: Vec<FunctionRecord>) -> FileCoverage {
        FileCoverage {
            path: "a.py".into(),
            lines: lines.iter().copied().collect(),
            functions: funcs,
            ..Default::default()
        }
    }

    fn func(name: &str, start: LineNo, end: Option<LineNo>, hits: Option<Count>) -> FunctionRecord {
        FunctionRecord {
            name: name.into(),
            start,
            end,
            hits,
        }
    }

    #[test]
    fn status() {
        let f = fc(&[(1, 0), (2, 3)], vec![]);
        assert_eq!(f.status(1), LineStatus::Miss);
        assert_eq!(f.status(2), LineStatus::Hit);
        assert_eq!(f.status(3), LineStatus::Absent);
    }

    #[test]
    fn branch_key_order() {
        let k = |l, b: &str, r: &str| BranchKey {
            line: l,
            block: b.into(),
            branch: r.into(),
        };
        let mut v = vec![
            k(2, "0", "0"),
            k(1, "1", "0"),
            k(1, "0", "1"),
            k(1, "0", "0"),
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                k(1, "0", "0"),
                k(1, "0", "1"),
                k(1, "1", "0"),
                k(2, "0", "0")
            ]
        );
    }

    #[test]
    fn merge() {
        let key = BranchKey {
            line: 1,
            block: "0".into(),
            branch: "0".into(),
        };
        let mut a = fc(&[(1, 1), (2, 0)], vec![func("f", 1, None, None)]);
        a.branches.insert(key.clone(), None);
        a.external = true;
        let mut b = fc(
            &[(2, 2), (3, 0)],
            vec![func("f", 1, Some(4), Some(2)), func("e", 1, None, Some(0))],
        );
        b.branches.insert(key.clone(), Some(3));
        let m = merge_file_coverage(&a, &b);
        assert_eq!(m.lines, [(1, 1), (2, 2), (3, 0)].into_iter().collect());
        assert_eq!(m.branches[&key], Some(3));
        assert_eq!(
            m.functions,
            vec![func("e", 1, None, Some(0)), func("f", 1, Some(4), Some(2))]
        );
        assert!(!m.external);
    }

    #[test]
    fn outcome_roundtrip() {
        for s in [
            "passed", "failed", "skipped", "xfailed", "xpassed", "error", "not_run",
        ] {
            assert_eq!(s.parse::<Outcome>().unwrap().as_str(), s);
        }
        assert!("nope".parse::<Outcome>().is_err());
    }

    #[test]
    fn to_dict() {
        let mut f = fc(&[(10, 1), (2, 0)], vec![func("f", 1, None, Some(1))]);
        f.branches.insert(
            BranchKey {
                line: 2,
                block: "0".into(),
                branch: "1".into(),
            },
            None,
        );
        let snap = Snapshot {
            files: [("a.py".to_string(), f)].into_iter().collect(),
            test_names: vec!["t".into()],
        };
        let v = snapshot_to_dict(&snap);
        assert_eq!(v["test_names"], json!(["t"]));
        assert_eq!(v["files"][0]["lines"], json!({"2": 0, "10": 1}));
        assert_eq!(v["files"][0]["branches"][0]["taken"], Value::Null);
        assert_eq!(v["files"][0]["functions"][0]["end"], Value::Null);
        let mut big = Snapshot::default();
        big.files
            .insert("b".into(), fc(&[(-1, 18446744073709551615)], vec![]));
        let out = serde_json::to_string(&snapshot_to_dict(&big)).unwrap();
        assert!(
            out.contains(r#""lines":{"-1":18446744073709551615}"#),
            "{out}"
        );
    }
}
