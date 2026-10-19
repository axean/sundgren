//! lcov tracefile parser and dumper. Port of `sundgren/core/ingest/lcov.py`.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fmt::Write;

use crate::model::{
    BranchKey, Count, FileCoverage, FunctionRecord, LineNo, Snapshot, add_taken,
    merge_file_coverage,
};
use crate::pyint::{py_int, py_int_i64, py_repr, py_repr_list, py_strip};

/// A known record that doesn't parse. Displays as the prototype's ValueError message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LcovError {
    /// 1-based line number in the tracefile.
    pub lineno: usize,
    pub tag: String,
    /// The stripped line.
    pub line: String,
}

impl fmt::Display for LcovError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lcov line {}: malformed {} record: {}",
            self.lineno,
            self.tag,
            py_repr(&self.line)
        )
    }
}

impl std::error::Error for LcovError {}

/// Summary records are recomputed by dump(); VER has no slot in the model.
const DROPPED: [&str; 7] = ["FNF", "FNH", "BRF", "BRH", "LF", "LH", "VER"];
const HANDLED: [&str; 6] = ["DA", "BRDA", "FN", "FNDA", "FNL", "FNA"];

/// The record didn't parse; the caller adds the location.
struct Malformed;

impl<E: std::error::Error> From<E> for Malformed {
    fn from(_: E) -> Self {
        Malformed
    }
}

type Handled = Result<(), Malformed>;

/// Accumulator for one SF section.
struct Record {
    path: String,
    lines: BTreeMap<LineNo, Count>,
    branches: BTreeMap<BranchKey, Option<Count>>,
    spans: HashMap<String, (LineNo, Option<LineNo>)>,
    hits: HashMap<String, Count>,
    fn_index: HashMap<String, (LineNo, Option<LineNo>)>,
}

impl Record {
    fn new(path: &str) -> Self {
        Record {
            path: path.to_string(),
            lines: BTreeMap::new(),
            branches: BTreeMap::new(),
            spans: HashMap::new(),
            hits: HashMap::new(),
            fn_index: HashMap::new(),
        }
    }

    fn handle(&mut self, tag: &str, value: &str) -> Handled {
        match tag {
            "DA" => self.da(value),
            "BRDA" => self.brda(value),
            "FN" => self.fn_(value),
            "FNDA" => self.fnda(value),
            "FNL" => self.fnl(value),
            "FNA" => self.fna(value),
            _ => unreachable!("unhandled tag {tag}"),
        }
    }

    fn da(&mut self, value: &str) -> Handled {
        let mut parts = value.split(',');
        let (Some(line), Some(count)) = (parts.next(), parts.next()) else {
            return Err(Malformed);
        };
        let ln = py_int_i64(line)?;
        *self.lines.entry(ln).or_insert(0) += py_int(count)?;
        Ok(())
    }

    fn brda(&mut self, value: &str) -> Handled {
        // Branch ids may contain commas (lcov 2.x expressions), so split from both ends.
        let mut parts = value.splitn(3, ',');
        let (Some(line), Some(block), Some(rest)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(Malformed);
        };
        let (branch, taken_s) = rest.rsplit_once(',').ok_or(Malformed)?;
        let taken = if taken_s == "-" {
            None
        } else {
            Some(py_int(taken_s)?)
        };
        let key = BranchKey {
            line: py_int_i64(line)?,
            block: block.to_string(),
            branch: branch.to_string(),
        };
        self.branches
            .entry(key)
            .and_modify(|t| *t = add_taken(*t, taken))
            .or_insert(taken);
        Ok(())
    }

    fn fn_(&mut self, value: &str) -> Handled {
        let (start, rest) = value.split_once(',').unwrap_or((value, ""));
        match rest.split_once(',') {
            // lcov 2.x: FN:<start>,<end>,<name>
            Some((mid, name)) if !mid.is_empty() && mid.bytes().all(|b| b.is_ascii_digit()) => {
                self.span(name, py_int_i64(start)?, Some(py_int_i64(mid)?))
            }
            _ => self.span(rest, py_int_i64(start)?, None),
        }
    }

    fn fnda(&mut self, value: &str) -> Handled {
        let (count, name) = value.split_once(',').unwrap_or((value, ""));
        self.hit(name, py_int(count)?)
    }

