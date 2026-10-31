//! Command line: `sundgren graph`.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::git::{self, Commit};
use crate::graph::{self, Line};
use crate::{EXIT_OK, Error, Result};

pub const GRAPH_SCHEMA: &str = "sundgren.graph/1";

const USAGE: &str = "\
Git history as a graph.

Usage: sundgren graph [REV] [--repo DIR] [-n N]

  REV                 branch, tag or commit (default HEAD)
  --repo DIR          a directory in the repository (default .)
  -n, --max-count N   list at most N commits (default 1000)

Prints the history of REV, laid out as a graph, as sundgren.graph/1 JSON.

Options:
  -h, --help          print this help
  -V, --version       print the version
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Graph {
        rev: String,
        repo: PathBuf,
        max_count: usize,
    },
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
        "-h" | "--help" | "help" => return Ok(Command::Help),
        "-V" | "--version" => return Ok(Command::Version),
        "graph" => {}
        other => {
            return Err(Error::usage(format!(
                "unknown command {other:?}; see --help"
            )));
        }
    }

    let mut rev = None;
    let mut repo = PathBuf::from(".");
    let mut max_count = 1000;
    let mut only_positional = false;
    while let Some(arg) = args.next() {
        if only_positional || !arg.starts_with('-') || arg == "-" {
            if rev.replace(arg).is_some() {
                return Err(Error::usage("graph takes one REV"));
            }
            continue;
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_owned(), Some(value.to_owned()))
            }
            _ => (arg, None),
        };
        let mut value = |name: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| Error::usage(format!("{name} needs a value")))
        };
        match flag.as_str() {
            "--" => only_positional = true,
            "-h" | "--help" => return Ok(Command::Help),
            "--repo" => repo = PathBuf::from(value("--repo")?),
            "-n" | "--max-count" => {
                let v = value("--max-count")?;
                max_count = v.parse().ok().filter(|&n: &usize| n > 0).ok_or_else(|| {
                    Error::usage(format!("--max-count must be a positive number, not {v:?}"))
                })?;
            }
            _ => return Err(Error::usage(format!("unknown option {flag:?}; see --help"))),
        }
    }
    Ok(Command::Graph {
        rev: rev.unwrap_or_else(|| "HEAD".to_owned()),
        repo,
        max_count,
    })
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
