//! Command line: `sundgren graph`, `sundgren contain`.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::contain;
use crate::git::{self, Commit};
use crate::graph::{self, Line};
use crate::{EXIT_OK, Error, Result};

pub const GRAPH_SCHEMA: &str = "sundgren.graph/1";

const USAGE: &str = "\
Git history as a graph, and how much work branches share.

Usage: sundgren graph [REV] [--repo DIR] [-n N]
       sundgren contain A B --base REV [--repo DIR] [--fuzzy T] [--format json|text]

graph: prints the history of REV, laid out as a graph, as sundgren.graph/1 JSON.
  REV                 branch, tag or commit (default HEAD)
  -n, --max-count N   list at most N commits (default 1000)

contain (prototype): how much of the work A and B each added since they left the base is found
in the other, as sundgren.containment/0 JSON.
  --base REV          the base both are compared from, e.g. main
  --fuzzy T           a line also counts as found if one is at least T similar (normalized
                      Levenshtein, 0 to 1; default 0.9; 1 for exact only)
  --format FORMAT     json (default) or text

Options:
  --repo DIR          a directory in the repository (default .)
  -h, --help          print this help
  -V, --version       print the version
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Text,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    Version,
    Graph {
        rev: String,
        repo: PathBuf,
        max_count: usize,
    },
    Contain {
        a: String,
        b: String,
        base: String,
        repo: PathBuf,
        fuzzy: f64,
        format: Format,
    },
}

/// A subcommand's arguments: positionals, and `--option value` pairs (short forms expanded).
struct Arguments {
    positional: Vec<String>,
    options: Vec<(String, String)>,
}

impl Arguments {
    fn get(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Split the arguments after a subcommand; None if they ask for help.
fn split(args: impl Iterator<Item = String>, known: &[&str]) -> Result<Option<Arguments>> {
    let mut args = args;
    let mut parsed = Arguments {
        positional: Vec::new(),
        options: Vec::new(),
    };
    let mut only_positional = false;
    while let Some(arg) = args.next() {
        if only_positional || !arg.starts_with('-') || arg == "-" {
            parsed.positional.push(arg);
            continue;
        }
        if arg == "--" {
            only_positional = true;
            continue;
        }
        if arg == "-h" || arg == "--help" {
            return Ok(None);
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_owned(), Some(value.to_owned()))
            }
            _ => (arg, None),
        };
        let flag = if flag == "-n" {
            "--max-count".to_owned()
        } else {
            flag
        };
        if !known.contains(&flag.as_str()) {
            return Err(Error::usage(format!("unknown option {flag:?}; see --help")));
        }
        let value = inline
            .or_else(|| args.next())
            .ok_or_else(|| Error::usage(format!("{flag} needs a value")))?;
        parsed.options.push((flag, value));
    }
    Ok(Some(parsed))
}

/// Parse the arguments after the program name.
pub fn parse<I, S>(args: I) -> Result<Command>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let Some(first) = args.next() else {
        return Ok(Command::Help);
    };
    match first.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "graph" => {
            let Some(parsed) = split(args, &["--repo", "--max-count"])? else {
                return Ok(Command::Help);
            };
            let [rev] = match parsed.positional.as_slice() {
                [] => ["HEAD".to_owned()],
                [rev] => [rev.clone()],
                _ => return Err(Error::usage("graph takes one REV")),
            };
            let max_count = match parsed.get("--max-count") {
                None => 1000,
                Some(v) => v.parse().ok().filter(|&n: &usize| n > 0).ok_or_else(|| {
                    Error::usage(format!("--max-count must be a positive number, not {v:?}"))
                })?,
            };
            Ok(Command::Graph {
                rev,
                repo: PathBuf::from(parsed.get("--repo").unwrap_or(".")),
                max_count,
            })
        }
        "contain" => {
            let known = ["--repo", "--base", "--fuzzy", "--format"];
            let Some(parsed) = split(args, &known)? else {
                return Ok(Command::Help);
            };
            let [a, b] = parsed.positional.as_slice() else {
                return Err(Error::usage("contain takes two revisions, A and B"));
            };
            let base = parsed
                .get("--base")
                .ok_or_else(|| Error::usage("contain needs --base"))?;
            let fuzzy = match parsed.get("--fuzzy") {
                None => 0.9,
                Some(v) => v
                    .parse()
                    .ok()
                    .filter(|t: &f64| (0.0..=1.0).contains(t))
                    .ok_or_else(|| {
                        Error::usage(format!("--fuzzy must be a number from 0 to 1, not {v:?}"))
                    })?,
            };
            let format = match parsed.get("--format") {
                None | Some("json") => Format::Json,
                Some("text") => Format::Text,
                Some(other) => {
                    return Err(Error::usage(format!(
                        "--format must be json or text, not {other:?}"
                    )));
                }
            };
            Ok(Command::Contain {
                a: a.clone(),
                b: b.clone(),
                base: base.to_owned(),
                repo: PathBuf::from(parsed.get("--repo").unwrap_or(".")),
                fuzzy,
                format,
            })
        }
        other => Err(Error::usage(format!(
            "unknown command {other:?}; see --help"
        ))),
    }
}

