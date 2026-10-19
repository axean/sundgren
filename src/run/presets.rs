//! Presets: command templates plus where their outputs land.

use std::collections::BTreeMap;
use std::path::Path;

use toml::{Table, Value};

use crate::{Error, Result};

const BUILTIN: [&str; 2] = [
    include_str!("../presets/python.toml"),
    include_str!("../presets/rust.toml"),
];

const KEYS: [&str; 9] = [
    "description",
    "commands",
    "env",
    "lcov",
    "junit",
    "selector",
    "tests_style",
    "shell",
    "hints",
];

/// How selected tests reach the commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestsStyle {
    /// Paths, one argv entry each; missing paths are dropped per side.
    Args,
    /// Name filters, passed as they are.
    Filter,
}

impl TestsStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            TestsStyle::Args => "args",
            TestsStyle::Filter => "filter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub name: String,
    pub description: String,
    /// Run in order, cwd = worktree.
    pub commands: Vec<String>,
    /// Values are templates.
    pub env: Vec<(String, String)>,
    /// Template for the lcov output path.
    pub lcov: String,
    pub junit: Option<String>,
    /// Selector id; None = run all unless --tests.
    pub selector: Option<String>,
    pub tests_style: TestsStyle,
    pub shell: bool,
    pub hints: Vec<String>,
    /// Where it was defined, shown by `sundgren presets`.
    pub source: String,
}

/// The preset behind `--cmd`.
pub fn adhoc_preset(
    commands: Vec<String>,
    lcov: String,
    junit: Option<String>,
    shell: bool,
) -> Preset {
    Preset {
        name: "adhoc".into(),
        description: "--cmd".into(),
        commands,
        env: Vec::new(),
        lcov,
        junit,
        selector: None,
        tests_style: TestsStyle::Args,
        shell,
        hints: Vec::new(),
        source: "--cmd".into(),
    }
}

/// Built-ins, overridden by the repo's presets of the same name. A bad file is a usage error.
pub fn all_presets(repo: &Path) -> Result<BTreeMap<String, Preset>> {
    let mut out = BTreeMap::new();
    for text in BUILTIN {
        let doc: Table = text.parse().expect("built-in presets are valid TOML");
        for p in parse_presets(&doc, "built-in", "built-in")? {
            out.insert(p.name.clone(), p);
        }
    }
    let pyproject = repo.join("pyproject.toml");
    if pyproject.is_file() {
        let (doc, file) = load(&pyproject)?;
        let sundgren = match doc.get("tool").and_then(|t| t.get("sundgren")) {
            None => Table::new(),
            Some(Value::Table(t)) => t.clone(),
            Some(_) => {
                return Err(Error::usage(format!(
                    "{file}: [tool.sundgren] must be a table"
                )));
            }
        };
        for p in parse_presets(&sundgren, &file, "pyproject.toml")? {
            out.insert(p.name.clone(), p);
        }
    }
    // sundgren.toml wins over pyproject.toml for the same name.
    let dedicated = repo.join("sundgren.toml");
    if dedicated.is_file() {
        let (doc, file) = load(&dedicated)?;
        for p in parse_presets(&doc, &file, "sundgren.toml")? {
            out.insert(p.name.clone(), p);
        }
    }
    Ok(out)
}

fn load(path: &Path) -> Result<(Table, String)> {
    let file = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| Error::usage(format!("{file}: {e}")))?;
    let doc = text
        .parse::<Table>()
        .map_err(|e| Error::usage(format!("{file}: {}", e.message())))?;
    Ok((doc, file))
}

/// Presets from the `presets` table of `doc`; an absent table means none.
fn parse_presets(doc: &Table, file: &str, source: &str) -> Result<Vec<Preset>> {
    match doc.get("presets") {
        None => Ok(Vec::new()),
        Some(Value::Table(table)) => table
            .iter()
            .map(|(name, raw)| parse_preset(name, raw, file, source))
            .collect(),
        Some(_) => Err(Error::usage(format!("{file}: 'presets' must be a table"))),
    }
}

fn parse_preset(name: &str, raw: &Value, file: &str, source: &str) -> Result<Preset> {
    let fail = |problem: &str| Error::usage(format!("{file}: preset '{name}': {problem}"));
    let Value::Table(raw) = raw else {
        return Err(fail("must be a table"));
    };
    let unknown: Vec<&str> = raw
        .keys()
        .map(String::as_str)
        .filter(|k| !KEYS.contains(k))
        .collect();
    if !unknown.is_empty() {
        return Err(fail(&format!("unknown key(s) {}", unknown.join(", "))));
    }
    for key in ["commands", "lcov"] {
        if !raw.contains_key(key) {
            return Err(fail(&format!("missing required key '{key}'")));
        }
    }
    let opt_str = |key: &str| match raw.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(fail(&format!("'{key}' must be a string"))),
    };
    let str_list = |key: &str| {
        let bad = || fail(&format!("'{key}' must be a list of strings"));
        match raw.get(key) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|v| v.as_str().map(str::to_owned).ok_or_else(bad))
                .collect(),
            Some(_) => Err(bad()),
        }
    };
    let commands = str_list("commands")?;
    if commands.is_empty() {
        return Err(fail("'commands' must not be empty"));
    }
    let env = match raw.get("env") {
        None => Vec::new(),
        Some(Value::Table(t)) => t
            .iter()
            .map(|(k, v)| match v {
                Value::String(s) => Ok((k.clone(), s.clone())),
                _ => Err(fail("'env' must be a table of strings")),
            })
            .collect::<Result<_>>()?,
        Some(_) => return Err(fail("'env' must be a table of strings")),
    };
    let tests_style = match raw.get("tests_style").map(Value::as_str) {
        None => TestsStyle::Args,
        Some(Some("args")) => TestsStyle::Args,
        Some(Some("filter")) => TestsStyle::Filter,
        Some(_) => return Err(fail("'tests_style' must be 'args' or 'filter'")),
    };
    let shell = match raw.get("shell") {
        None => false,
        Some(Value::Boolean(b)) => *b,
        Some(_) => return Err(fail("'shell' must be a boolean")),
    };
    let lcov = opt_str("lcov")?.unwrap_or_default();
    if lcov.is_empty() {
        return Err(fail("'lcov' must be a non-empty string"));
    }
    Ok(Preset {
        name: name.to_owned(),
        description: opt_str("description")?.unwrap_or_default(),
        commands,
        env,
        lcov,
        junit: opt_str("junit")?,
        selector: opt_str("selector")?,
        tests_style,
        shell,
        hints: str_list("hints")?,
        source: source.to_owned(),
    })
}

#[cfg(test)]
mod tests;
