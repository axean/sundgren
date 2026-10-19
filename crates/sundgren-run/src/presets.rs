//! Presets: command templates plus where their outputs land. Pure data.
//!
//! TOML decoding is the caller's: the bindings use Python's `tomllib`, so accepted input and
//! decode errors stay tomllib's. This module validates the decoded tables.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::pyfmt::repr_str;
use crate::pypath::py_str;

/// How selected tests reach the commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestsStyle {
    Args,
    Filter,
}

impl TestsStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            TestsStyle::Args => "args",
            TestsStyle::Filter => "filter",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "args" => Some(TestsStyle::Args),
            "filter" => Some(TestsStyle::Filter),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Preset {
    pub name: String,
    pub description: String,
    /// Run in order, cwd = worktree.
    pub commands: Vec<String>,
    /// Values are templates. Keeps the table's order.
    pub env: Vec<(String, String)>,
    /// Template for the lcov output path.
    pub lcov: String,
    /// Template for the JUnit output path.
    pub junit: Option<String>,
    /// Selector id; None = run all unless --tests.
    pub selector: Option<String>,
    pub tests_style: TestsStyle,
    pub shell: bool,
    pub hints: Vec<String>,
    /// Where it was defined, shown by `sundgren presets`.
    pub source: String,
}

/// Field-wise, with `env` compared as a map like the dataclass's dict.
impl PartialEq for Preset {
    fn eq(&self, other: &Self) -> bool {
        let env = |p: &Self| p.env.iter().cloned().collect::<BTreeMap<_, _>>();
        self.name == other.name
            && self.description == other.description
            && self.commands == other.commands
            && env(self) == env(other)
            && self.lcov == other.lcov
            && self.junit == other.junit
            && self.selector == other.selector
            && self.tests_style == other.tests_style
            && self.shell == other.shell
            && self.hints == other.hints
            && self.source == other.source
    }
}

impl Eq for Preset {}

/// A decoded TOML value, reduced to what validation looks at.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Bool(bool),
    Array(Vec<Value>),
    Table(Table),
    /// Numbers and dates.
    Other,
}

/// A TOML table in document order.
pub type Table = Vec<(String, Value)>;

fn get<'a>(table: &'a Table, key: &str) -> Option<&'a Value> {
    table.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// A presets file or table is malformed (`Invalid`), or the caller's loader failed (`Host`).
