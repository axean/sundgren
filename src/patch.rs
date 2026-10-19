//! Git-style unified diff parser.
//!
//! Every delimiter is ASCII, so byte offsets from `find`/`strip_prefix` are char boundaries.

use std::collections::BTreeSet;

use crate::model::LineNo;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: LineNo,
    pub old_count: LineNo,
    pub new_start: LineNo,
    pub new_count: LineNo,
    /// Body lines with their ' ', '-' or '+' prefix; '\ No newline' dropped.
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePatch {
    /// None for a new file; first path component stripped.
    pub old_path: Option<String>,
    /// None for a deleted file; first path component stripped.
    pub new_path: Option<String>,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
}

impl FilePatch {
    pub fn path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .expect("a file patch has a path")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    pub files: Vec<FilePatch>,
    pub warnings: Vec<String>,
}

impl Patch {
    /// Sorted old and new paths of every file in the patch.
    pub fn touched_paths(&self) -> Vec<String> {
        let paths: BTreeSet<&String> = self
            .files
            .iter()
            .flat_map(|f| [f.old_path.as_ref(), f.new_path.as_ref()])
            .flatten()
            .collect();
        paths.into_iter().cloned().collect()
    }
}

/// Parse a git-style unified diff. Malformed input is an error naming its 1-based line.
pub fn parse_patch(text: &str) -> Result<Patch, String> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let mut p = Parser {
        lines,
        i: 0,
        files: Vec::new(),
        warnings: Vec::new(),
        seen: BTreeSet::new(),
    };
    while let Some(line) = p.peek(0) {
        p.step(line)?;
    }
    if p.files.is_empty() && p.lines.iter().any(|l| !l.trim().is_empty()) {
        p.warnings.push("no file diffs found in patch".into());
    }
    Ok(Patch {
        files: p.files,
        warnings: p.warnings,
    })
}

const HEADER_END: [&str; 5] = [
    "diff --git ",
    "--- ",
    "@@ ",
    "Binary files ",
    "GIT binary patch",
];

type Res<T> = Result<T, String>;

struct Parser<'a> {
    lines: Vec<&'a str>,
    i: usize,
    files: Vec<FilePatch>,
    warnings: Vec<String>,
    seen: BTreeSet<String>,
}

