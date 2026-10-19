//! Run mode: worktrees at the base, the preset's commands on both sides, then diff.

pub mod presets;
pub mod runner;
pub mod selectors;
pub mod workspace;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::diff::{Delta, DiffOptions, diff};
use crate::linemap::build_linemaps;
use crate::model::{Outcomes, Snapshot, Warning, warning};
use crate::normalize::{normalize_snapshot, realpath};
use crate::patch::{Patch, parse_patch};
use crate::{Error, Result};

use presets::{Preset, TestsStyle};
pub use selectors::Selection;
pub use workspace::repo_root;
use workspace::{Workspace, is_dirty};

/// `python` on PATH, else `python3`, else "python".
pub fn default_python() -> String {
    ["python", "python3"]
        .into_iter()
        .find_map(which)
        .unwrap_or_else(|| "python".into())
}

fn which(name: &str) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| {
            p.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .map(|p| p.display().to_string())
}

/// What to run and where.
pub struct RunConfig {
    pub repo: PathBuf,
    pub base: String,
    /// Absolute.
    pub patch_path: PathBuf,
    pub workdir: Option<PathBuf>,
    pub keep_worktrees: bool,
    pub preset: Preset,
    pub python: String,
    /// `--env` overrides, applied after the preset's env.
    pub env: Vec<(String, String)>,
    /// `--tests`; empty = let the preset's selector choose.
    pub tests: Vec<String>,
    pub repeat: i64,
}

/// Both sides run and diffed.
pub struct RunOutcome {
    /// The diff git applied, parsed.
    pub patch: Patch,
    pub delta: Delta,
    /// Full sha of the base.
    pub base_ref: String,
    pub base_sha: Vec<String>,
    pub patched_sha: Vec<String>,
    /// Display commands for the report's meta.
    pub commands: Vec<String>,
    /// The selection as it applies to the patched tree.
    pub selection: Selection,
    pub warnings: Vec<Warning>,
}

/// Run the preset on the base and the patched tree and diff them.
pub fn run_pair(cfg: &RunConfig) -> Result<RunOutcome> {
    let mut warnings = Vec::new();
    if is_dirty(&cfg.repo)? {
        warnings.push(warning(
            "dirty_tree",
            "working tree has uncommitted changes; the base is the ref",
        ));
    }
    let ws = Workspace::new(
        &cfg.repo,
        &cfg.base,
        &cfg.patch_path,
        cfg.workdir.as_deref(),
        cfg.keep_worktrees,
    )?;
    // Use what git applied, not the user's hunk headers or path prefixes.
    let patch = parse_patch(&ws.applied_patch)
        .map_err(|e| Error::failure(format!("applied patch: {e}")))?;
    warnings.extend(patch.warnings.iter().map(|w| warning("patch", w.clone())));
    if !ws.offset_paths.is_empty() {
        warnings.push(warning(
            "patch_offset",
            format!(
                "patch applied at an offset in {}; line maps use the applied diff",
                ws.offset_paths.join(", ")
            ),
        ));
    }
    let linemaps = build_linemaps(&patch);
    let selection = selectors::choose(
        &cfg.tests,
        cfg.preset.selector.as_deref(),
        &patch,
        &ws.patched,
    )?;
    let out = ws.workdir.join("out");
    let base = run_side(
        "base",
        cfg,
        &selection,
        &ws.base,
        &out.join("base"),
        &mut warnings,
    )?;
    let patched = run_side(
        "patched",
        cfg,
        &selection,
        &ws.patched,
        &out.join("patched"),
        &mut warnings,
    )?;
    let opts = DiffOptions {
        base_root: Some(&ws.base),
        patched_root: Some(&ws.patched),
        base_outcomes: base.outcomes.as_ref(),
        patched_outcomes: patched.outcomes.as_ref(),
    };
    let delta = diff(&base.snapshots, &patched.snapshots, &linemaps, opts)?;
    let selection = selectors::restrict_to_tree(&selection, &ws.patched).unwrap_or(selection);
    let commands = runner::display_commands(&cfg.preset, &cfg.python, &selection)?;
    Ok(RunOutcome {
        patch,
        delta,
        base_ref: ws.base_sha.clone(),
        base_sha: base.hashes,
        patched_sha: patched.hashes,
        commands,
        selection,
        warnings,
    })
}

