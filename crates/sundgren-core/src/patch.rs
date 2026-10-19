//! Git-style unified diff parser. Port of src/sundgren/core/patch.py.
//!
//! Works on the generalized UTF-8 bytes of a `PyText`. Every delimiter is ASCII, so byte
//! slicing agrees with Python's code point slicing.

use std::collections::BTreeSet;
use std::fmt;

pub use crate::model::LineNo;
use crate::pytext::PyText;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hunk {
    pub old_start: LineNo,
    pub old_count: LineNo,
    pub new_start: LineNo,
    pub new_count: LineNo,
    /// Body lines with their ' ', '-' or '+' prefix; '\ No newline' dropped.
    pub lines: Vec<PyText>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FilePatch {
    /// None for a new file; first path component stripped.
    pub old_path: Option<PyText>,
    /// None for a deleted file; first path component stripped.
    pub new_path: Option<PyText>,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
}

impl FilePatch {
    pub fn path(&self) -> &PyText {
        self.new_path
            .as_ref()
            .or(self.old_path.as_ref())
            .expect("a file patch has a path")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Patch {
    pub files: Vec<FilePatch>,
    pub warnings: Vec<PyText>,
}

impl Patch {
    /// Sorted old and new paths of every file in the patch.
    pub fn touched_paths(&self) -> Vec<PyText> {
        let paths: BTreeSet<&PyText> = self
            .files
            .iter()
            .flat_map(|f| [f.old_path.as_ref(), f.new_path.as_ref()])
            .flatten()
            .collect();
        paths.into_iter().cloned().collect()
    }
}

/// Malformed input, at a 1-based line. Python raises it as `ValueError`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchError {
    At {
        line: usize,
        msg: PyText,
    },
    /// `bad quoted path {raw!r}`; the bindings format `raw` with Python's `repr`.
    BadQuotedPath {
        line: usize,
        raw: PyText,
    },
}

impl PatchError {
    pub fn line(&self) -> usize {
        match self {
            Self::At { line, .. } | Self::BadQuotedPath { line, .. } => *line,
        }
    }
}

/// Matches Python's message, except that `repr` of a bad quoted path is approximated.
impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::At { line, msg } => write!(f, "line {line}: {msg}"),
            Self::BadQuotedPath { line, raw } => {
                write!(f, "line {line}: bad quoted path {}", py_repr_approx(raw))
            }
        }
    }
}

impl std::error::Error for PatchError {}

/// Parse a git-style unified diff; error on malformed input.
pub fn parse_patch(text: &PyText) -> Result<Patch, PatchError> {
    Parser::new(text.as_bytes()).parse()
}

const HEADER_END: [&[u8]; 5] = [
    b"diff --git ",
    b"--- ",
    b"@@ ",
    b"Binary files ",
    b"GIT binary patch",
];

type Res<T> = Result<T, PatchError>;

struct BadQuote(PyText);

struct Parser<'a> {
    lines: Vec<&'a [u8]>,
    i: usize,
    files: Vec<FilePatch>,
    warnings: Vec<PyText>,
    seen: BTreeSet<PyText>,
}

impl<'a> Parser<'a> {
    fn new(text: &'a [u8]) -> Self {
        let mut lines: Vec<&[u8]> = text.split(|&b| b == b'\n').collect();
        if lines.last() == Some(&&b""[..]) {
            lines.pop();
        }
        Self {
            lines,
            i: 0,
            files: Vec::new(),
            warnings: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    fn peek(&self, offset: usize) -> Option<&'a [u8]> {
        self.lines.get(self.i + offset).copied()
    }

    fn peek_or_empty(&self, offset: usize) -> &'a [u8] {
        self.peek(offset).unwrap_or(b"")
    }

    fn error_at(&self, msg: impl Into<PyText>, line: usize) -> PatchError {
        PatchError::At {
            line,
            msg: msg.into(),
        }
    }

    fn error(&self, msg: impl Into<PyText>) -> PatchError {
        self.error_at(msg, self.i + 1)
    }

    fn quote_error(&self, e: BadQuote) -> PatchError {
        PatchError::BadQuotedPath {
            line: self.i + 1,
            raw: e.0,
        }
    }

