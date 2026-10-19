//! The run pipeline: worktrees at the base, preset commands on both sides, then diff.
//!
//! Shared by the CLI's `run` and by verification. Errors are typed; the CLI turns them into its
//! messages and exit codes.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use sundgren_core::diff::{Delta, DiffError, DiffOptions, SymbolizerLookup, diff};
use sundgren_core::linemap::{LineMap, build_linemaps};
use sundgren_core::model::{Outcome, ReportWarning, Snapshot};
use sundgren_core::patch::{Patch, parse_patch};
use sundgren_core::pytext::PyText;
use sundgren_core::symbols::SymbolizerError;
use sundgren_core::{hashing, junit, lcov, normalize};

use crate::guards::wrong_tree_paths;
use crate::presets::{Preset, TestsStyle};
use crate::pypath::py_str;
use crate::runner::{RunError, TemplateRunner};
use crate::selectors::{ChooseError, HostError, Selection, Selector, choose, restrict_to_tree};
use crate::text::{ReadError, read_text};
use crate::workspace::{Workspace, WorkspaceError, is_dirty};

pub type LineMaps = BTreeMap<PyText, LineMap>;
pub type Outcomes = BTreeMap<String, Outcome>;
pub type SelectorLookup<'a> = &'a dyn Fn(&str) -> Option<Box<dyn Selector + 'a>>;

pub fn warning(code: &str, message: &str) -> ReportWarning {
    ReportWarning {
        code: code.into(),
        message: message.into(),
    }
}