impl<'a> Parser<'a> {
    fn peek(&self, offset: usize) -> Option<&'a str> {
        self.lines.get(self.i + offset).copied()
    }

    fn peek_or_empty(&self, offset: usize) -> &'a str {
        self.peek(offset).unwrap_or("")
    }

    fn error_at(&self, msg: impl std::fmt::Display, line: usize) -> String {
        format!("line {line}: {msg}")
    }

    fn error(&self, msg: impl std::fmt::Display) -> String {
        self.error_at(msg, self.i + 1)
    }

    fn quote_error(&self, raw: &str) -> String {
        self.error(format!("bad quoted path {raw:?}"))
    }

    fn step(&mut self, line: &'a str) -> Res<()> {
        let start = self.i + 1;
        if line.starts_with("diff --git ") {
            let fp = self.git_file()?;
            self.add(fp, start)
        } else if line.starts_with("diff --cc ") || line.starts_with("diff --combined ") {
            Err(self.error("combined diffs are not supported"))
        } else if line.starts_with("--- ")
            && self.peek_or_empty(1).starts_with("+++ ")
            && self.peek_or_empty(2).starts_with("@@ ")
        {
            let fp = self.plain_file()?;
            self.add(fp, start)
        } else if let Some((a, b)) = match_binary(rstrip_cr(line)) {
            let old = side_path(a).map_err(|r| self.quote_error(r))?;
            let new = side_path(b).map_err(|r| self.quote_error(r))?;
            let (old, new) = plain_names(old, new);
            self.i += 1;
            let fp = self.binary(old, new, start)?;
            self.add(fp, start)
        } else {
            self.i += 1; // preamble, trailers, `diff -r` noise
            Ok(())
        }
    }

    fn add(&mut self, fp: FilePatch, start: usize) -> Res<()> {
        // One git diff names each target once; a repeat means a concatenated
        // series, whose line numbers chain and can't be keyed by path.
        if let Some(new) = &fp.new_path
            && !self.seen.insert(new.clone())
        {
            let msg = format!("{new} is patched twice (patch series?); use one combined diff");
            return Err(self.error_at(msg, start));
        }
        self.files.push(fp);
        Ok(())
    }

    fn binary(&mut self, old: Option<String>, new: Option<String>, start: usize) -> Res<FilePatch> {
        if old.is_none() && new.is_none() {
            return Err(self.error_at("binary diff without a path", start));
        }
        let fp = FilePatch {
            old_path: old,
            new_path: new,
            hunks: Vec::new(),
            binary: true,
        };
        self.warnings
            .push(format!("{}: binary diff ignored", fp.path()));
        Ok(fp)
    }

    fn git_file(&mut self) -> Res<FilePatch> {
        let start = self.i + 1;
        let header = rstrip_cr(&self.lines[self.i]["diff --git ".len()..]);
        let (mut old, mut new) = git_header_paths(header).map_err(|r| self.quote_error(r))?;
        let (mut created, mut deleted, mut copied) = (false, false, false);
        self.i += 1;
        // Extended header; unknown lines are skipped, as git apply does.
        let mut line = rstrip_cr(self.peek_or_empty(0));
        while self.i < self.lines.len() && !HEADER_END.iter().any(|p| line.starts_with(p)) {
            if line.starts_with("new file mode ") {
                created = true;
            } else if line.starts_with("deleted file mode ") {
                deleted = true;
            } else if line.starts_with("rename from ") || line.starts_with("copy from ") {
                let raw = after_second_space(line);
                old = Some(unquote(raw).ok_or_else(|| self.quote_error(raw))?);
                copied = line.starts_with("copy");
            } else if line.starts_with("rename to ") || line.starts_with("copy to ") {
                let raw = after_second_space(line);
                new = Some(unquote(raw).ok_or_else(|| self.quote_error(raw))?);
            }
            self.i += 1;
            line = rstrip_cr(self.peek_or_empty(0));
        }
        if created {
            old = None;
        }
        if deleted {
            new = None;
        }
        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            self.i += 1;
            if line.starts_with("GIT binary patch") {
                while self.peek(0).is_some_and(|l| !l.starts_with("diff --git ")) {
                    self.i += 1;
                }
            }
            return self.binary(old, new, start);
        }
        if line.starts_with("@@ ") {
            return Err(self.error("hunk without '---'/'+++' lines"));
        }
        let mut hunks = Vec::new();
        if line.starts_with("--- ") {
            let (minus, plus) = self.side_paths()?;
            // The mode lines win over '---'/'+++', as in git apply.
            old = if created { None } else { minus };
            new = if deleted { None } else { plus };
            hunks = self.hunks(or_path(&new, &old))?;
        }
        if old.is_none() && new.is_none() {
            return Err(self.error_at("cannot determine file path", start));
        }
        if copied {
            let name = |p: &Option<String>| p.clone().unwrap_or_else(|| "None".into());
            self.warnings.push(format!(
                "{}: copy of {} treated as a rename",
                name(&new),
                name(&old)
            ));
        }
        self.checked(
            FilePatch {
                old_path: old,
                new_path: new,
                hunks,
                binary: false,
            },
            start,
        )
    }

    fn plain_file(&mut self) -> Res<FilePatch> {
        let start = self.i + 1;
        let (minus, plus) = self.side_paths()?;
        let (old, new) = plain_names(minus, plus);
        if old.is_none() && new.is_none() {
            return Err(self.error_at("both sides are /dev/null", start));
        }
        let hunks = self.hunks(or_path(&new, &old))?;
        self.checked(
            FilePatch {
                old_path: old,
                new_path: new,
                hunks,
                binary: false,
            },
            start,
        )
    }

    fn side_paths(&mut self) -> Res<(Option<String>, Option<String>)> {
        if !self.peek_or_empty(1).starts_with("+++ ") {
            return Err(self.error("'---' line not followed by '+++'"));
        }
        let old = side_path(&self.lines[self.i][4..]).map_err(|r| self.quote_error(r))?;
        self.i += 1;
        let new = side_path(&self.lines[self.i][4..]).map_err(|r| self.quote_error(r))?;
        self.i += 1;
        Ok((old, new))
    }

    fn checked(&self, fp: FilePatch, start: usize) -> Res<FilePatch> {
        if fp.old_path.is_none() && fp.hunks.iter().any(|h| h.old_count != 0) {
            return Err(self.error_at("new file has a hunk with old lines", start));
        }
        if fp.new_path.is_none() && fp.hunks.iter().any(|h| h.new_count != 0) {
            return Err(self.error_at("deleted file has a hunk with new lines", start));
        }
        Ok(fp)
    }

    fn hunks(&mut self, path: String) -> Res<Vec<Hunk>> {
        let mut hunks = Vec::new();
        // First line not yet covered by a hunk.
        let (mut old_end, mut new_end) = (1, 1);
        let mut delta = 0;
        while self.peek(0).is_some_and(|l| l.starts_with("@@ ")) {
            let lineno = self.i + 1;
            let hunk = self.hunk()?;
            // A zero-length range starts at the line before it.
            let old_first = hunk.old_start + LineNo::from(hunk.old_count == 0);
            let new_first = hunk.new_start + LineNo::from(hunk.new_count == 0);
            if old_first < old_end || new_first < new_end {
                return Err(self.error_at("hunk overlaps or precedes the previous one", lineno));
            }
            if new_first - old_first != delta {
                self.warnings.push(format!(
                    "{path}: hunk at line {lineno} is offset from its header"
                ));
            }
            old_end = old_first + hunk.old_count;
            new_end = new_first + hunk.new_count;
            delta += hunk.new_count - hunk.old_count;
            hunks.push(hunk);
        }
        let line = rstrip_cr(self.peek_or_empty(0));
        // '--- ' may start the next plain diff; '-- ' and '--' start mail signatures.
        if line.starts_with([' ', '+', '-'])
            && !line.starts_with("--- ")
            && !line.starts_with("-- ")
            && line != "--"
        {
            return Err(self.error("hunk has more lines than its header says"));
        }
        Ok(hunks)
    }

    fn hunk(&mut self) -> Res<Hunk> {
        let Some([old_start, old_count, new_start, new_count]) = match_hunk(self.lines[self.i])
        else {
            return Err(self.error("malformed hunk header"));
        };
        if (old_count != 0 && old_start == 0) || (new_count != 0 && new_start == 0) {
            return Err(self.error("hunk range starts at line 0"));
        }
        self.i += 1;
        let mut body = Vec::new();
        let (mut old_left, mut new_left) = (old_count, new_count);
        while old_left != 0 || new_left != 0 {
            let Some(line) = self.peek(0) else {
                return Err(self.error_at("patch ends inside a hunk", self.lines.len()));
            };
            let text = match line.as_bytes().first() {
                Some(b'\\') => {
                    self.i += 1;
                    continue;
                }
                // Some tools strip the space off empty context.
                None | Some(b' ') => {
                    old_left -= 1;
                    new_left -= 1;
                    format!(" {}", line.get(1..).unwrap_or(""))
                }
                Some(b'\r') if line.len() == 1 => {
                    old_left -= 1;
                    new_left -= 1;
                    " ".to_string()
                }
                Some(b'-') => {
                    old_left -= 1;
                    line.to_string()
                }
                Some(b'+') => {
                    new_left -= 1;
                    line.to_string()
                }
                _ => return Err(self.error("unexpected line in hunk")),
            };
            if old_left < 0 || new_left < 0 {
                return Err(self.error("hunk has more lines than its header says"));
            }
            body.push(text);
            self.i += 1;
        }
        if self.peek(0).is_some_and(|l| l.starts_with('\\')) {
            self.i += 1;
        }
        Ok(Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            lines: body,
        })
    }
}

