//! Command line: diff, run, presets, ingest.

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

use crate::diff::{DiffOptions, diff};
use crate::linemap::{LineMaps, build_linemaps};
use crate::model::{Outcomes, Snapshot, Warning, snapshot_json, warning};
use crate::patch::{Patch, parse_patch};
use crate::run::presets::{Preset, adhoc_preset, all_presets};
use crate::run::{RunConfig, default_python, repo_root, run_pair};
use crate::{EXIT_OK, Error, Result, cancel, junit, lcov, normalize, report, sha256_file};

const PYTHON_MARKERS: [&str; 3] = ["pyproject.toml", "setup.cfg", "pytest.ini"];
const RUST_MARKERS: [&str; 1] = ["Cargo.toml"];

#[derive(Parser, Debug)]
#[command(
    name = "sundgren",
    version,
    about = "Command line: diff, run, presets, ingest.",
    infer_long_args = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// diff two existing lcov files
    #[command(infer_long_args = true)]
    Diff(DiffArgs),
    /// run coverage on base and patched trees, then diff
    #[command(infer_long_args = true)]
    Run(RunArgs),
    /// list built-in and user presets
    #[command(infer_long_args = true)]
    Presets {
        #[arg(long, value_name = "DIR", default_value = ".")]
        repo: String,
    },
    /// debug: print a normalized lcov Snapshot as JSON
    #[command(infer_long_args = true)]
    Ingest {
        #[arg(value_name = "FILE.lcov")]
        lcov: String,
        #[arg(long, value_name = "DIR")]
        root: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Text,
}

#[derive(Args, Debug)]
pub struct OutputArgs {
    #[arg(long, value_enum, default_value = "json")]
    pub format: Format,
    /// write the report here instead of stdout
    #[arg(short, long, value_name = "OUT")]
    pub output: Option<String>,
    /// include unchanged lines, functions, tests
    #[arg(long)]
    pub full: bool,
    /// fill files[].count_changed
    #[arg(long)]
    pub counts: bool,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    #[arg(value_name = "BASE.lcov")]
    pub base_lcov: String,
    #[arg(value_name = "PATCHED.lcov")]
    pub patched_lcov: String,
    /// patch between the two trees, for line mapping
    #[arg(long, value_name = "FILE")]
    pub patch: Option<String>,
    #[arg(long, value_name = "DIR")]
    pub base_root: Option<String>,
    #[arg(long, value_name = "DIR")]
    pub patched_root: Option<String>,
    #[arg(long, value_name = "PREFIX", num_args = 1..)]
    pub strip_prefix: Vec<String>,
    #[arg(long, value_name = "FILE")]
    pub base_junit: Option<String>,
    #[arg(long, value_name = "FILE")]
    pub patched_junit: Option<String>,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[arg(long, value_name = "FILE")]
    pub patch: String,
    #[arg(long, value_name = "NAME", conflicts_with = "cmd")]
    pub preset: Option<String>,
    /// ad-hoc command (repeatable)
    #[arg(long, value_name = "CMD")]
    pub cmd: Vec<String>,
    /// lcov output path template (with --cmd)
    #[arg(long, value_name = "TEMPLATE")]
    pub lcov: Option<String>,
    /// JUnit output path template (with --cmd)
    #[arg(long, value_name = "TEMPLATE")]
    pub junit: Option<String>,
    /// run --cmd via /bin/sh -c
    #[arg(long)]
    pub shell: bool,
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub repo: String,
    #[arg(long, value_name = "REF", default_value = "HEAD")]
    pub base: String,
    #[arg(long, value_name = "T", num_args = 1..)]
    pub tests: Vec<String>,
    #[arg(long, value_name = "PATH")]
    pub python: Option<String>,
    #[arg(long, value_name = "K=V")]
    pub env: Vec<String>,
    #[arg(
        long,
        value_name = "N",
        default_value_t = 1,
        allow_negative_numbers = true
    )]
    pub repeat: i64,
    #[arg(long, value_name = "DIR")]
    pub workdir: Option<String>,
    #[arg(long)]
    pub keep_worktrees: bool,
    #[command(flatten)]
    pub output: OutputArgs,
}

/// Parse the command line, run it and return the exit code.
pub fn main() -> i32 {
    let cli = Cli::parse();
    // Only run starts commands and leaves things to clean up; elsewhere a signal just kills us.
    let result = match cli.command {
        Command::Run(_) => cancel::install().and_then(|()| run(&cli)),
        _ => run(&cli),
    };
    // Whatever failed after a signal failed because of it.
    if let Some(sig) = cancel::signal() {
        eprintln!("sundgren: cancelled");
        return cancel::exit_code(sig);
    }
    match result {
        Ok(()) => EXIT_OK,
        Err(e) => {
            eprintln!("sundgren: {}", e.msg);
            e.code
        }
    }
}

/// Run one parsed command line.
pub fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Diff(a) => cmd_diff(a),
        Command::Run(a) => cmd_run(a),
        Command::Presets { repo } => cmd_presets(Path::new(repo)),
        Command::Ingest { lcov, root } => cmd_ingest(lcov, root.as_deref()),
    }
}

