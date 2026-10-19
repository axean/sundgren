//! `sundgren review`: read the source tree, join coverage, report duplicate functions with patches.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use similar::TextDiff;
use sundgren_analyze::{DuplicateOptions, Suggestion, duplicate_functions};
use sundgren_core::model::{Snapshot, merge_file_coverage};
use sundgren_core::pytext::PyText;
use sundgren_frontend_python::{Options as PyOptions, Request, parse_files_with};
use sundgren_frontend_rust::{Options as RsOptions, parse_with};
use sundgren_ir::{File, apply_edits, sha256_hex};
use sundgren_join::{Granularity, Measured, Overlay, join};
use sundgren_run::template::default_python;

use crate::cli::{CliError, CliResult, EXIT_OK, EXIT_USAGE, Format, Host, load_snapshot, resolve};

pub const SCHEMA: &str = "sundgren.review/1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewArgs {
    pub root: String,
    pub lcov: Vec<String>,
    pub python: Option<String>,
    pub min_nodes: usize,
    pub format: Format,
    pub output: Option<String>,
}

pub fn cmd_review(args: &ReviewArgs, host: &mut dyn Host) -> CliResult<i32> {
    let root = PathBuf::from(resolve(&args.root));
    if !root.is_dir() {
        return Err(CliError::new(format!("not a directory: {}", args.root), EXIT_USAGE).into());
    }
    let mut warnings: Vec<String> = Vec::new();
    let paths = source_files(&root);
    let files = parse(&root, &paths, args, host, &mut warnings);

    let snapshot = coverage(&root, &args.lcov)?;
    let overlay = match &snapshot {
        Some(s) => {
            let runs = std::slice::from_ref(s);
            let shas: Vec<String> = files
                .iter()
                .map(|f| {
                    std::fs::read(root.join(&f.path))
                        .map(|b| sha256_hex(&b))
                        .unwrap_or_default()
                })
                .collect();
            let o = join(files.iter().zip(&shas).map(|(f, sha)| {
                (
                    f,
                    Measured {
                        runs,
                        source_sha256: sha,
                        granularity: Granularity::Line,
                    },
                )
            }));
            warnings.extend(
                o.warnings
                    .iter()
                    .map(|w| format!("{}: {}", w.code, w.message)),
            );
            o
        }
        None => Overlay::default(),
    };

    let options = DuplicateOptions {
        min_nodes: args.min_nodes,
    };
    let mut suggestions = duplicate_functions(&files, &overlay, options);
    sundgren_lang_python::add_duplicate_edits(&mut suggestions, &files, &root);

    let text = match args.format {
        Format::Json => {
            let report = json!({
                "schema": SCHEMA,
                "files": files.len(),
                "coverage": snapshot.is_some(),
                "suggestions": suggestions.iter().map(|s| suggestion_json(s, &files, &root)).collect::<Vec<_>>(),
                "warnings": warnings,
            });
            let mut t = serde_json::to_string_pretty(&report).expect("plain JSON");
            t.push('\n');
            t
        }
        Format::Text => render_text(&suggestions, &files, &root, &warnings, snapshot.is_some()),
    };
    match &args.output {
        Some(out) => std::fs::write(out, text)?,
        None => host.write_stdout(&PyText::from(text))?,
    }
    Ok(EXIT_OK)
}