/// `new or old or ""`: an empty path counts as missing.
fn or_path(new: &Option<String>, old: &Option<String>) -> String {
    [new, old]
        .into_iter()
        .flatten()
        .find(|p| !p.is_empty())
        .cloned()
        .unwrap_or_default()
}

fn rstrip_cr(line: &str) -> &str {
    line.trim_end_matches('\r')
}

/// The text after the second space, for a line known to have two.
fn after_second_space(line: &str) -> &str {
    line.splitn(3, ' ').nth(2).unwrap_or("")
}

/// `@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@`, anchored at the start. Counts default to 1.
fn match_hunk(line: &str) -> Option<[LineNo; 4]> {
    let mut rest = line.strip_prefix("@@ -")?;
    let (old_start, old_count) = range(&mut rest)?;
    rest = rest.strip_prefix(" +")?;
    let (new_start, new_count) = range(&mut rest)?;
    rest.strip_prefix(" @@")?;
    Some([old_start, old_count, new_start, new_count])
}

fn range(rest: &mut &str) -> Option<(LineNo, LineNo)> {
    let start = number(rest)?;
    let count = match rest.strip_prefix(',') {
        Some(after) => {
            *rest = after;
            number(rest)?
        }
        None => 1,
    };
    Some((start, count))
}

fn number(rest: &mut &str) -> Option<LineNo> {
    let n = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (digits, after) = rest.split_at(n);
    *rest = after;
    digits.parse().ok()
}

