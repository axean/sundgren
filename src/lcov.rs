//! lcov tracefile parser.

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use crate::model::{
    BranchKey, Count, FileCoverage, FunctionRecord, LineNo, Snapshot, add_taken,
    merge_file_coverage,
};

/// Everything else (summaries, VER, unknown records) is skipped.
const HANDLED: [&str; 6] = ["DA", "BRDA", "FN", "FNDA", "FNL", "FNA"];

/// The record didn't parse; the caller adds the location.
struct Malformed;

type Handled = Result<(), Malformed>;

fn int<T: FromStr>(s: &str) -> Result<T, Malformed> {
    s.trim().parse().map_err(|_| Malformed)
}

/// Accumulator for one SF section.
#[derive(Default)]
struct Record {
    path: String,
    lines: BTreeMap<LineNo, Count>,
    branches: BTreeMap<BranchKey, Option<Count>>,
    spans: HashMap<String, (LineNo, Option<LineNo>)>,
    hits: HashMap<String, Count>,
    fn_index: HashMap<String, (LineNo, Option<LineNo>)>,
}

impl Record {
    fn handle(&mut self, tag: &str, value: &str) -> Handled {
        match tag {
            "DA" => {
                let mut parts = value.split(',');
                let (Some(line), Some(count)) = (parts.next(), parts.next()) else {
                    return Err(Malformed);
                };
                *self.lines.entry(int(line)?).or_insert(0) += int::<Count>(count)?;
            }
            "BRDA" => {
                // Branch ids may contain commas (lcov 2.x expressions), so split from both ends.
                let mut parts = value.splitn(3, ',');
                let (Some(line), Some(block), Some(rest)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    return Err(Malformed);
                };
                let (branch, taken) = rest.rsplit_once(',').ok_or(Malformed)?;
                let taken = if taken == "-" { None } else { Some(int(taken)?) };
                let key = BranchKey {
                    line: int(line)?,
                    block: block.to_string(),
                    branch: branch.to_string(),
                };
                self.branches
                    .entry(key)
                    .and_modify(|t| *t = add_taken(*t, taken))
                    .or_insert(taken);
            }
            "FN" => {
                let (start, rest) = value.split_once(',').unwrap_or((value, ""));
                match rest.split_once(',') {
                    // lcov 2.x: FN:<start>,<end>,<name>
                    Some((end, name)) if !end.is_empty() && end.bytes().all(|b| b.is_ascii_digit()) => {
                        self.span(name, int(start)?, Some(int(end)?))?
                    }
                    _ => self.span(rest, int(start)?, None)?,
                }
            }
            "FNDA" => {
                let (count, name) = value.split_once(',').unwrap_or((value, ""));
                self.hit(name, int(count)?)?;
            }
            "FNL" => {
                let parts: Vec<&str> = value.split(',').collect();
                let end = match parts.len() {
                    2 => None,
                    3 => Some(int(parts[2])?),
                    _ => return Err(Malformed),
                };
                self.fn_index
                    .insert(parts[0].to_string(), (int(parts[1])?, end));
            }
            "FNA" => {
                let mut parts = value.splitn(3, ',');
                let (Some(index), Some(count), Some(name)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    return Err(Malformed);
                };
                let &(start, end) = self.fn_index.get(index).ok_or(Malformed)?;
                self.span(name, start, end)?;
                self.hit(name, int(count)?)?;
            }
            _ => unreachable!("unhandled tag {tag}"),
        }
        Ok(())
    }

