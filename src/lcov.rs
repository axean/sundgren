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
                let taken = if taken == "-" {
                    None
                } else {
                    Some(int(taken)?)
                };
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
                    Some((end, name))
                        if !end.is_empty() && end.bytes().all(|b| b.is_ascii_digit()) =>
                    {
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
        let span = self.spans.entry(demangle(name)).or_insert((start, end));
        if span.1.is_none() {
            span.1 = end;
        }
        Ok(())
    }

    fn hit(&mut self, name: &str, count: Count) -> Handled {
        if name.is_empty() {
            return Err(Malformed);
        }
        *self.hits.entry(demangle(name)).or_insert(0) += count;
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

/// A Rust symbol (v0 `_R...` or legacy `_ZN...E`) as a path without hashes or generic arguments;
/// other names as they are. The instances of a generic function then share a name and merge,
/// like same-named functions.
fn demangle(name: &str) -> String {
    if !name.starts_with("_R") && !name.starts_with("_ZN") {
        return name.to_owned();
    }
    match rustc_demangle::try_demangle(name) {
        Ok(d) => strip_generics(&format!("{d:#}")),
        Err(_) => name.to_owned(),
    }
}

/// Drop generic argument lists: `f::<u8>` -> `f`, `<Wrapper<u8> as Trait>` -> `<Wrapper as Trait>`.
fn strip_generics(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut depth = 0usize;
    let mut prev = ' ';
    for c in path.chars() {
        if depth > 0 {
            match c {
                '<' => depth += 1,
                '>' if prev != '-' => depth -= 1,
                _ => {}
            }
        } else if c == '<' && out.ends_with("::") {
            out.truncate(out.len() - 2);
            depth = 1;
        } else if c == '<' && out.ends_with(|p: char| p.is_alphanumeric() || p == '_') {
            depth = 1;
        } else {
            out.push(c);
        }
        prev = c;
    }
    out
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
mod tests;
