//! Command handlers: diff, run, presets, ingest. Port of cli.py's handlers.
//!
//! Argument parsing stays with the caller (argparse in cli.py for now), which fills [`Args`].
//! Everything the caller's language provides (registries, `which`, the TOML loader, the version
//! and the output streams) comes in through [`Host`].

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use sundgren_core::diff::{Delta, DiffError, DiffOptions, diff};
use sundgren_core::linemap::{LineMap, build_linemaps};
use sundgren_core::model::{Outcome, ReportWarning, Snapshot};
use sundgren_core::patch::{Patch, parse_patch};
use sundgren_core::pytext::PyText;
use sundgren_core::symbols::{Symbolizer, SymbolizerError};
use sundgren_core::{hashing, junit, lcov, normalize, report_json, report_text};
use sundgren_run::presets::{Preset, PresetError, TomlLoader, all_presets};
use sundgren_run::pypath::py_str;
use sundgren_run::selectors::Selector;

pub const EXIT_OK: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_PATCH: i32 = 3;

pub const PYTHON_MARKERS: [&str; 3] = ["pyproject.toml", "setup.cfg", "pytest.ini"];
pub const RUST_MARKERS: [&str; 1] = ["Cargo.toml"];

/// An error the caller raises as it is (Python: an uncaught exception).
pub type HostError = Box<dyn std::error::Error + Send + Sync>;