/// One commit with its place in the graph.
#[derive(Debug, Serialize)]
pub struct Node {
    #[serde(flatten)]
    pub commit: Commit,
    pub column: usize,
    pub color: usize,
    pub lines: Vec<Line>,
}

/// The `sundgren.graph/1` document.
#[derive(Debug, Serialize)]
pub struct GraphDocument {
    pub schema: &'static str,
    pub repository: PathBuf,
    pub rev: String,
    pub oid: String,
    /// True if the history goes on past the last commit listed.
    pub truncated: bool,
    pub columns: usize,
    pub commits: Vec<Node>,
    pub tail: Vec<Line>,
}

pub fn main() -> i32 {
    match parse(std::env::args().skip(1)).and_then(|command| run(&command)) {
        Ok(()) => EXIT_OK,
        Err(e) => {
            eprintln!("sundgren: {}", e.msg);
            e.code
        }
    }
}

/// Run one parsed command line.
pub fn run(command: &Command) -> Result<()> {
    match command {
        Command::Help => {
            print!("{USAGE}");
            Ok(())
        }
        Command::Version => {
            println!("sundgren {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Contain {
            a,
            b,
            base,
            repo,
            fuzzy,
            format,
        } => {
            // 1.0 means exact only: fuzzy matching is off above 1.
            let threshold = if *fuzzy >= 1.0 { 2.0 } else { *fuzzy };
            let mut doc = contain::containment_document(repo, base, a, b, threshold)?;
            doc.threshold = *fuzzy;
            let mut out = std::io::stdout().lock();
            match format {
                Format::Json => {
                    serde_json::to_writer(&mut out, &doc)
                        .map_err(|e| Error::failure(e.to_string()))?;
                    writeln!(out)?;
                }
                Format::Text => write!(out, "{}", contain::text(&doc))?,
            }
            Ok(())
        }
        Command::Graph {
            rev,
            repo,
            max_count,
        } => {
            let doc = graph_document(repo, rev, *max_count)?;
            let mut out = std::io::stdout().lock();
            serde_json::to_writer(&mut out, &doc).map_err(|e| Error::failure(e.to_string()))?;
            writeln!(out)?;
            Ok(())
        }
    }
}

/// The history of `rev` in the repository containing `repo`.
pub fn graph_document(repo: &Path, rev: &str, max_count: usize) -> Result<GraphDocument> {
    let repository = git::toplevel(repo)?;
    let oid = git::resolve(&repository, rev)?;
    // One more than asked, to tell whether there are more.
    let mut commits = git::log(&repository, &oid, max_count.saturating_add(1))?;
    let truncated = commits.len() > max_count;
    commits.truncate(max_count);

    let layout = graph::layout(
        commits
            .iter()
            .map(|c| (c.oid.as_str(), c.parents.iter().map(String::as_str))),
    );
    let commits = commits
        .into_iter()
        .zip(layout.rows)
        .map(|(commit, row)| Node {
            commit,
            column: row.column,
            color: row.color,
            lines: row.lines,
        })
        .collect();
    Ok(GraphDocument {
        schema: GRAPH_SCHEMA,
        repository,
        rev: rev.to_owned(),
        oid,
        truncated,
        columns: layout.columns,
        commits,
        tail: layout.tail,
    })
}

#[cfg(test)]
mod tests;