/// `.py` and `.rs` files under `root`, repo-relative with `/`: git's tracked files when `root`
/// is in a git work tree, else a walk that skips hidden directories and build output.
fn source_files(root: &Path) -> Vec<String> {
    let git = Command::new("git")
        .args(["ls-files", "-z", "--", "*.py", "*.rs"])
        .current_dir(root)
        .output();
    if let Ok(out) = git
        && out.status.success()
    {
        let mut v: Vec<String> = out
            .stdout
            .split(|&b| b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect();
        v.sort();
        return v;
    }
    let mut v = Vec::new();
    walk(root, root, &mut v);
    v.sort();
    v
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        if path.is_dir() {
            let skip = name.starts_with('.')
                || ["target", "__pycache__", "node_modules", "venv"].contains(&name.as_str());
            if !skip {
                walk(root, &path, out);
            }
        } else if (name.ends_with(".py") || name.ends_with(".rs"))
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn parse(
    root: &Path,
    paths: &[String],
    args: &ReviewArgs,
    host: &dyn Host,
    warnings: &mut Vec<String>,
) -> Vec<File> {
    let mut files = Vec::new();
    let py: Vec<Request> = paths
        .iter()
        .filter(|p| p.ends_with(".py"))
        .map(|p| Request {
            file: root.join(p),
            path: p.clone(),
        })
        .collect();
    if !py.is_empty() {
        let python = args
            .python
            .clone()
            .unwrap_or_else(|| default_python(|n| host.which(n)));
        match parse_files_with(python.as_ref(), &py, PyOptions { l1: true }) {
            Ok(results) => {
                for (req, r) in py.iter().zip(results) {
                    match r {
                        Ok(f) => files.push(f),
                        Err(e) => warnings.push(format!("python_unparsed: {}: {e:?}", req.path)),
                    }
                }
            }
            Err(e) => warnings.push(format!("python_frontend: {python}: {e:?}")),
        }
    }
    for p in paths.iter().filter(|p| p.ends_with(".rs")) {
        let parsed = std::fs::read(root.join(p))
            .map_err(|e| format!("{e}"))
            .and_then(|b| {
                let opts = RsOptions {
                    l1: true,
                    ..RsOptions::default()
                };
                parse_with(p, &b, opts).map_err(|e| format!("{e:?}"))
            });
        match parsed {
            Ok(r) => files.push(r.file),
            Err(e) => warnings.push(format!("rust_unparsed: {p}: {e}")),
        }
    }
    files
}

/// All `--lcov` files merged into one snapshot, keyed by path relative to `root`.
fn coverage(root: &Path, lcov: &[String]) -> CliResult<Option<Snapshot>> {
    let roots = [root.to_string_lossy().into_owned()];
    let mut merged: Option<Snapshot> = None;
    for path in lcov {
        let s = load_snapshot(Path::new(path), &roots)?;
        merged = Some(match merged {
            None => s,
            Some(mut m) => {
                for (k, fc) in s.files {
                    let fc = match m.files.get(&k) {
                        Some(old) => merge_file_coverage(old, &fc),
                        None => fc,
                    };
                    m.files.insert(k, fc);
                }
                m
            }
        });
    }
    Ok(merged)
}

/// The suggestion's edits as a unified diff that `git apply` takes, or None.
fn patch(s: &Suggestion, root: &Path) -> Option<String> {
    let mut out = String::new();
    for fe in s.edits.as_ref()? {
        let old = std::fs::read(root.join(&fe.path)).ok()?;
        let new = apply_edits(&old, &fe.edits).ok()?;
        let (old, new) = (String::from_utf8_lossy(&old), String::from_utf8_lossy(&new));
        let diff = TextDiff::from_lines(old.as_ref(), new.as_ref());
        let a = format!("a/{}", fe.path);
        let b = format!("b/{}", fe.path);
        out.push_str(
            &diff
                .unified_diff()
                .context_radius(3)
                .header(&a, &b)
                .to_string(),
        );
    }
    Some(out)
}

fn suggestion_json(s: &Suggestion, files: &[File], root: &Path) -> Value {
    let touched = s.touched(files);
    let nodes: Vec<Value> = s
        .evidence
        .nodes
        .iter()
        .zip(&touched)
        .map(|(e, (path, start, end))| {
            json!({
                "id": e.node.as_str(),
                "path": path,
                "start": start,
                "end": end,
                "status": e.status.map(|st| st.as_str()),
                "executable": e.executable,
                "hit": e.hit,
                "tests": e.tests,
            })
        })
        .collect();
    json!({
        "id": s.id,
        "analysis": s.analysis,
        "message": s.message,
        "confidence": s.confidence.as_str(),
        "nodes": nodes,
        "patch": patch(s, root),
    })
}

fn render_text(
    suggestions: &[Suggestion],
    files: &[File],
    root: &Path,
    warnings: &[String],
    coverage: bool,
) -> String {
    let mut out = format!(
        "sundgren review: {} files, {} suggestions{}\n",
        files.len(),
        suggestions.len(),
        if coverage { "" } else { " (no coverage given)" }
    );
    for (i, s) in suggestions.iter().enumerate() {
        out.push_str(&format!(
            "\n[{}] {} (confidence: {})\n{}\n",
            i + 1,
            s.analysis,
            s.confidence.as_str(),
            s.message
        ));
        for (k, (e, (path, start, end))) in
            s.evidence.nodes.iter().zip(s.touched(files)).enumerate()
        {
            let status = e.status.map_or("no coverage", |st| st.as_str());
            let role = if k == 0 { "keep" } else { "replace" };
            out.push_str(&format!("  {role:<7} {path}:{start}-{end}  {status}\n"));
        }
        match patch(s, root) {
            Some(p) => {
                out.push('\n');
                out.push_str(&p);
            }
            None => out.push_str("  (no patch)\n"),
        }
    }
    for w in warnings {
        out.push_str(&format!("warning: {w}\n"));
    }
    out
}