/// Parse a patch and build its line maps; patch warnings become `patch` report warnings.
pub fn load_patch(text: &PyText) -> Result<(Patch, LineMaps, Vec<ReportWarning>), String> {
    let patch = parse_patch(text).map_err(|e| e.to_string())?;
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

/// Parse a JUnit file; an unparsable one becomes a `junit_unreadable` warning. Err is the OS
/// error of reading it.
pub fn load_junit(
    path: Option<&Path>,
    warnings: &mut Vec<ReportWarning>,
) -> Result<Option<Outcomes>, io::Error> {
    let Some(path) = path else {
        return Ok(None);
    };
    let parsed = match read_text(path) {
        Ok(text) => junit::parse_junit(&text).map_err(|e| e.to_string()),
        Err(ReadError::Decode(msg)) => Err(msg),
        Err(ReadError::Os(e)) => return Err(e),
    };
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

/// What to run and where.
pub struct PairConfig<'a> {
    /// Repository root.
    pub repo: &'a Path,
    /// Base ref; both worktrees check out its sha.
    pub base: &'a str,
    pub patch_path: &'a Path,
    /// Keep runs under this directory instead of a temp dir.
    pub workdir: Option<PathBuf>,
    pub keep_worktrees: bool,
    pub preset: &'a Preset,
    pub runner: &'a TemplateRunner,
    /// Explicit tests (`--tests`); they beat the preset's selector.
    pub tests: Option<&'a [String]>,
    pub repeat: i64,
}

/// Both sides run and diffed.
pub struct PairOutcome {
    /// The diff git applied, parsed.
    pub patch: Patch,
    pub linemaps: LineMaps,
    pub delta: Delta,
    /// Full sha of the base.
    pub base_ref: String,
    pub base_sha: Vec<String>,
    pub patched_sha: Vec<String>,
    pub base_outcomes: Option<Outcomes>,
    pub patched_outcomes: Option<Outcomes>,
    /// The selection as it applies to the patched tree.
    pub patched_selection: Selection,
}

#[derive(Debug)]
pub enum PipelineError {
    Workspace(WorkspaceError),
    Run(RunError),
    /// A user-facing failure (the CLI exits 1): bad applied patch, unknown selector, bad lcov
    /// output, a run that measured the main checkout.
    Failure(String),
    /// Reading a JUnit file failed.
    JunitRead(PathBuf, io::Error),
    Io(io::Error),
    /// A selector or symbolizer supplied by the caller failed.
    Host(HostError),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::Workspace(e) => e.fmt(f),
            PipelineError::Run(e) => e.fmt(f),
            PipelineError::Failure(msg) => f.write_str(msg),
            PipelineError::JunitRead(path, e) => {
                write!(f, "cannot read JUnit file {}: {e}", py_str(path))
            }
            PipelineError::Io(e) => e.fmt(f),
            PipelineError::Host(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<WorkspaceError> for PipelineError {
    fn from(e: WorkspaceError) -> Self {
        PipelineError::Workspace(e)
    }
}

impl From<RunError> for PipelineError {
    fn from(e: RunError) -> Self {
        PipelineError::Run(e)
    }
}

impl From<io::Error> for PipelineError {
    fn from(e: io::Error) -> Self {
        PipelineError::Io(e)
    }
}

/// Run the preset on the base and the patched tree and diff them. Warnings are appended to
/// `warnings` even when it fails.
pub fn run_pair(
    cfg: &PairConfig<'_>,
    selectors: SelectorLookup<'_>,
    symbolizers: SymbolizerLookup<'_>,
    warnings: &mut Vec<ReportWarning>,
) -> Result<PairOutcome, PipelineError> {
    if is_dirty(cfg.repo)? {
        warnings.push(warning(
            "dirty_tree",
            "working tree has uncommitted changes; the base is the ref",
        ));
    }
    let mut ws = Workspace::new(
        cfg.repo.to_path_buf(),
        cfg.base.to_owned(),
        cfg.patch_path.to_path_buf(),
        cfg.workdir.clone(),
        cfg.keep_worktrees,
    );
    ws.enter()?;
    // The worktrees go away however the body ends.
    let result = in_workspace(&ws, cfg, selectors, symbolizers, warnings);
    ws.exit();
    result
}

fn in_workspace(
    ws: &Workspace,
    cfg: &PairConfig<'_>,
    selectors: SelectorLookup<'_>,
    symbolizers: SymbolizerLookup<'_>,
    warnings: &mut Vec<ReportWarning>,
) -> Result<PairOutcome, PipelineError> {
    let (base, patched, workdir) = match (&ws.base, &ws.patched, &ws.workdir) {
        (Some(b), Some(p), Some(w)) => (b.as_path(), p.as_path(), w.as_path()),
        _ => unreachable!("Workspace::enter sets base, patched and workdir"),
    };
    // Use what git applied, not the user's hunk headers or path prefixes.
    let applied = PyText::from_surrogateescape_bytes(ws.applied_patch.as_deref().unwrap_or(&[]));
    let (patch, linemaps, patch_warnings) =
        load_patch(&applied).map_err(|e| PipelineError::Failure(format!("applied patch: {e}")))?;
    warnings.extend(patch_warnings);
    if let Some(paths) = ws.offset_paths.as_ref().filter(|p| !p.is_empty()) {
        warnings.push(warning(
            "patch_offset",
            &format!(
                "patch applied at an offset in {}; line maps use the applied diff",
                paths.join(", ")
            ),
        ));
    }
    let selection = choose(
        cfg.tests,
        cfg.preset.selector.as_deref(),
        &patch,
        patched,
        selectors,
    )
    .map_err(|e| match e {
        ChooseError::Unknown(_) => PipelineError::Failure(e.to_string()),
        ChooseError::Host(e) => PipelineError::Host(e),
    })?;
    let out = workdir.join("out");
    let (base_snaps, base_junit, base_sha) =
        run_side("base", cfg, &selection, base, &out.join("base"), warnings)?;
    let (patched_snaps, patched_junit, patched_sha) = run_side(
        "patched",
        cfg,
        &selection,
        patched,
        &out.join("patched"),
        warnings,
    )?;
    let opts = DiffOptions {
        base_root: Some(base),
        patched_root: Some(patched),
        base_outcomes: base_junit.as_ref(),
        patched_outcomes: patched_junit.as_ref(),
    };
    let delta =
        diff(&base_snaps, &patched_snaps, &linemaps, opts, symbolizers).map_err(|e| match e {
            DiffError::Io(e) => PipelineError::Io(e),
            DiffError::Symbolizer(SymbolizerError::Other(e)) => PipelineError::Host(e),
            other => PipelineError::Host(other.to_string().into()),
        })?;
    let patched_selection = restrict_to_tree(&selection, patched).unwrap_or(selection);
    Ok(PairOutcome {
        patch,
        linemaps,
        delta,
        base_ref: ws.base_sha.clone().unwrap_or_default(),
        base_sha,
        patched_sha,
        base_outcomes: base_junit,
        patched_outcomes: patched_junit,
        patched_selection,
    })
}

type SideResult = (Vec<Snapshot>, Option<Outcomes>, Vec<String>);

/// Run one side; returns its snapshots, first-run outcomes and lcov hashes.
fn run_side(
    side: &str,
    cfg: &PairConfig<'_>,
    selection: &Selection,
    worktree: &Path,
    out: &Path,
    warnings: &mut Vec<ReportWarning>,
) -> Result<SideResult, PipelineError> {
    let side_selection = if cfg.preset.tests_style == TestsStyle::Args {
        restrict_to_tree(selection, worktree)
    } else {
        Some(selection.clone())
    };
    let Some(side_selection) = side_selection else {
        // Every selected test is new in the patch: nothing ran on this side.
        warnings.push(warning(
            "nothing_to_run",
            &format!(
                "no selected tests exist on the {side} side; its coverage is treated as empty"
            ),
        ));
        return Ok((vec![Snapshot::default()], Some(BTreeMap::new()), Vec::new()));
    };

    let runs = cfg.runner.run(worktree, &side_selection, out, cfg.repeat)?;
    let mut snapshots = Vec::new();
    let mut hashes = Vec::new();
    let roots = [py_str(worktree)];
    for art in &runs {
        let raw = match read_text(&art.lcov) {
            Ok(text) => lcov::parse(&text).map_err(|e| e.to_string()),
            Err(ReadError::Decode(msg)) => Err(msg),
            Err(ReadError::Os(e)) => return Err(e.into()),
        };
        let raw =
            raw.map_err(|msg| PipelineError::Failure(format!("{side}: bad lcov output: {msg}")))?;
        let wrong = wrong_tree_paths(raw.files.values().map(|fc| &fc.path), worktree, cfg.repo);
        if !wrong.is_empty() {
            return Err(PipelineError::Failure(wrong_tree_message(
                side, &wrong, cfg.preset,
            )));
        }
        snapshots.push(normalize::normalize_snapshot(&raw, &roots));
        hashes.push(hashing::sha256_file(&art.lcov)?);
    }
    let junit = runs[0].junit.as_deref();
    let outcomes = load_junit(junit, warnings)
        .map_err(|e| PipelineError::JunitRead(junit.unwrap().into(), e))?;
    Ok((snapshots, outcomes, hashes))
}

pub fn wrong_tree_message(side: &str, paths: &[String], preset: &Preset) -> String {
    let mut lines = vec![format!(
        "the {side} run measured your main checkout instead of its worktree \
         (editable install or stale build?):"
    )];
    lines.extend(paths.iter().take(20).map(|p| format!("  {p}")));
    if paths.len() > 20 {
        lines.push(format!("  ... and {} more", paths.len() - 20));
    }
    if !preset.hints.is_empty() {
        lines.push("hints:".into());
        lines.extend(preset.hints.iter().map(|h| format!("  - {h}")));
    }
    lines.join("\n")
}