/// An optional argument, with "" counting as not given.
fn given(arg: &Option<String>) -> Option<&str> {
    arg.as_deref().filter(|s| !s.is_empty())
}

/// `Path(p).resolve()` as a string.
fn resolve(p: &str) -> String {
    normalize::realpath(p)
}

fn read_text(path: &Path, what: &str) -> Result<Vec<u8>> {
    std::fs::read(path)
        .map_err(|e| Error::usage(format!("cannot read {what} {}: {e}", path.display())))
}

fn load_snapshot(path: &Path, roots: &[String]) -> Result<Snapshot> {
    let bytes = read_text(path, "lcov")?;
    let raw = String::from_utf8(bytes)
        .map_err(|e| e.to_string())
        .and_then(|text| lcov::parse(&text))
        .map_err(|e| Error::failure(format!("{}: {e}", path.display())))?;
    Ok(normalize::normalize_snapshot(&raw, roots))
}

/// A JUnit file's outcomes; one that doesn't parse becomes a `junit_unreadable` warning.
pub fn load_junit(path: Option<&Path>, warnings: &mut Vec<Warning>) -> Result<Option<Outcomes>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let bytes = read_text(path, "JUnit file")?;
    let parsed = String::from_utf8(bytes)
        .map_err(|e| e.to_string())
        .and_then(|text| junit::parse_junit(&text));
    match parsed {
        Ok(outcomes) => Ok(Some(outcomes)),
        Err(msg) => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            warnings.push(warning("junit_unreadable", format!("{name}: {msg}")));
            Ok(None)
        }
    }
}

/// `meta.patch`: the path as given, the file's sha256, and the files the patch touches.
fn patch_meta(arg: &str, path: &Path, patch: &Patch) -> Result<Value> {
    Ok(json!({"path": arg, "sha256": sha256_file(path)?, "files": patch.touched_paths()}))
}

/// `meta` with every field null except the given ones.
fn meta(fields: Value) -> Value {
    let mut meta = json!({
        "sundgren_version": env!("CARGO_PKG_VERSION"),
        "created_utc": utc_now(),
        "mode": null,
        "base_ref": null,
        "patch": null,
        "preset": null,
        "commands": null,
        "selection": null,
        "repeat": null,
        "inputs": null,
    });
    if let (Some(m), Value::Object(f)) = (meta.as_object_mut(), fields) {
        m.extend(f);
    }
    meta
}

/// The current UTC time as `%Y-%m-%dT%H:%M:%SZ`.
fn utc_now() -> String {
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
    let (h, min, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}Z")
}

/// Write the report as JSON or text, to `-o` or stdout.
fn emit(report: &Value, out: &OutputArgs, hints: &[String]) -> Result<()> {
    let text = match out.format {
        Format::Json => report::dumps(report),
        Format::Text => report::render_text(report, hints),
    };
    match &out.output {
        Some(path) => std::fs::write(path, text)?,
        None => std::io::stdout().write_all(text.as_bytes())?,
    }
    Ok(())
}

fn cmd_diff(args: &DiffArgs) -> Result<()> {
    let base_root = args.base_root.as_deref().map(resolve);
    let patched_root = args.patched_root.as_deref().map(resolve);
    let roots = |root: &Option<String>| -> Vec<String> {
        root.iter().chain(&args.strip_prefix).cloned().collect()
    };
    let (base_path, patched_path) = (Path::new(&args.base_lcov), Path::new(&args.patched_lcov));
    let base = load_snapshot(base_path, &roots(&base_root))?;
    let patched = load_snapshot(patched_path, &roots(&patched_root))?;

    let mut warnings = Vec::new();
    let mut linemaps = LineMaps::new();
    let mut patch_info = Value::Null;
    if let Some(arg) = &args.patch {
        let path = Path::new(arg);
        let bytes = read_text(path, "patch")?;
        // Only paths matter: other bytes that aren't UTF-8 are harmless.
        let patch = parse_patch(&String::from_utf8_lossy(&bytes))
            .map_err(|e| Error::failure(format!("{arg}: {e}")))?;
        warnings.extend(patch.warnings.iter().map(|w| warning("patch", w.clone())));
        linemaps = build_linemaps(&patch);
        patch_info = patch_meta(arg, path, &patch)?;
    }
    let base_junit = load_junit(args.base_junit.as_deref().map(Path::new), &mut warnings)?;
    let patched_junit = load_junit(args.patched_junit.as_deref().map(Path::new), &mut warnings)?;

    let (base_root, patched_root) = (
        base_root.map(PathBuf::from),
        patched_root.map(PathBuf::from),
    );
    let opts = DiffOptions {
        base_root: base_root.as_deref(),
        patched_root: patched_root.as_deref(),
        base_outcomes: base_junit.as_ref(),
        patched_outcomes: patched_junit.as_ref(),
    };
    let delta = diff(&[base], &[patched], &linemaps, opts)?;
    let meta = meta(json!({
        "mode": "diff",
        "patch": patch_info,
        "inputs": {
            "base_lcov_sha256": [sha256_file(base_path)?],
            "patched_lcov_sha256": [sha256_file(patched_path)?],
        },
    }));
    let o = &args.output;
    let report = report::build_report(&delta, meta, o.full, o.counts, &warnings);
    emit(&report, o, &[])
}