#[derive(Debug)]
pub enum PresetError {
    Invalid(String),
    Host(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetError::Invalid(msg) => f.write_str(msg),
            PresetError::Host(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for PresetError {}

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

/// Decodes a TOML document; `file` names it in errors. Closures implement it.
pub trait TomlLoader {
    fn load(&self, data: &[u8], file: &str) -> Result<Table, PresetError>;
}

impl<F: Fn(&[u8], &str) -> Result<Table, PresetError>> TomlLoader for F {
    fn load(&self, data: &[u8], file: &str) -> Result<Table, PresetError> {
        self(data, file)
    }
}

/// Add the presets parsed from a lang pack's presets.toml to `builtins`; returns them.
pub fn register_builtin(
    builtins: &mut BTreeMap<String, Preset>,
    doc: &Table,
    file: &str,
) -> Result<Vec<Preset>, PresetError> {
    let parsed = parse_presets(doc, file, "built-in")?;
    for preset in &parsed {
        if builtins.get(&preset.name).is_some_and(|p| p != preset) {
            return Err(PresetError::Invalid(format!(
                "{file}: preset {}: already registered by another pack",
                repr_str(&preset.name)
            )));
        }
    }
    builtins.extend(parsed.iter().map(|p| (p.name.clone(), p.clone())));
    Ok(parsed)
}

/// Presets from sundgren.toml or [tool.sundgren.presets.*] in pyproject.toml.
pub fn user_presets(
    repo: &Path,
    load: &dyn TomlLoader,
) -> Result<BTreeMap<String, Preset>, PresetError> {
    let mut out = BTreeMap::new();
    let pyproject = repo.join("pyproject.toml");
    if pyproject.is_file() {
        let file = py_str(&pyproject);
        let doc = load.load(&read(&pyproject)?, &file)?;
        let sundgren = match get(&doc, "tool") {
            Some(Value::Table(tool)) => get(tool, "sundgren"),
            _ => None,
        };
        let empty = Table::new();
        let sundgren = match sundgren {
            None => &empty,
            Some(Value::Table(t)) => t,
            Some(_) => {
                return Err(PresetError::Invalid(format!(
                    "{file}: [tool.sundgren] must be a table"
                )));
            }
        };
        for p in parse_presets(sundgren, &file, "pyproject.toml")? {
            out.insert(p.name.clone(), p);
        }
    }
    // sundgren.toml wins over pyproject.toml for the same name.
    let dedicated = repo.join("sundgren.toml");
    if dedicated.is_file() {
        let file = py_str(&dedicated);
        let doc = load.load(&read(&dedicated)?, &file)?;
        for p in parse_presets(&doc, &file, "sundgren.toml")? {
            out.insert(p.name.clone(), p);
        }
    }
    Ok(out)
}

/// Built-ins overridden by user presets of the same name.
pub fn all_presets(
    builtins: &BTreeMap<String, Preset>,
    repo: &Path,
    load: &dyn TomlLoader,
) -> Result<BTreeMap<String, Preset>, PresetError> {
    let mut out = builtins.clone();
    out.extend(user_presets(repo, load)?);
    Ok(out)
}

fn read(path: &Path) -> Result<Vec<u8>, PresetError> {
    std::fs::read(path).map_err(|e| PresetError::Host(Box::new(e)))
}

/// Presets from the `presets` table of `doc`, in document order; absent table = none.
pub fn parse_presets(doc: &Table, file: &str, source: &str) -> Result<Vec<Preset>, PresetError> {
    match get(doc, "presets") {
        None => Ok(Vec::new()),
        Some(Value::Table(table)) => table
            .iter()
            .map(|(name, raw)| parse_preset(name, raw, file, source))
            .collect(),
        Some(_) => Err(PresetError::Invalid(format!(
            "{file}: 'presets' must be a table"
        ))),
    }
}

fn parse_preset(name: &str, raw: &Value, file: &str, source: &str) -> Result<Preset, PresetError> {
    let fail = |problem: &str| {
        PresetError::Invalid(format!("{file}: preset {}: {problem}", repr_str(name)))
    };
    let Value::Table(raw) = raw else {
        return Err(fail("must be a table"));
    };
    let mut unknown: Vec<&str> = raw
        .iter()
        .map(|(k, _)| k.as_str())
        .filter(|k| !KEYS.contains(k))
        .collect();
    if !unknown.is_empty() {
        unknown.sort_unstable();
        unknown.dedup();
        return Err(fail(&format!("unknown key(s) {}", unknown.join(", "))));
    }
    for key in ["commands", "lcov"] {
        if get(raw, key).is_none() {
            return Err(fail(&format!("missing required key {}", repr_str(key))));
        }
    }
    let opt_str = |key: &str| match get(raw, key) {
        None => Ok(None),
        Some(Value::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(fail(&format!("'{key}' must be a string"))),
    };
    let str_list = |key: &str| match get(raw, key) {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(fail(&format!("'{key}' must be a list of strings"))),
            })
            .collect(),
        Some(_) => Err(fail(&format!("'{key}' must be a list of strings"))),
    };

    let commands = str_list("commands")?;
    if commands.is_empty() {
        return Err(fail("'commands' must not be empty"));
    }
    let env = match get(raw, "env") {
        None => Vec::new(),
        Some(Value::Table(t)) => t
            .iter()
            .map(|(k, v)| match v {
                Value::Str(s) => Ok((k.clone(), s.clone())),
                _ => Err(fail("'env' must be a table of strings")),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(fail("'env' must be a table of strings")),
    };
    let tests_style = match get(raw, "tests_style") {
        None => Some(TestsStyle::Args),
        Some(Value::Str(s)) => TestsStyle::parse(s),
        Some(_) => None,
    }
    .ok_or_else(|| fail("'tests_style' must be 'args' or 'filter'"))?;
    let shell = match get(raw, "shell") {
        None => false,
        Some(Value::Bool(b)) => *b,
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
mod tests {
    use super::*;

    fn s(v: &str) -> Value {
        Value::Str(v.into())
    }

    fn table(items: &[(&str, Value)]) -> Value {
        Value::Table(
            items
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }

    fn doc(preset: Value) -> Table {
        vec![("presets".into(), table(&[("bad", preset)]))]
    }

    fn err(preset: Value) -> String {
        parse_presets(&doc(preset), "f.toml", "x")
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn minimal_preset_gets_defaults() {
        let got = parse_presets(
            &doc(table(&[
                ("commands", Value::Array(vec![s("run")])),
                ("lcov", s("x")),
            ])),
            "f.toml",
            "sundgren.toml",
        )
        .unwrap();
        assert_eq!(
            got,
            [Preset {
                name: "bad".into(),
                description: String::new(),
                commands: vec!["run".into()],
                env: vec![],
                lcov: "x".into(),
                junit: None,
                selector: None,
                tests_style: TestsStyle::Args,
                shell: false,
                hints: vec![],
                source: "sundgren.toml".into(),
            }]
        );
    }

    #[test]
    fn validation_messages() {
        let cmds = || ("commands", Value::Array(vec![s("x")]));
        assert_eq!(
            err(table(&[("lcov", s("x"))])),
            "f.toml: preset 'bad': missing required key 'commands'"
        );
        assert_eq!(
            err(table(&[cmds(), ("lcov", Value::Other)])),
            "f.toml: preset 'bad': 'lcov' must be a string"
        );
        assert_eq!(
            err(table(&[
                cmds(),
                ("lcov", s("x")),
                ("zz", s("")),
                ("aa", s(""))
            ])),
            "f.toml: preset 'bad': unknown key(s) aa, zz"
        );
        assert_eq!(err(s("x")), "f.toml: preset 'bad': must be a table");
        assert_eq!(
            parse_presets(&vec![("presets".into(), s("x"))], "f.toml", "x")
                .unwrap_err()
                .to_string(),
            "f.toml: 'presets' must be a table"
        );
    }
}
