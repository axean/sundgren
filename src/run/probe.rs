//! For `run --check`: would the tests import the project from the main checkout instead of the
//! worktree they run in (an editable install, PYTHONPATH)? Then both sides measure the same code.

use std::path::Path;
use std::process::Command;

use super::presets::Preset;
use crate::Result;

const SCRIPT: &str = include_str!("probe.py");
const MARKER: &str = "SUNDGREN-PROBE ";

/// Whether the preset runs pytest with `{python}`, so the probe means something.
pub fn applies(preset: &Preset) -> bool {
    preset
        .commands
        .iter()
        .any(|c| c.contains("{python}") && c.contains("-m pytest"))
}

/// (package, file in the main checkout) for each package the tests would import from there.
/// Empty when the probe can't tell, e.g. pytest isn't installed.
pub fn wrong_tree_imports(
    python: &str,
    env: &[(String, String)],
    worktree: &Path,
    repo: &Path,
    tests: &[String],
) -> Result<Vec<(String, String)>> {
    let mut cmd = Command::new(python);
    cmd.arg("-c")
        .arg(SCRIPT)
        .arg(worktree)
        .arg(repo)
        .args(tests)
        .current_dir(worktree)
        .envs(env.iter().map(|(k, v)| (k, v)));
    let output = crate::cancel::output(&mut cmd);
    crate::cancel::check()?;
    let Ok(output) = output else {
        return Ok(Vec::new());
    };
    Ok(parse(&String::from_utf8_lossy(&output.stdout)))
}

fn parse(stdout: &str) -> Vec<(String, String)> {
    stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(MARKER))
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