struct Side {
    snapshots: Vec<Snapshot>,
    /// From the first run.
    outcomes: Option<Outcomes>,
    /// sha256 of each run's lcov.
    hashes: Vec<String>,
}

fn run_side(
    side: &str,
    cfg: &RunConfig,
    selection: &Selection,
    worktree: &Path,
    out: &Path,
    warnings: &mut Vec<Warning>,
) -> Result<Side> {
    let selection = match cfg.preset.tests_style {
        TestsStyle::Args => selectors::restrict_to_tree(selection, worktree),
        TestsStyle::Filter => Some(selection.clone()),
    };
    let Some(selection) = selection else {
        // Every selected test is new in the patch: nothing ran on this side.
        warnings.push(warning(
            "nothing_to_run",
            format!("no selected tests exist on the {side} side; its coverage is treated as empty"),
        ));
        return Ok(Side {
            snapshots: vec![Snapshot::default()],
            outcomes: Some(BTreeMap::new()),
            hashes: Vec::new(),
        });
    };
    let runs = runner::run(
        &cfg.preset,
        &cfg.python,
        &cfg.env,
        worktree,
        &selection,
        out,
        cfg.repeat,
    )?;
    let root = worktree.display().to_string();
    let mut snapshots = Vec::new();
    let mut hashes = Vec::new();
    for run in &runs {
        let text = String::from_utf8_lossy(&std::fs::read(&run.lcov)?).into_owned();
        let raw = crate::lcov::parse(&text)
            .map_err(|e| Error::failure(format!("{side}: bad lcov output: {e}")))?;
        let wrong = wrong_tree_paths(
            raw.files.values().map(|fc| fc.path.as_str()),
            worktree,
            &cfg.repo,
        );
        if !wrong.is_empty() {
            return Err(Error::failure(wrong_tree_message(
                side,
                &wrong,
                &cfg.preset,
            )));
        }
        snapshots.push(normalize_snapshot(&raw, &[&root]));
        hashes.push(crate::sha256_file(&run.lcov)?);
    }
    let outcomes = match &runs[0].junit {
        None => None,
        Some(path) => {
            let text = String::from_utf8_lossy(&std::fs::read(path)?).into_owned();
            match crate::junit::parse_junit(&text) {
                Ok(outcomes) => Some(outcomes),
                Err(e) => {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    warnings.push(warning("junit_unreadable", format!("{name}: {e}")));
                    None
                }
            }
        }
    };
    Ok(Side {
        snapshots,
        outcomes,
        hashes,
    })
}

/// Absolute SF paths outside `worktree` but inside `main_repo`: coverage measured the main
/// checkout (editable install, stale build). Sorted.
fn wrong_tree_paths<'a>(
    paths: impl Iterator<Item = &'a str>,
    worktree: &Path,
    main_repo: &Path,
) -> Vec<String> {
    let tree = PathBuf::from(realpath(&worktree.to_string_lossy()));
    let repo = PathBuf::from(realpath(&main_repo.to_string_lossy()));
    let bad: BTreeSet<String> = paths
        .filter(|raw| raw.starts_with('/'))
        .filter(|raw| {
            let real = PathBuf::from(realpath(raw));
            // The worktree may itself sit inside main_repo (--workdir in the repo).
            !real.starts_with(&tree) && real.starts_with(&repo)
        })
        .map(str::to_owned)
        .collect();
    bad.into_iter().collect()
}