    fn fnl(&mut self, value: &str) -> Handled {
        let mut parts = value.split(',');
        let index = parts.next().unwrap_or_default();
        let span: Vec<&str> = parts.collect();
        let end = match span.len() {
            1 => None,
            2 => Some(py_int_i64(span[1])?),
            _ => return Err(Malformed),
        };
        let start = py_int_i64(span[0])?;
        self.fn_index.insert(index.to_string(), (start, end));
        Ok(())
    }

    fn fna(&mut self, value: &str) -> Handled {
        let mut parts = value.splitn(3, ',');
        let (Some(index), Some(count), Some(name)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(Malformed);
        };
        let &(start, end) = self.fn_index.get(index).ok_or(Malformed)?;
        self.span(name, start, end)?;
        self.hit(name, py_int(count)?)
    }

    fn span(&mut self, name: &str, start: LineNo, end: Option<LineNo>) -> Handled {
        if name.is_empty() {
            return Err(Malformed);
        }
        match self.spans.get_mut(name) {
            None => {
                self.spans.insert(name.to_string(), (start, end));
            }
            Some(old) if old.1.is_none() => old.1 = end,
            Some(_) => {}
        }
        Ok(())
    }

    fn hit(&mut self, name: &str, count: Count) -> Handled {
        if name.is_empty() {
            return Err(Malformed);
        }
        *self.hits.entry(name.to_string()).or_insert(0) += count;
        Ok(())
    }

    fn build(self, log: &mut dyn FnMut(String)) -> FileCoverage {
        let mut orphans: Vec<&String> = self
            .hits
            .keys()
            .filter(|n| !self.spans.contains_key(*n))
            .collect();
        if !orphans.is_empty() {
            orphans.sort();
            log(format!(
                "{}: dropping FNDA without FN: {}",
                self.path,
                py_repr_list(&orphans)
            ));
        }
        // TODO: demangle Rust names
        let mut functions: Vec<FunctionRecord> = self
            .spans
            .iter()
            .map(|(name, &(start, end))| FunctionRecord {
                name: name.clone(),
                start,
                end,
                hits: self.hits.get(name).copied(),
            })
            .collect();
        sort_functions(&mut functions);
        FileCoverage {
            path: self.path,
            lines: self.lines,
            branches: self.branches,
            functions,
            external: false,
        }
    }
}

fn sort_functions(functions: &mut [FunctionRecord]) {
    functions.sort_by(|a, b| (a.start, &a.name).cmp(&(b.start, &b.name)));
}

fn store(files: &mut BTreeMap<String, FileCoverage>, fc: FileCoverage) {
    let merged = match files.remove(&fc.path) {
        None => fc,
        Some(old) => merge_file_coverage(&old, &fc),
    };
    files.insert(merged.path.clone(), merged);
}

/// Parse an lcov tracefile. Paths stay raw; duplicate SF sections are merged.
pub fn parse(text: &str) -> Result<Snapshot, LcovError> {
    parse_logged(text, &mut |_| {})
}

/// `parse`, sending debug messages (ignored records, dropped FNDA) to `log`.
pub fn parse_logged(text: &str, log: &mut dyn FnMut(String)) -> Result<Snapshot, LcovError> {
    let mut files: BTreeMap<String, FileCoverage> = BTreeMap::new();
    let mut test_names: Vec<String> = Vec::new();
    let mut rec: Option<Record> = None;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    // Split on '\n' only: str.splitlines() would also split on \f, \x85,   etc. in paths.
    for (i, raw) in text.split('\n').enumerate() {
        let lineno = i + 1;
        let line = py_strip(raw);
        if line.is_empty() {
            continue;
        }
        if line == "end_of_record" {
            if let Some(r) = rec.take() {
                store(&mut files, r.build(log));
            }
            continue;
        }
        let (tag, value) = match line.split_once(':') {
            Some((t, v)) => (t, Some(v)),
            None => (line, None),
        };
        match (tag, value) {
            ("TN", Some(v)) => {
                if !v.is_empty() && !test_names.iter().any(|n| n == v) {
                    test_names.push(v.to_string());
                }
            }
            ("SF", Some(v)) => {
                if let Some(r) = rec.replace(Record::new(v)) {
                    store(&mut files, r.build(log));
                }
            }
            (t, Some(_)) if DROPPED.contains(&t) => {}
            (t, Some(v)) if HANDLED.contains(&t) => match rec.as_mut() {
                None => log(format!(
                    "lcov line {lineno}: ignoring {t} record outside SF"
                )),
                Some(r) => r.handle(t, v).map_err(|_| LcovError {
                    lineno,
                    tag: t.to_string(),
                    line: line.to_string(),
                })?,
            },
            _ => log(format!(
                "lcov line {lineno}: ignoring unknown record {}",
                py_repr(line)
            )),
        }
    }
    if let Some(r) = rec.take() {
        store(&mut files, r.build(log));
    }
    Ok(Snapshot { files, test_names })
}