    fn parse(mut self) -> Res<Patch> {
        while let Some(line) = self.peek(0) {
            self.step(line)?;
        }
        if self.files.is_empty() && self.lines.iter().any(|l| !is_py_blank(l)) {
            self.warnings.push("no file diffs found in patch".into());
        }
        Ok(Patch {
            files: self.files,
            warnings: self.warnings,
        })
    }

    fn step(&mut self, line: &'a [u8]) -> Res<()> {
        let start = self.i + 1;
        if line.starts_with(b"diff --git ") {
            let fp = self.git_file()?;
            self.add(fp, start)
        } else if line.starts_with(b"diff --cc ") || line.starts_with(b"diff --combined ") {
            Err(self.error("combined diffs are not supported"))
        } else if line.starts_with(b"--- ")
            && self.peek_or_empty(1).starts_with(b"+++ ")
            && self.peek_or_empty(2).starts_with(b"@@ ")
        {
            let fp = self.plain_file()?;
            self.add(fp, start)
        } else if let Some((a, b)) = match_binary(rstrip_cr(line)) {
            let old = side_path(a).map_err(|e| self.quote_error(e))?;
            let new = side_path(b).map_err(|e| self.quote_error(e))?;
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
        if let Some(new) = &fp.new_path {
            if self.seen.contains(new) {
                let mut msg = new.clone();
                msg.push_str(" is patched twice (patch series?); use one combined diff");
                return Err(self.error_at(msg, start));
            }
            self.seen.insert(new.clone());
        }
        self.files.push(fp);
        Ok(())
    }

    fn binary(&mut self, old: Option<PyText>, new: Option<PyText>, start: usize) -> Res<FilePatch> {
        if old.is_none() && new.is_none() {
            return Err(self.error_at("binary diff without a path", start));
        }
        let fp = FilePatch {
            old_path: old,
            new_path: new,
            hunks: Vec::new(),
            binary: true,
        };
        let mut w = fp.path().clone();
        w.push_str(": binary diff ignored");
        self.warnings.push(w);
        Ok(fp)
    }

    fn git_file(&mut self) -> Res<FilePatch> {
        let start = self.i + 1;
        let header = rstrip_cr(&self.lines[self.i][b"diff --git ".len()..]);
        let (mut old, mut new) = git_header_paths(header).map_err(|e| self.quote_error(e))?;
        let (mut created, mut deleted, mut copied) = (false, false, false);
        self.i += 1;
        // Extended header; unknown lines are skipped, as git apply does.
        let mut line = rstrip_cr(self.peek_or_empty(0));
        while self.i < self.lines.len() && !HEADER_END.iter().any(|p| line.starts_with(p)) {
            if line.starts_with(b"new file mode ") {
                created = true;
            } else if line.starts_with(b"deleted file mode ") {
                deleted = true;
            } else if line.starts_with(b"rename from ") || line.starts_with(b"copy from ") {
                old = Some(unquote(after_second_space(line)).map_err(|e| self.quote_error(e))?);
                copied = line.starts_with(b"copy");
            } else if line.starts_with(b"rename to ") || line.starts_with(b"copy to ") {
                new = Some(unquote(after_second_space(line)).map_err(|e| self.quote_error(e))?);
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
        if line.starts_with(b"Binary files ") || line.starts_with(b"GIT binary patch") {
            self.i += 1;
            if line.starts_with(b"GIT binary patch") {
                while let Some(nxt) = self.peek(0) {
                    if nxt.starts_with(b"diff --git ") {
                        break;
                    }
                    self.i += 1;
                }
            }
            return self.binary(old, new, start);
        }
        if line.starts_with(b"@@ ") {
            return Err(self.error("hunk without '---'/'+++' lines"));
        }
        let mut hunks = Vec::new();
        if line.starts_with(b"--- ") {
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
            let mut w = py_str_or_none(&new);
            w.push_str(": copy of ");
            w.push_text(&py_str_or_none(&old));
            w.push_str(" treated as a rename");
            self.warnings.push(w);
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

    fn side_paths(&mut self) -> Res<(Option<PyText>, Option<PyText>)> {
        if !self.peek_or_empty(1).starts_with(b"+++ ") {
            return Err(self.error("'---' line not followed by '+++'"));
        }
        let old = side_path(&self.lines[self.i][4..]).map_err(|e| self.quote_error(e))?;
        self.i += 1;
        let new = side_path(&self.lines[self.i][4..]).map_err(|e| self.quote_error(e))?;
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

    fn hunks(&mut self, path: PyText) -> Res<Vec<Hunk>> {
        let mut hunks = Vec::new();
        // First line not yet covered by a hunk.
        let (mut old_end, mut new_end) = (1i64, 1i64);
        let mut delta = 0i64;
        while self.peek(0).is_some_and(|l| l.starts_with(b"@@ ")) {
            let lineno = self.i + 1;
            let hunk = self.hunk()?;
            // A zero-length range starts at the line before it.
            let old_first = hunk.old_start + i64::from(hunk.old_count == 0);
            let new_first = hunk.new_start + i64::from(hunk.new_count == 0);
            if old_first < old_end || new_first < new_end {
                return Err(self.error_at("hunk overlaps or precedes the previous one", lineno));
            }
            if new_first - old_first != delta {
                let mut w = path.clone();
                w.push_str(&format!(
                    ": hunk at line {lineno} is offset from its header"
                ));
                self.warnings.push(w);
            }
            old_end = old_first + hunk.old_count;
            new_end = new_first + hunk.new_count;
            delta += hunk.new_count - hunk.old_count;
            hunks.push(hunk);
        }
        let line = rstrip_cr(self.peek_or_empty(0));
        // '--- ' may start the next plain diff; '-- ' and '--' start mail signatures.
        if matches!(line.first(), Some(b' ' | b'+' | b'-'))
            && !line.starts_with(b"--- ")
            && !line.starts_with(b"-- ")
            && line != b"--"
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
            let text = match line.first() {
                Some(b'\\') => {
                    self.i += 1;
                    continue;
                }
                // Some tools strip the space off empty context.
                None | Some(b' ') => {
                    old_left -= 1;
                    new_left -= 1;
                    with_space(line)
                }
                Some(b'\r') if line.len() == 1 => {
                    old_left -= 1;
                    new_left -= 1;
                    with_space(line)
                }
                Some(b'-') => {
                    old_left -= 1;
                    line.to_vec()
                }
                Some(b'+') => {
                    new_left -= 1;
                    line.to_vec()
                }
                _ => return Err(self.error("unexpected line in hunk")),
            };
            if old_left < 0 || new_left < 0 {
                return Err(self.error("hunk has more lines than its header says"));
            }
            body.push(PyText::from_bytes_unchecked(text));
            self.i += 1;
        }
        if self.peek(0).is_some_and(|l| l.starts_with(b"\\")) {
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

/// `" " + line[1:]`.
fn with_space(line: &[u8]) -> Vec<u8> {
    let mut out = vec![b' '];
    out.extend_from_slice(line.get(1..).unwrap_or(b""));
    out
}

/// `new or old or ""`: an empty path counts as missing, as in Python.
fn or_path(new: &Option<PyText>, old: &Option<PyText>) -> PyText {
    [new, old]
        .into_iter()
        .flatten()
        .find(|p| !p.is_empty())
        .cloned()
        .unwrap_or_default()
}

/// A path, or `None` as Python's f-string writes it.
fn py_str_or_none(p: &Option<PyText>) -> PyText {
    p.clone().unwrap_or_else(|| "None".into())
}

/// `line.rstrip("\r")`.
fn rstrip_cr(mut line: &[u8]) -> &[u8] {
    while let [rest @ .., b'\r'] = line {
        line = rest;
    }
    line
}

/// `line.split(" ", 2)[2]` for a line with at least two spaces.
fn after_second_space(line: &[u8]) -> &[u8] {
    let first = line
        .iter()
        .position(|&b| b == b' ')
        .expect("prefix has a space");
    let second = first
        + 1
        + line[first + 1..]
            .iter()
            .position(|&b| b == b' ')
            .expect("two spaces");
    &line[second + 1..]
}

/// `@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@`, anchored at the start. Counts default to 1.
fn match_hunk(line: &[u8]) -> Option<[LineNo; 4]> {
    let mut rest = line.strip_prefix(b"@@ -")?;
    let (old_start, old_count) = range(&mut rest)?;
    rest = rest.strip_prefix(b" +")?;
    let (new_start, new_count) = range(&mut rest)?;
    rest.strip_prefix(b" @@")?;
    Some([old_start, old_count, new_start, new_count])
}

fn range(rest: &mut &[u8]) -> Option<(LineNo, LineNo)> {
    let start = number(rest)?;
    let count = match rest.strip_prefix(b",") {
        Some(after) => {
            *rest = after;
            number(rest)?
        }
        None => 1,
    };
    Some((start, count))
}

/// ASCII digits. Python's `\d` also takes other Unicode digits, and its ints don't overflow;
/// both only matter for nonsense headers, which then read as malformed.
fn number(rest: &mut &[u8]) -> Option<LineNo> {
    let n = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    if n == 0 {
        return None;
    }
    let (digits, after) = rest.split_at(n);
    *rest = after;
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// `Binary files (.+) and (.+) differ`, full match; the first group is greedy.
fn match_binary(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let mid = line
        .strip_prefix(b"Binary files ")?
        .strip_suffix(b" differ")?;
    let sep = b" and ";
    (1..mid.len().saturating_sub(sep.len()))
        .rev()
        .find(|&p| &mid[p..p + sep.len()] == sep)
        .map(|p| (&mid[..p], &mid[p + sep.len()..]))
}

fn git_header_paths(rest: &[u8]) -> Result<(Option<PyText>, Option<PyText>), BadQuote> {
    let (old, tail): (PyText, &[u8]);
    if rest.starts_with(b"\"") {
        match quoted(rest) {
            Some((name, after)) if after.starts_with(b" ") || after.starts_with(b"\t") => {
                old = name;
                tail = &after[1..];
            }
            _ => return Ok((None, None)),
        }
    } else if rest.ends_with(b"\"") && find(rest, b" \"").is_some() {
        let cut = find(rest, b" \"").expect("checked");
        old = bytes_text(&rest[..cut]);
        tail = &rest[cut + 1..];
    } else {
        // Unquoted names may hold spaces; like git, accept only "X/P Y/P".
        let name = strip(rest);
        for (i, &c) in name.iter().enumerate() {
            if (c == b' ' || c == b'\t') && strip(&name[i + 1..]) == &name[..i] {
                let p = bytes_text(&name[..i]);
                return Ok((Some(p.clone()), Some(p)));
            }
        }
        return Ok((None, None));
    }
    let new = unquote(tail)?;
    Ok((
        Some(bytes_text(strip(old.as_bytes()))),
        Some(bytes_text(strip(new.as_bytes()))),
    ))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn bytes_text(b: &[u8]) -> PyText {
    PyText::from_bytes_unchecked(b.to_vec())
}

fn side_path(raw: &[u8]) -> Result<Option<PyText>, BadQuote> {
    let raw = rstrip_cr(raw);
    let path = if raw.starts_with(b"\"") {
        unquote(raw)?
    } else {
        bytes_text(raw.split(|&b| b == b'\t').next().unwrap_or(b""))
    };
    if path == "/dev/null" {
        return Ok(None);
    }
    Ok(Some(bytes_text(strip(path.as_bytes()))))
}

/// Drop the first path component like `git apply -p1`; a bare name stays.
fn strip(path: &[u8]) -> &[u8] {
    match path.iter().position(|&b| b == b'/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// Plain diffs never rename: git apply uses the '+++' name, or '---' if it prefixes it.
fn plain_names(old: Option<PyText>, new: Option<PyText>) -> (Option<PyText>, Option<PyText>) {
    match (old, new) {
        (Some(old), Some(new)) => {
            let name = if new.starts_with(old.as_bytes()) {
                old
            } else {
                new
            };
            (Some(name.clone()), Some(name))
        }
        (old, new) => (old, new),
    }
}

fn unquote(raw: &[u8]) -> Result<PyText, BadQuote> {
    if !raw.starts_with(b"\"") {
        return Ok(bytes_text(raw));
    }
    quoted(raw)
        .map(|(name, _)| name)
        .ok_or_else(|| BadQuote(bytes_text(raw)))
}

/// Decode a leading C-quoted string; return it and the text after the closing quote.
fn quoted(raw: &[u8]) -> Option<(PyText, &[u8])> {
    let mut out = Vec::new();
    let mut i = 1;
    while i < raw.len() {
        let c = raw[i];
        if c == b'"' {
            return Some((PyText::from_surrogateescape_bytes(&out), &raw[i + 1..]));
        }
        if c != b'\\' {
            // c.encode("utf-8", "surrogateescape"), one code point at a time.
            let n = utf8_len(c);
            let chunk = PyText::from_bytes_unchecked(raw[i..i + n].to_vec());
            out.extend(chunk.encode_surrogateescape()?);
            i += n;
            continue;
        }
        let esc = raw.get(i + 1).copied();
        if let Some(byte) = esc.and_then(escape) {
            out.push(byte);
            i += 2;
        } else {
            let oct = raw
                .get(i + 1..i + 4)
                .filter(|o| o.iter().all(|b| (b'0'..=b'7').contains(b)))?;
            let v = oct
                .iter()
                .fold(0u32, |acc, &d| acc * 8 + u32::from(d - b'0'));
            out.push((v & 0xFF) as u8);
            i += 4;
        }
    }
    None
}

fn escape(c: u8) -> Option<u8> {
    Some(match c {
        b'a' => 7,
        b'b' => 8,
        b't' => 9,
        b'n' => 10,
        b'v' => 11,
        b'f' => 12,
        b'r' => 13,
        b'"' => 34,
        b'\\' => 92,
        _ => return None,
    })
}

fn utf8_len(lead: u8) -> usize {
    match lead {
        0xF0..=0xF7 => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

/// `not line.strip()`, with Python's notion of whitespace.
fn is_py_blank(line: &[u8]) -> bool {
    bytes_text(line).code_points().all(|cp| {
        char::from_u32(cp).is_some_and(|c| c.is_whitespace() || ('\x1c'..='\x1f').contains(&c))
    })
}

/// Close to Python's `repr` of a str: single quotes, ASCII escapes, `\udcXX` for surrogates.
fn py_repr_approx(t: &PyText) -> String {
    let mut out = String::from("'");
    for cp in t.code_points() {
        match char::from_u32(cp) {
            Some('\'') => out.push_str("\\'"),
            Some('\\') => out.push_str("\\\\"),
            Some('\t') => out.push_str("\\t"),
            Some('\n') => out.push_str("\\n"),
            Some('\r') => out.push_str("\\r"),
            Some(c) if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            Some(c) if c.is_ascii() || !c.is_control() => out.push(c),
            Some(c) => out.push_str(&format!("\\x{:02x}", c as u32)),
            None => out.push_str(&format!("\\u{cp:04x}")),
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Patch, PatchError> {
        parse_patch(&PyText::from(s))
    }

    #[test]
    fn simple_git_diff() {
        let p =
            parse("diff --git a/f.py b/f.py\n--- a/f.py\n+++ b/f.py\n@@ -1 +1,2 @@\n-a\n+b\n+c\n")
                .unwrap();
        assert_eq!(p.files.len(), 1);
        let f = &p.files[0];
        assert_eq!(f.old_path.as_ref().unwrap(), "f.py");
        assert_eq!(f.hunks[0].new_count, 2);
        assert_eq!(f.hunks[0].lines, ["-a", "+b", "+c"].map(PyText::from));
    }

    #[test]
    fn errors_have_line_numbers() {
        let e = parse("--- a/f\n+++ b/f\n@@ -1 +1 @@\n?x\n").unwrap_err();
        assert_eq!(e.to_string(), "line 4: unexpected line in hunk");
        let e = parse("diff --git a/f b/f\nrename from \"x\\q\"\n").unwrap_err();
        assert_eq!(e.to_string(), "line 2: bad quoted path '\"x\\\\q\"'");
    }

    #[test]
    fn quoted_octal_bytes() {
        let p = parse("diff --git \"a/caf\\303\\251\" \"b/caf\\303\\251\"\nold mode 1\n").unwrap();
        assert_eq!(p.files[0].path(), "café");
        let p = parse("diff --git \"a/\\377\" \"b/\\377\"\nold mode 1\n").unwrap();
        assert_eq!(
            p.files[0].path().encode_surrogateescape().unwrap(),
            b"\xff".to_vec()
        );
    }

    #[test]
    fn binary_line_split_is_greedy() {
        assert_eq!(
            match_binary(b"Binary files a and b and c differ"),
            Some((&b"a and b"[..], &b"c"[..]))
        );
        assert_eq!(match_binary(b"Binary files  and x differ"), None);
    }

    #[test]
    fn blank_text_has_no_warning() {
        assert!(parse(" \u{3000}\n\x1c\n").unwrap().warnings.is_empty());
        assert_eq!(
            parse("x\n").unwrap().warnings,
            [PyText::from("no file diffs found in patch")]
        );
    }
}