    fn span(&mut self, name: &str, start: LineNo, end: Option<LineNo>) -> Handled {
        if name.is_empty() {
            return Err(Malformed);
        }
        let span = self.spans.entry(name.to_string()).or_insert((start, end));
        if span.1.is_none() {
            span.1 = end;
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

    /// FNDA records without a FN are dropped.
    fn build(self) -> FileCoverage {
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
        functions.sort_by(|a, b| (a.start, &a.name).cmp(&(b.start, &b.name)));
        FileCoverage {
            path: self.path,
            lines: self.lines,
            branches: self.branches,
            functions,
            external: false,
        }
    }
}

fn store(files: &mut BTreeMap<String, FileCoverage>, rec: Record) {
    let fc = rec.build();
    let merged = match files.remove(&fc.path) {
        None => fc,
        Some(old) => merge_file_coverage(&old, &fc),
    };
    files.insert(merged.path.clone(), merged);
}

/// Parse an lcov tracefile. Paths stay raw; duplicate SF sections are merged; unknown records
/// are skipped.
pub fn parse(text: &str) -> Result<Snapshot, String> {
    let mut files: BTreeMap<String, FileCoverage> = BTreeMap::new();
    let mut test_names: Vec<String> = Vec::new();
    let mut rec: Option<Record> = None;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    // Split on '\n' only: paths may hold other line-break characters.
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line == "end_of_record" {
            if let Some(r) = rec.take() {
                store(&mut files, r);
            }
            continue;
        }
        let Some((tag, value)) = line.split_once(':') else {
            continue;
        };
        if tag == "TN" {
            if !value.is_empty() && !test_names.iter().any(|n| n == value) {
                test_names.push(value.to_string());
            }
        } else if tag == "SF" {
            let new = Record {
                path: value.to_string(),
                ..Record::default()
            };
            if let Some(r) = rec.replace(new) {
                store(&mut files, r);
            }
        } else if HANDLED.contains(&tag)
            && let Some(r) = rec.as_mut()
        {
            r.handle(tag, value)
                .map_err(|_| format!("lcov line {}: malformed {tag} record: {line:?}", i + 1))?;
        }
    }
    if let Some(r) = rec.take() {
        store(&mut files, r);
    }
    Ok(Snapshot { files, test_names })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> FileCoverage {
        let snap = parse(text).unwrap();
        assert_eq!(snap.files.len(), 1);
        snap.files.into_values().next().unwrap()
    }

    #[test]
    fn records_and_merge() {
        let text = "\u{feff}TN:t\nSF:a\nFN:1,3,f\nFN:5,g,h\nFNDA:2,f\nFNDA:1,ghost\n\
                    DA:1,1\nDA:1,2\nBRDA:1,0,a,b,-\nBRDA:1,0,a,b,3\nend_of_record\n\
                    SF:a\nFNL:0,7\nFNA:0,4,k\nDA:2,0\nend_of_record\n";
        let snap = parse(text).unwrap();
        let fc = &snap.files["a"];
        assert_eq!(snap.test_names, vec!["t"]);
        assert_eq!(fc.lines, [(1, 3), (2, 0)].into_iter().collect());
        let names: Vec<_> = fc.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["f", "g,h", "k"]);
        assert_eq!(fc.functions[0].end, Some(3));
        assert_eq!(fc.functions[1].end, None);
        assert_eq!(fc.functions[2].hits, Some(4));
        assert_eq!(fc.branches.values().next(), Some(&Some(3)));
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
            assert!(err.starts_with("lcov line 2: malformed"), "{err}");
        }
    }

    #[test]
    fn skips_unknown_and_summary_records() {
        let fc = one("VER:2\nSF:a\nXYZ:1\nLF:9\nLH:9\nbogus\nDA:1,1\nend_of_record\n");
        assert_eq!(fc.lines, [(1, 1)].into_iter().collect());
        // Records before any SF are ignored.
        assert!(parse("DA:1,1\n").unwrap().files.is_empty());
    }

    #[test]
    fn branch_ids_with_commas() {
        let fc = one("SF:a\nBRDA:3,e0,(x, y),2\nBRDA:3,e0,(x, y),-\nend_of_record\n");
        let (key, taken) = fc.branches.iter().next().unwrap();
        assert_eq!((key.line, key.block.as_str()), (3, "e0"));
        assert_eq!(key.branch, "(x, y)");
        assert_eq!(*taken, Some(2));
    }

    #[test]
    fn dash_plus_dash_stays_dash() {
        let fc = one("SF:a\nBRDA:1,0,0,-\nBRDA:1,0,0,-\nend_of_record\n");
        assert_eq!(fc.branches.values().next(), Some(&None));
    }

    #[test]
    fn functions_fn_without_fnda_has_no_hits() {
        let fc = one("SF:a\nFN:2,f\nFN:4,7,g\nFNDA:0,g\nend_of_record\n");
        assert_eq!(fc.functions[0].hits, None);
        assert_eq!(fc.functions[1].hits, Some(0));
        assert_eq!(fc.functions[1].end, Some(7));
    }

    #[test]
    fn huge_and_negative_counts() {
        let fc = one("SF:a\nDA:1,18446744073709551615\nDA:2,-1\nend_of_record\n");
        assert_eq!(fc.lines[&1], 18446744073709551615);
        assert_eq!(fc.lines[&2], -1);
    }

    #[test]
    fn crlf_and_blank_lines() {
        let fc = one("SF:a\r\n\r\nDA:1,1\r\nend_of_record\r\n");
        assert_eq!(fc.path, "a");
        assert_eq!(fc.lines[&1], 1);
    }

    #[test]
    fn missing_end_of_record_still_counts() {
        assert_eq!(one("SF:a\nDA:1,0\n").lines[&1], 0);
    }

    #[test]
    fn test_names_dedup() {
        let snap = parse("TN:a\nTN:\nTN:a\nTN:b\n").unwrap();
        assert_eq!(snap.test_names, ["a", "b"]);
    }
}
