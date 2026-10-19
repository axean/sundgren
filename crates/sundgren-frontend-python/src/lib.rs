//! The Python frontend: IR for Python files, built by Python's own `ast`.
//!
//! The script (`src/sundgren/lang/python/ir_frontend.py`, embedded here) runs under the target
//! project's interpreter, so its grammar always matches. One process handles many files over
//! JSON lines; a file that fails doesn't stop the batch, and a crashed interpreter is replaced.

use std::ffi::OsStr;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread::{self, JoinHandle};

use serde::Deserialize;
use sundgren_core::pytext::PyText;
use sundgren_core::report_json::{Json, to_json_string};
use sundgren_ir::File;

/// The frontend script, run with `python -I -c SCRIPT --batch`.
pub const SCRIPT: &str = include_str!("../../../src/sundgren/lang/python/ir_frontend.py");

/// One file to parse: where to read it, and the repo-relative path to record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub file: PathBuf,
    pub path: String,
}

/// Why one file has no IR. The rest of the batch is unaffected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    /// The source does not parse (includes null bytes, bad encoding, too deep nesting).
    Syntax(String),
    /// The file could not be read.
    Unreadable(String),
    /// The script failed on this file in some other way.
    Failed(String),
    /// The interpreter died on this file; the stderr tail.
    Crashed(String),
    /// The IR does not match the file (`File::validate`) or the request.
    Invalid(String),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Syntax(m) => write!(f, "syntax error: {m}"),
            FileError::Unreadable(m) => write!(f, "unreadable: {m}"),
            FileError::Failed(m) => write!(f, "frontend failed: {m}"),
            FileError::Crashed(m) => write!(f, "interpreter crashed: {m}"),
            FileError::Invalid(m) => write!(f, "invalid IR: {m}"),
        }
    }
}

impl std::error::Error for FileError {}

/// The interpreter can't be used at all.
#[derive(Debug)]
pub enum RunError {
    Spawn(io::Error),
    /// It started but never said it was ready (too old, broken); its stderr.
    NotReady(String),
    Io(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Spawn(e) => write!(f, "cannot start the Python frontend: {e}"),
            RunError::NotReady(err) => write!(f, "the Python frontend did not start: {err}"),
            RunError::Io(e) => write!(f, "Python frontend I/O: {e}"),
        }
    }
}

impl std::error::Error for RunError {}

pub type FileResult = Result<File, FileError>;

/// Parse `requests` with the interpreter `python`. One result per request, in order.
pub fn parse_files(python: &OsStr, requests: &[Request]) -> Result<Vec<FileResult>, RunError> {
    let mut out = Vec::with_capacity(requests.len());
    while out.len() < requests.len() {
        // Each round makes progress: a crash consumes the file it happened on.
        out.extend(run_batch(python, &requests[out.len()..])?);
    }
    Ok(out)
}

#[derive(Deserialize)]
struct Head {
    ready: Option<String>,
    schema: Option<String>,
    path: Option<String>,
    error: Option<String>,
    kind: Option<String>,
}

struct Proc {
    child: Child,
    stdout: BufReader<ChildStdout>,
    writer: Option<JoinHandle<()>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
}

impl Proc {
    fn spawn(python: &OsStr, requests: &[Request]) -> Result<Proc, RunError> {
        let mut child = Command::new(python)
            .args(["-I", "-c", SCRIPT, "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(RunError::Spawn)?;
        let mut stdin = child.stdin.take().expect("piped");
        let lines: Vec<u8> = requests.iter().flat_map(request_line).collect();
        // A thread, so a full stdout pipe can't block us while we write.
        let writer = thread::spawn(move || {
            // The process may die early; then the rest isn't needed.
            let _ = stdin.write_all(&lines);
        });
        let mut err = child.stderr.take().expect("piped");
        let stderr = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = err.read_to_end(&mut buf);
            buf
        });
        let stdout = BufReader::new(child.stdout.take().expect("piped"));
        Ok(Proc {
            child,
            stdout,
            writer: Some(writer),
            stderr: Some(stderr),
        })
    }

    /// The next output line, or None at EOF.
    fn line(&mut self) -> Result<Option<String>, RunError> {
        let mut buf = Vec::new();
        let n = self
            .stdout
            .read_until(b'\n', &mut buf)
            .map_err(RunError::Io)?;
        if n == 0 || buf.last() != Some(&b'\n') {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
    }

    /// Wait for the process; its stderr, trimmed to the last lines.
    fn finish(mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(w) = self.writer.take() {
            let _ = w.join();
        }
        let err = self
            .stderr
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default();
        let text = String::from_utf8_lossy(&err);
        let lines: Vec<&str> = text.trim_end().lines().collect();
        lines[lines.len().saturating_sub(20)..].join("\n")
    }
}

fn request_line(r: &Request) -> Vec<u8> {
    let file = Json::Str(os_text(r.file.as_os_str()));
    let path = Json::Str(r.path.as_str().into());
    let line = Json::object([("file", file), ("path", path)]);
    let mut out = to_json_string(&line, None).into_bytes();
    out.push(b'\n');
    out
}

#[cfg(unix)]
fn os_text(s: &OsStr) -> PyText {
    use std::os::unix::ffi::OsStrExt;
    PyText::from_surrogateescape_bytes(s.as_bytes())
}

#[cfg(not(unix))]
fn os_text(s: &OsStr) -> PyText {
    PyText::from(s.to_string_lossy().as_ref())
}

/// Run one process over `requests` until it finishes them or crashes. Returns at least one
/// result.
fn run_batch(python: &OsStr, requests: &[Request]) -> Result<Vec<FileResult>, RunError> {
    let mut proc = Proc::spawn(python, requests)?;
    let ready = match proc.line()? {
        Some(line) => serde_json::from_str::<Head>(&line)
            .ok()
            .and_then(|h| h.ready)
            .is_some(),
        None => false,
    };
    if !ready {
        return Err(RunError::NotReady(proc.finish()));
    }
    let mut out = Vec::with_capacity(requests.len());
    for req in requests {
        match proc.line()? {
            Some(line) => out.push(file_result(req, &line)),
            None => {
                out.push(Err(FileError::Crashed(proc.finish())));
                return Ok(out);
            }
        }
    }
    proc.finish();
    Ok(out)
}

fn file_result(req: &Request, line: &str) -> FileResult {
    let head: Head = serde_json::from_str(line)
        .map_err(|e| FileError::Invalid(format!("bad output line: {e}")))?;
    if head.path.as_deref() != Some(req.path.as_str()) {
        return Err(FileError::Invalid(format!(
            "output for {:?}, expected {:?}",
            head.path, req.path
        )));
    }
    if let Some(msg) = head.error {
        return Err(match head.kind.as_deref() {
            Some("syntax") => FileError::Syntax(msg),
            Some("unreadable") => FileError::Unreadable(msg),
            _ => FileError::Failed(msg),
        });
    }
    if head.schema.is_none() {
        return Err(FileError::Invalid(
            "output line is neither IR nor an error".into(),
        ));
    }
    let file = File::from_json(line).map_err(|e| FileError::Invalid(e.to_string()))?;
    let source = std::fs::read(&req.file).map_err(|e| FileError::Unreadable(e.to_string()))?;
    file.validate(&source)
        .map_err(|e| FileError::Invalid(e.to_string()))?;
    Ok(file)
}

#[cfg(test)]
mod tests;