/// `Binary files (.+) and (.+) differ`, full match; the first group is greedy.
fn match_binary(line: &str) -> Option<(&str, &str)> {
    let mid = line
        .strip_prefix("Binary files ")?
        .strip_suffix(" differ")?;
    let (a, b) = mid.rsplit_once(" and ")?;
    (!a.is_empty() && !b.is_empty()).then_some((a, b))
}

/// Paths from a `diff --git` header; the raw text on a bad quote.
#[allow(clippy::type_complexity)]
fn git_header_paths(rest: &str) -> Result<(Option<String>, Option<String>), &str> {
    let (old, tail): (String, &str);
    if rest.starts_with('"') {
        match quoted(rest) {
            Some((name, after)) if after.starts_with([' ', '\t']) => {
                old = name;
                tail = &after[1..];
            }
            _ => return Ok((None, None)),
        }
    } else if let Some(cut) = rest.find(" \"").filter(|_| rest.ends_with('"')) {
        old = rest[..cut].to_string();
        tail = &rest[cut + 1..];
    } else {
        // Unquoted names may hold spaces; like git, accept only "X/P Y/P".
        let name = strip(rest);
        for (i, c) in name.char_indices() {
            if (c == ' ' || c == '\t') && strip(&name[i + 1..]) == &name[..i] {
                let p = name[..i].to_string();
                return Ok((Some(p.clone()), Some(p)));
            }
        }
        return Ok((None, None));
    }
    let new = unquote(tail).ok_or(tail)?;
    Ok((Some(strip(&old).to_string()), Some(strip(&new).to_string())))
}

/// A `---`/`+++` path; None for /dev/null; the raw text on a bad quote.
fn side_path(raw: &str) -> Result<Option<String>, &str> {
    let raw = rstrip_cr(raw);
    let path = if raw.starts_with('"') {
        unquote(raw).ok_or(raw)?
    } else {
        raw.split('\t').next().unwrap_or("").to_string()
    };
    if path == "/dev/null" {
        return Ok(None);
    }
    Ok(Some(strip(&path).to_string()))
}

/// Drop the first path component like `git apply -p1`; a bare name stays.
fn strip(path: &str) -> &str {
    path.split_once('/').map_or(path, |(_, rest)| rest)
}

/// Plain diffs never rename: git apply uses the '+++' name, or '---' if it prefixes it.
fn plain_names(old: Option<String>, new: Option<String>) -> (Option<String>, Option<String>) {
    match (old, new) {
        (Some(old), Some(new)) => {
            let name = if new.starts_with(&old) { old } else { new };
            (Some(name.clone()), Some(name))
        }
        (old, new) => (old, new),
    }
}

/// The path as written, or decoded when C-quoted; None for a bad quote.
fn unquote(raw: &str) -> Option<String> {
    if !raw.starts_with('"') {
        return Some(raw.to_string());
    }
    quoted(raw).map(|(name, _)| name)
}

/// Decode a leading C-quoted string; return it and the text after the closing quote.
/// Non-UTF-8 bytes become U+FFFD.
fn quoted(raw: &str) -> Option<(String, &str)> {
    let b = raw.as_bytes();
    let mut out = Vec::new();
    let mut i = 1;
    while i < b.len() {
        match b[i] {
            b'"' => return Some((String::from_utf8_lossy(&out).into_owned(), &raw[i + 1..])),
            b'\\' => {
                let esc = match b.get(i + 1) {
                    Some(b'a') => Some(7),
                    Some(b'b') => Some(8),
                    Some(b't') => Some(9),
                    Some(b'n') => Some(10),
                    Some(b'v') => Some(11),
                    Some(b'f') => Some(12),
                    Some(b'r') => Some(13),
                    Some(b'"') => Some(b'"'),
                    Some(b'\\') => Some(b'\\'),
                    _ => None,
                };
                if let Some(byte) = esc {
                    out.push(byte);
                    i += 2;
                } else {
                    let oct = b
                        .get(i + 1..i + 4)
                        .filter(|o| o.iter().all(|c| (b'0'..=b'7').contains(c)))?;
                    let v = oct
                        .iter()
                        .fold(0u32, |acc, &d| acc * 8 + u32::from(d - b'0'));
                    out.push((v & 0xFF) as u8);
                    i += 4;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests;