fn cmd_run(args: &RunArgs) -> Result<()> {
    if args.repeat < 1 {
        return Err(Error::usage("--repeat must be >= 1"));
    }
    if args.cmd.is_empty()
        && (given(&args.lcov).is_some() || given(&args.junit).is_some() || args.shell)
    {
        return Err(Error::usage(
            "--lcov, --junit and --shell only apply with --cmd",
        ));
    }
    let patch_path = PathBuf::from(resolve(&args.patch));
    if !patch_path.is_file() {
        return Err(Error::usage(format!("patch not found: {}", args.patch)));
    }
    let repo = repo_root(Path::new(&args.repo))
        .map_err(|e| Error::usage(format!("not a git repository: {} ({})", args.repo, e.msg)))?;
    let preset = resolve_preset(args, &repo)?;
    let cfg = RunConfig {
        repo,
        base: args.base.clone(),
        patch_path: patch_path.clone(),
        workdir: given(&args.workdir).map(|w| PathBuf::from(resolve(w))),
        keep_worktrees: args.keep_worktrees,
        preset: preset.clone(),
        python: given(&args.python).map_or_else(default_python, str::to_string),
        env: parse_env(&args.env)?,
        tests: args.tests.clone(),
        repeat: args.repeat,
    };
    let run = run_pair(&cfg)?;
    let meta = meta(json!({
        "mode": "run",
        "base_ref": run.base_ref,
        "patch": patch_meta(&args.patch, &patch_path, &run.patch)?,
        "preset": preset.name,
        "commands": run.commands,
        "selection": {
            "tests": run.selection.tests,
            "run_all": run.selection.run_all,
            "reason": run.selection.reason,
        },
        "repeat": args.repeat,
        "inputs": {"base_lcov_sha256": run.base_sha, "patched_lcov_sha256": run.patched_sha},
    }));
    let o = &args.output;
    let report = report::build_report(&run.delta, meta, o.full, o.counts, &run.warnings);
    emit(&report, o, &preset.hints)
}

fn resolve_preset(args: &RunArgs, repo: &Path) -> Result<Preset> {
    if !args.cmd.is_empty() {
        let lcov = given(&args.lcov).ok_or_else(|| Error::usage("--cmd needs --lcov"))?;
        return Ok(adhoc_preset(
            args.cmd.clone(),
            lcov.to_string(),
            args.junit.clone(),
            args.shell,
        ));
    }
    let mut presets = all_presets(repo)?;
    let name = match given(&args.preset) {
        Some(n) => n.to_string(),
        None => default_preset_name(repo)?,
    };
    presets.remove(&name).ok_or_else(|| {
        let known: Vec<&str> = presets.keys().map(String::as_str).collect();
        Error::usage(format!(
            "unknown preset {name:?} (known: {})",
            known.join(", ")
        ))
    })
}

fn default_preset_name(repo: &Path) -> Result<String> {
    let python = PYTHON_MARKERS.iter().any(|m| repo.join(m).exists());
    let rust = RUST_MARKERS.iter().any(|m| repo.join(m).exists());
    match (python, rust) {
        (true, false) => Ok("pytest".into()),
        (false, true) => Ok("cargo-llvm-cov".into()),
        _ => Err(Error::usage(
            "can't pick a default preset here; pass --preset or --cmd",
        )),
    }
}

/// `--env K=V` items in order; a repeated key keeps its first position and last value.
fn parse_env(items: &[String]) -> Result<Vec<(String, String)>> {
    let mut env: Vec<(String, String)> = Vec::new();
    for item in items {
        let Some((key, value)) = item.split_once('=').filter(|(k, _)| !k.is_empty()) else {
            return Err(Error::usage(format!("--env expects K=V, got {item:?}")));
        };
        match env.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.to_string(),
            None => env.push((key.to_string(), value.to_string())),
        }
    }
    Ok(env)
}

fn cmd_presets(repo: &Path) -> Result<()> {
    let mut out = String::new();
    for (name, p) in all_presets(repo)? {
        let selector = p
            .selector
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("-");
        out.push_str(&format!("{name}  [{}]  {}\n", p.source, p.description));
        out.push_str(&format!(
            "    tests: {}, selector: {selector}\n",
            p.tests_style.as_str()
        ));
        for cmd in &p.commands {
            out.push_str(&format!("    $ {cmd}\n"));
        }
    }
    std::io::stdout().write_all(out.as_bytes())?;
    Ok(())
}

fn cmd_ingest(lcov: &str, root: Option<&str>) -> Result<()> {
    let roots: Vec<String> = root.map(resolve).into_iter().collect();
    let snapshot = load_snapshot(Path::new(lcov), &roots)?;
    std::io::stdout().write_all(report::dumps(&snapshot_json(&snapshot)).as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests;