/// Report format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputArgs {
    pub format: Format,
    /// Write the report here instead of stdout.
    pub output: Option<String>,
    pub full: bool,
    pub counts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffArgs {
    pub base_lcov: String,
    pub patched_lcov: String,
    pub patch: Option<String>,
    pub base_root: Option<String>,
    pub patched_root: Option<String>,
    pub strip_prefix: Vec<String>,
    pub base_junit: Option<String>,
    pub patched_junit: Option<String>,
    pub output: OutputArgs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunArgs {
    pub patch: String,
    pub preset: Option<String>,
    /// `--cmd`, repeatable; None when not given.
    pub cmd: Option<Vec<String>>,
    pub lcov: Option<String>,
    pub junit: Option<String>,
    pub shell: bool,
    pub repo: String,
    pub base: String,
    pub tests: Vec<String>,
    pub python: Option<String>,
    /// `--env K=V` items, unparsed.
    pub env: Vec<String>,
    pub repeat: i64,
    pub workdir: Option<String>,
    pub keep_worktrees: bool,
    pub output: OutputArgs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetsArgs {
    pub repo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestArgs {
    pub lcov: String,
    pub root: Option<String>,
}

/// One parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Args {
    Diff(DiffArgs),
    Run(RunArgs),
    Presets(PresetsArgs),
    Ingest(IngestArgs),
}

/// What the caller's language provides.
pub trait Host {
    /// `sys.stdout.write(text)`.
    fn write_stdout(&mut self, text: &PyText) -> Result<(), HostError>;
    /// `print(text, file=sys.stderr)`, newline included in `text`.
    fn write_stderr(&mut self, text: &str) -> Result<(), HostError>;
    /// The installed sundgren version.
    fn version(&self) -> Result<String, HostError>;
    /// Presets registered by the language packs.
    fn builtin_presets(&self) -> Result<BTreeMap<String, Preset>, HostError>;
    fn toml_loader(&self) -> &dyn TomlLoader;
    /// The registered selector for an id.
    fn selector(&self, id: &str) -> Option<Box<dyn Selector + '_>>;
    /// The registered symbolizer for a repo-relative path.
    fn symbolizer(&self, path: &PyText) -> Option<Box<dyn Symbolizer>>;
    /// `shutil.which(name)`.
    fn which(&self, name: &str) -> Option<String>;
}

/// A user-facing failure: printed as `sundgren: {message}`, exit `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub message: String,
    pub code: i32,
}

impl CliError {
    pub fn new(message: impl Into<String>, code: i32) -> Self {
        CliError {
            message: message.into(),
            code,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Cli(CliError),
    Host(HostError),
}

impl From<CliError> for Error {
    fn from(e: CliError) -> Self {
        Error::Cli(e)
    }
}

impl From<HostError> for Error {
    fn from(e: HostError) -> Self {
        Error::Host(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Host(Box::new(e))
    }
}

pub type CliResult<T> = Result<T, Error>;

/// Run one command; a `CliError` is printed to stderr and becomes the exit code.
pub fn main(args: Args, host: &mut dyn Host) -> Result<i32, HostError> {
    let result = match &args {
        Args::Diff(a) => cmd_diff(a, host),
        Args::Run(a) => crate::run::cmd_run(a, host),
        Args::Presets(a) => cmd_presets(a, host),
        Args::Ingest(a) => cmd_ingest(a, host),
    };
    match result {
        Ok(code) => Ok(code),
        Err(Error::Cli(e)) => {
            host.write_stderr(&format!("sundgren: {}\n", e.message))?;
            Ok(e.code)
        }
        Err(Error::Host(e)) => Err(e),
    }
}

// --- shared helpers ---------------------------------------------------------

/// A `read_text` failure: the OS error, or the UTF-8 decode error (a ValueError in Python).
#[derive(Debug)]
pub enum ReadError {
    Os(io::Error),
    Decode(String),
}

/// `Path.read_text(encoding="utf-8")`: strict UTF-8, universal newlines.
pub fn read_text(path: &Path) -> Result<String, ReadError> {
    let bytes = std::fs::read(path).map_err(ReadError::Os)?;
    let text = String::from_utf8(bytes).map_err(|e| {
        let bytes = e.as_bytes();
        ReadError::Decode(utf8_error_message(bytes, e.utf8_error()))
    })?;
    if !text.contains('\r') {
        return Ok(text);
    }
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Python's UnicodeDecodeError text for strict UTF-8.
pub fn utf8_error_message(bytes: &[u8], err: std::str::Utf8Error) -> String {
    let start = err.valid_up_to();
    let (len, reason) = match err.error_len() {
        None => (bytes.len() - start, "unexpected end of data"),
        Some(n) => {
            let b = bytes[start];
            let bad_start = (0x80..0xC2).contains(&b) || b >= 0xF5;
            (
                n,
                if bad_start {
                    "invalid start byte"
                } else {
                    "invalid continuation byte"
                },
            )
        }
    };
    if len == 1 {
        format!(
            "'utf-8' codec can't decode byte 0x{:02x} in position {start}: {reason}",
            bytes[start]
        )
    } else {
        format!(
            "'utf-8' codec can't decode bytes in position {start}-{}: {reason}",
            start + len - 1
        )
    }
}

/// `exc.strerror` of an OS error.
pub fn strerror(e: &io::Error) -> String {
    let text = e.to_string();
    match e.raw_os_error() {
        Some(code) => text
            .strip_suffix(&format!(" (os error {code})"))
            .unwrap_or(&text)
            .to_owned(),
        None => text,
    }
}

/// `_read_text`: an OS error is a usage error naming `what`; a decode error stays a ValueError.
pub fn read_text_or_usage(path: &Path, what: &str) -> CliResult<Result<String, String>> {
    match read_text(path) {
        Ok(text) => Ok(Ok(text)),
        Err(ReadError::Decode(msg)) => Ok(Err(msg)),
        Err(ReadError::Os(e)) => Err(CliError::new(
            format!("cannot read {what} {}: {}", py_str(path), strerror(&e)),
            EXIT_USAGE,
        )
        .into()),
    }
}

/// Parse and normalize an lcov file; a bad file exits 1.
pub fn load_snapshot<S: AsRef<str>>(path: &Path, roots: &[S]) -> CliResult<Snapshot> {
    let parsed = read_text_or_usage(path, "lcov")?
        .and_then(|text| lcov::parse(&text).map_err(|e| e.to_string()));
    match parsed {
        Ok(raw) => Ok(normalize::normalize_snapshot(&raw, roots)),
        Err(msg) => Err(CliError::new(format!("{}: {msg}", py_str(path)), EXIT_FAILURE).into()),
    }
}

/// Parse a JUnit file; an unreadable one becomes a `junit_unreadable` warning.
pub fn load_junit(
    path: Option<&Path>,
    warnings: &mut Vec<ReportWarning>,
) -> CliResult<Option<BTreeMap<String, Outcome>>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let parsed = read_text_or_usage(path, "JUnit file")?
        .and_then(|text| junit::parse_junit(&text).map_err(|e| e.to_string()));
    match parsed {
        Ok(outcomes) => Ok(Some(outcomes)),
        Err(msg) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            warnings.push(warning("junit_unreadable", &format!("{name}: {msg}")));
            Ok(None)
        }
    }
}

pub fn warning(code: &str, message: &str) -> ReportWarning {
    ReportWarning {
        code: code.to_owned(),
        message: PyText::from(message.to_owned()),
    }
}

/// A patch with its line maps and parse warnings.
pub type LoadedPatch = (Patch, BTreeMap<PyText, LineMap>, Vec<ReportWarning>);

/// Read a patch file as bytes, decoded with surrogateescape, and parse it.
pub fn load_patch(path: &Path) -> CliResult<LoadedPatch> {
    let bytes = std::fs::read(path).map_err(|e| {
        CliError::new(
            format!("cannot read patch {}: {}", py_str(path), strerror(&e)),
            EXIT_USAGE,
        )
    })?;
    parse_patch_text(&PyText::from_surrogateescape_bytes(&bytes), &py_str(path))
}

/// Parse patch text; malformed input exits 1, naming `place`.
pub fn parse_patch_text(text: &PyText, place: &str) -> CliResult<LoadedPatch> {
    let patch =
        parse_patch(text).map_err(|e| CliError::new(format!("{place}: {e}"), EXIT_FAILURE))?;
    let warnings = patch
        .warnings
        .iter()
        .map(|w| ReportWarning {
            code: "patch".into(),
            message: w.clone(),
        })
        .collect();
    let linemaps = build_linemaps(&patch);
    Ok((patch, linemaps, warnings))
}

/// `meta.patch`: the path as given, the file's sha256, and the files the patch touches.
pub fn patch_meta(arg: &str, path: &Path, patch: &Patch) -> CliResult<Json> {
    Ok(Json::object([
        ("path", Json::from(arg)),
        ("sha256", Json::from(hashing::sha256_file(path)?)),
        ("files", Json::from(patch.touched_paths())),
    ]))
}

/// `meta` with every field null except the given ones, in the prototype's key order.
pub fn meta(host: &dyn Host, fields: Vec<(&str, Json)>) -> CliResult<Json> {
    let mut items: Vec<(&str, Json)> = vec![
        ("sundgren_version", Json::from(host.version()?)),
        ("created_utc", Json::from(utc_now())),
    ];
    for key in [
        "mode",
        "base_ref",
        "patch",
        "preset",
        "commands",
        "selection",
        "repeat",
        "inputs",
    ] {
        items.push((key, Json::Null));
    }
    for (k, v) in fields {
        match items.iter_mut().find(|(key, _)| *key == k) {
            Some(slot) => slot.1 = v,
            None => items.push((k, v)),
        }
    }
    Ok(Json::object(items))
}

/// The current UTC time as `%Y-%m-%dT%H:%M:%SZ`.
pub fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    format_utc(secs)
}

fn format_utc(secs: i64) -> String {
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Write the report as JSON or text, to `-o` or stdout.
pub fn emit(
    host: &mut dyn Host,
    report: &Json,
    out: &OutputArgs,
    hints: &[String],
) -> CliResult<()> {
    let text: PyText = match out.format {
        Format::Json => report_json::dumps(report).into(),
        Format::Text => {
            let hints: Vec<PyText> = hints.iter().map(|h| PyText::from(h.clone())).collect();
            report_text::render_text(report, &hints)
                .map_err(|e| Error::Host(e.to_string().into()))?
        }
    };
    match &out.output {
        Some(path) => {
            let text = text.as_str().ok_or_else(|| {
                Error::Host("'utf-8' codec can't encode a surrogate in the report".into())
            })?;
            std::fs::write(path, text)?
        }
        None => host.write_stdout(&text)?,
    }
    Ok(())
}

/// `Path(p).resolve()` as a string.
pub fn resolve(p: &str) -> String {
    let abs = sundgren_run::pypath::absolute(Path::new(p))
        .map_or_else(|_| p.to_owned(), |a| a.to_string_lossy().into_owned());
    normalize::realpath(&abs)
}

/// Build the report for a delta.
pub fn build_report(
    delta: &Delta,
    meta: Json,
    out: &OutputArgs,
    extra_warnings: &[ReportWarning],
) -> Json {
    report_json::build_report(delta, meta, out.full, out.counts, extra_warnings)
}

pub use sundgren_core::report_json::Json;

// --- diff -------------------------------------------------------------------

fn cmd_diff(args: &DiffArgs, host: &mut dyn Host) -> CliResult<i32> {
    let base_root = args.base_root.as_deref().map(resolve);
    let patched_root = args.patched_root.as_deref().map(resolve);
    let roots = |root: &Option<String>| -> Vec<String> {
        root.iter().chain(&args.strip_prefix).cloned().collect()
    };
    let (base_path, patched_path) = (Path::new(&args.base_lcov), Path::new(&args.patched_lcov));
    let base = load_snapshot(base_path, &roots(&base_root))?;
    let patched = load_snapshot(patched_path, &roots(&patched_root))?;

    let mut warnings = Vec::new();
    let mut linemaps = BTreeMap::new();
    let mut patch_meta_value = Json::Null;
    if let Some(arg) = &args.patch {
        let (patch, maps, patch_warnings) = load_patch(Path::new(arg))?;
        linemaps = maps;
        warnings.extend(patch_warnings);
        patch_meta_value = patch_meta(arg, Path::new(arg), &patch)?;
    }
    let base_junit = load_junit(args.base_junit.as_deref().map(Path::new), &mut warnings)?;
    let patched_junit = load_junit(args.patched_junit.as_deref().map(Path::new), &mut warnings)?;

    let base_root = base_root.map(PathBuf::from);
    let patched_root = patched_root.map(PathBuf::from);
    let opts = DiffOptions {
        base_root: base_root.as_deref(),
        patched_root: patched_root.as_deref(),
        base_outcomes: base_junit.as_ref(),
        patched_outcomes: patched_junit.as_ref(),
    };
    let delta = diff_snapshots(host, &[base], &[patched], &linemaps, opts)?;
    let inputs = Json::object([
        (
            "base_lcov_sha256",
            Json::from(vec![hashing::sha256_file(base_path)?]),
        ),
        (
            "patched_lcov_sha256",
            Json::from(vec![hashing::sha256_file(patched_path)?]),
        ),
    ]);
    let meta = meta(
        host,
        vec![
            ("mode", Json::from("diff")),
            ("patch", patch_meta_value),
            ("inputs", inputs),
        ],
    )?;
    let report = build_report(&delta, meta, &args.output, &warnings);
    emit(host, &report, &args.output, &[])?;
    Ok(EXIT_OK)
}

/// `diff(...)` with the host's symbolizers.
pub fn diff_snapshots(
    host: &dyn Host,
    base: &[Snapshot],
    patched: &[Snapshot],
    linemaps: &BTreeMap<PyText, LineMap>,
    opts: DiffOptions<'_>,
) -> CliResult<Delta> {
    let lookup = |path: &PyText| host.symbolizer(path);
    diff(base, patched, linemaps, opts, &lookup).map_err(|e| match e {
        DiffError::Io(e) => e.into(),
        DiffError::Symbolizer(SymbolizerError::Other(e)) => Error::Host(e),
        other => Error::Host(other.to_string().into()),
    })
}

// --- presets / ingest -------------------------------------------------------

/// `all_presets(repo)`; a malformed presets file is a usage error.
pub fn load_all_presets(host: &dyn Host, repo: &Path) -> CliResult<BTreeMap<String, Preset>> {
    let builtins = host.builtin_presets()?;
    match all_presets(&builtins, repo, host.toml_loader()) {
        Ok(p) => Ok(p),
        Err(PresetError::Invalid(msg)) => Err(CliError::new(msg, EXIT_USAGE).into()),
        Err(PresetError::Host(e)) => Err(Error::Host(e)),
    }
}

fn cmd_presets(args: &PresetsArgs, host: &mut dyn Host) -> CliResult<i32> {
    let presets = load_all_presets(host, Path::new(&args.repo))?;
    let mut out = String::new();
    for (name, p) in &presets {
        out.push_str(&format!("{name}  [{}]  {}\n", p.source, p.description));
        out.push_str(&format!(
            "    tests: {}, selector: {}\n",
            p.tests_style.as_str(),
            p.selector
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or("-")
        ));
        for cmd in &p.commands {
            out.push_str(&format!("    $ {cmd}\n"));
        }
    }
    host.write_stdout(&PyText::from(out))?;
    Ok(EXIT_OK)
}

fn cmd_ingest(args: &IngestArgs, host: &mut dyn Host) -> CliResult<i32> {
    let roots: Vec<String> = args.root.as_deref().map(resolve).into_iter().collect();
    let snapshot = load_snapshot(Path::new(&args.lcov), &roots)?;
    let dict = sundgren_core::model::snapshot_to_dict(&snapshot);
    host.write_stdout(&PyText::from(report_json::dumps(&Json::from(&dict))))?;
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_format() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_792_454_399), "2026-10-19T23:59:59Z");
    }

    #[test]
    fn decode_errors_like_python() {
        let msg = |b: &[u8]| utf8_error_message(b, std::str::from_utf8(b).unwrap_err());
        assert_eq!(
            msg(b"ab\xff"),
            "'utf-8' codec can't decode byte 0xff in position 2: invalid start byte"
        );
        assert_eq!(
            msg(b"\xe2\x82"),
            "'utf-8' codec can't decode bytes in position 0-1: unexpected end of data"
        );
        assert_eq!(
            msg(b"\xe2\x82A"),
            "'utf-8' codec can't decode bytes in position 0-1: invalid continuation byte"
        );
        assert_eq!(
            msg(b"\xe2A"),
            "'utf-8' codec can't decode byte 0xe2 in position 0: invalid continuation byte"
        );
    }
}