fn wrong_tree_message(side: &str, paths: &[String], preset: &Preset) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use presets::adhoc_preset;

    #[test]
    fn wrong_tree_paths_cases() {
        let paths = [
            "/repo/src/a.py",
            "rel.py",
            "/repo/wt/src/a.py",
            "/elsewhere/x.py",
            "/repo/src/a.py",
        ];
        let got = wrong_tree_paths(paths.into_iter(), Path::new("/repo/wt"), Path::new("/repo"));
        assert_eq!(got, ["/repo/src/a.py"]);
        // Component-wise: /repo-other is not inside /repo.
        let got = wrong_tree_paths(
            ["/repo-other/a.py"].into_iter(),
            Path::new("/wt"),
            Path::new("/repo"),
        );
        assert!(got.is_empty());
    }

    #[test]
    fn wrong_tree_message_truncates_and_adds_hints() {
        let mut preset = adhoc_preset(vec!["x".into()], "l".into(), None, false);
        let paths: Vec<String> = (0..22).map(|i| format!("/repo/f{i}.py")).collect();
        let msg = wrong_tree_message("base", &paths, &preset);
        assert!(msg.starts_with(
            "the base run measured your main checkout instead of its worktree \
             (editable install or stale build?):\n  /repo/f0.py\n"
        ));
        assert!(msg.ends_with("  /repo/f19.py\n  ... and 2 more"));
        preset.hints = vec!["reinstall".into()];
        let msg = wrong_tree_message("patched", &paths[..1], &preset);
        assert!(msg.ends_with("  /repo/f0.py\nhints:\n  - reinstall"));
    }

    #[test]
    fn default_python_is_a_path_or_python() {
        let p = default_python();
        assert!(p == "python" || p.starts_with('/'), "{p}");
    }

    fn config(repo: &Path, preset: Preset) -> RunConfig {
        RunConfig {
            repo: repo.to_path_buf(),
            base: "HEAD".into(),
            patch_path: repo.join("p.patch"),
            workdir: None,
            keep_worktrees: false,
            preset,
            python: "python".into(),
            env: Vec::new(),
            tests: Vec::new(),
            repeat: 1,
        }
    }

    #[test]
    fn run_pair_end_to_end() {
        let repo = workspace::tests::repo();
        workspace::tests::patch(repo.path(), "sed -i s/^2$/two/ f.txt");
        // "Coverage": line 2 is hit on the base only.
        let cmd = "sh -c 'if grep -qx 2 f.txt; then h=1; else h=0; fi; \
                   printf \"SF:f.txt\\nDA:1,1\\nDA:2,$h\\nend_of_record\\n\" > {lcov}'";
        let preset = adhoc_preset(vec![cmd.into()], "{out}/l.info".into(), None, false);
        let got = run_pair(&config(repo.path(), preset)).unwrap();
        assert_eq!(got.base_ref.len(), 40);
        assert_eq!((got.base_sha.len(), got.patched_sha.len()), (1, 1));
        assert_eq!(got.patch.files.len(), 1);
        assert!(got.selection.run_all);
        // p.patch is untracked, so the tree isn't dirty.
        assert!(got.warnings.is_empty(), "{:?}", got.warnings);
        let out = Command::new("git")
            .args(["worktree", "list"])
            .current_dir(repo.path())
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).lines().count(), 1);
    }

    #[test]
    fn run_pair_fails_on_wrong_tree_and_cleans_up() {
        let repo = workspace::tests::repo();
        workspace::tests::patch(repo.path(), "sed -i s/^2$/two/ f.txt");
        let main = repo.path().canonicalize().unwrap();
        let cmd = format!(
            "sh -c 'printf \"SF:{}/f.txt\\nDA:1,1\\nend_of_record\\n\" > {{lcov}}'",
            main.display()
        );
        let preset = adhoc_preset(vec![cmd], "{out}/l.info".into(), None, false);
        let e = run_pair(&config(repo.path(), preset)).err().unwrap();
        assert_eq!(e.code, crate::EXIT_FAILURE);
        assert!(
            e.msg
                .starts_with("the base run measured your main checkout"),
            "{}",
            e.msg
        );
    }

    use std::process::Command;
}