/// Serialize a Snapshot back to lcov (summary records recomputed).
pub fn dump(snapshot: &Snapshot) -> String {
    let mut out = String::new();
    let mut line = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    for name in &snapshot.test_names {
        line(format!("TN:{name}"));
    }
    for fc in snapshot.files.values() {
        line(format!("SF:{}", fc.path));
        if !fc.functions.is_empty() {
            for f in &fc.functions {
                let mut span = f.start.to_string();
                if let Some(end) = f.end {
                    let _ = write!(span, ",{end}");
                }
                line(format!("FN:{span},{}", f.name));
            }
            for f in &fc.functions {
                if let Some(h) = f.hits {
                    line(format!("FNDA:{h},{}", f.name));
                }
            }
            line(format!("FNF:{}", fc.functions.len()));
            let fnh = fc.functions.iter().filter(|f| f.hits.unwrap_or(0) != 0);
            line(format!("FNH:{}", fnh.count()));
        }
        if !fc.branches.is_empty() {
            for (bk, taken) in &fc.branches {
                let taken = taken.map_or("-".to_string(), |t| t.to_string());
                line(format!(
                    "BRDA:{},{},{},{taken}",
                    bk.line, bk.block, bk.branch
                ));
            }
            line(format!("BRF:{}", fc.branches.len()));
            let brh = fc.branches.values().filter(|t| t.unwrap_or(0) != 0);
            line(format!("BRH:{}", brh.count()));
        }
        if !fc.lines.is_empty() {
            for (ln, c) in &fc.lines {
                line(format!("DA:{ln},{c}"));
            }
            line(format!("LF:{}", fc.lines.len()));
            line(format!(
                "LH:{}",
                fc.lines.values().filter(|&&c| c > 0).count()
            ));
        }
        line("end_of_record".to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_merge() {
        let text = "\u{feff}TN:t\nSF:a\nFN:1,3,f\nFN:5,g,h\nFNDA:2,f\nFNDA:1,ghost\n\
                    DA:1,1\nDA:1,2\nBRDA:1,0,a,b,-\nBRDA:1,0,a,b,3\nend_of_record\n\
                    SF:a\nFNL:0,7\nFNA:0,4,k\nDA:2,0\nend_of_record\n";
        let mut logs = Vec::new();
        let snap = parse_logged(text, &mut |m| logs.push(m)).unwrap();
        let fc = &snap.files["a"];
        assert_eq!(snap.test_names, vec!["t"]);
        assert_eq!(fc.lines, [(1, 3), (2, 0)].into_iter().collect());
        let names: Vec<_> = fc.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["f", "g,h", "k"]);
        assert_eq!(fc.functions[2].hits, Some(4));
        assert_eq!(fc.branches.values().next(), Some(&Some(3)));
        assert_eq!(logs, ["a: dropping FNDA without FN: ['ghost']"]);
        assert_eq!(parse(&dump(&snap)).unwrap(), snap);
    }

    #[test]
    fn malformed() {
        for rec in [
            "DA:1",
            "DA:x,1",
            "BRDA:1,0,1",
            "BRDA:1,0,0,x",
            "FN:x,f",
            "FN:4",
            "FNDA:1",
            "FNA:9,1,f",
            "FNL:1",
            "FNL:1,2,3,4",
        ] {
            let err = parse(&format!("SF:x\n{rec}\nend_of_record\n")).unwrap_err();
            assert!(
                err.to_string().starts_with("lcov line 2: malformed"),
                "{err}"
            );
        }
    }
}
