//! Command line: `sundgren graph`, `sundgren contain`, `sundgren outline`.

use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;

use crate::contain::{self, Comments};
use crate::git;
use crate::map;
use crate::syntax::{self, Item, Kind, Language};
use crate::{EXIT_OK, Error, Result};

const USAGE: &str = "\
Git history as a graph, and how much work branches share.

Usage: sundgren graph [--base REV] [--show MODE] [--like BRANCH] [--branches N] [--commits N]
                      [--since DATE] [--until DATE] [--protected PATTERN]... [--pin BRANCH]...
                      [--hide BRANCH]... [--fuzzy T] [--format json|text]
       sundgren contain A B --base REV [--repo DIR] [--fuzzy T] [--comments ignore|count]
                        [--lines] [--format json|text]
       sundgren outline FILE... [--tokens] [--format text|json]

graph (prototype): a map of branches around the tip of the base, time running down, as
sundgren.graph/3 JSON or as Unicode.
  --base REV          the reference lane (default: the remote's default branch, main or master)
  --show MODE         which other branches: local (default: local ones first), recent, mine
                      (commits by git user.email first), like (most contained in --like)
  --like BRANCH       for --show like; implies it
  --branches N        how many branches besides the base and pinned ones (default 12)
  --commits N         how many commits of the base the page goes back (default 50)
  --since DATE        the page starts here instead (unix seconds, or as git log --since)
  --until DATE        the page ends here (default: now)
  --protected PATTERN branches drawn as protected; repeatable, * matches anything
                      (default main and master)
  --pin BRANCH        always show BRANCH; repeatable
  --hide BRANCH       never show BRANCH (not the base or --like); repeatable
  --fuzzy T           for --show like, as for contain (default 0.9)
  --format FORMAT     json (default) or text

contain (prototype): how much of the work A and B each added since they left the base is found
in the other, as sundgren.containment/0 JSON.
  --base REV          the base both are compared from, e.g. main
  --fuzzy T           a line also counts as found if one is at least T similar (normalized
                      Levenshtein, 0 to 1; default 0.9; 1 for exact only)
  --comments MODE     ignore (default): lines that are only a comment, in Python, Rust and C
                      files, are not work; count: they are
  --lines             also list every added line and where it was found
  --format FORMAT     json (default) or text

outline (prototype): the items (functions, classes, impls, ...) of Python, Rust and C files, as
read by sundgren, with the lines each spans.
  --tokens            also list the tokens, line by line
  --format FORMAT     text (default) or json (sundgren.outline/0)

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
        repo: PathBuf,
        options: map::Options,
        format: Format,
    },
    Contain {
        a: String,
        b: String,
        base: String,
        repo: PathBuf,
        fuzzy: f64,
        comments: Comments,
        lines: bool,
        format: Format,
    },
    Outline {
        files: Vec<PathBuf>,
        tokens: bool,
        format: Format,
    },
}

/// A subcommand's arguments: positionals, and `--option value` pairs (short forms expanded).
struct Arguments {
    positional: Vec<String>,
    options: Vec<(String, String)>,
    /// Options without a value that were given.
    switches: Vec<String>,
}

impl Arguments {
    fn all(&self, name: &str) -> Vec<String> {
        self.options
            .iter()
            .filter(|(flag, _)| flag == name)
            .map(|(_, value)| value.clone())
            .collect()
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Split the arguments after a subcommand: options in `known` take a value, those in
/// `switches` do not. None if they ask for help.
fn split(
    args: impl Iterator<Item = String>,
    known: &[&str],
    switches: &[&str],
) -> Result<Option<Arguments>> {
    let mut args = args;
    let mut parsed = Arguments {
        positional: Vec::new(),
        options: Vec::new(),
        switches: Vec::new(),
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
        if switches.contains(&flag.as_str()) && inline.is_none() {
            parsed.switches.push(flag);
            continue;
        }
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
            let known = [
                "--repo",
                "--base",
                "--show",
                "--like",
                "--branches",
                "--commits",
                "--protected",
                "--pin",
                "--hide",
                "--since",
                "--until",
                "--fuzzy",
                "--format",
            ];
            let Some(parsed) = split(args, &known, &[])? else {
                return Ok(Command::Help);
            };
            if !parsed.positional.is_empty() {
                return Err(Error::usage(
                    "graph takes no positional arguments; see --help",
                ));
            }
            let count = |flag: &str, default: usize, least: usize, most: usize| -> Result<usize> {
                match parsed.get(flag) {
                    None => Ok(default),
                    Some(v) => v
                        .parse()
                        .ok()
                        .filter(|n: &usize| (least..=most).contains(n))
                        .ok_or_else(|| {
                            Error::usage(format!(
                                "{flag} must be a number from {least} to {most}, not {v:?}"
                            ))
                        }),
                }
            };
            let like = parsed.get("--like").map(str::to_owned);
            let show = match (parsed.get("--show"), like) {
                (None | Some("like"), Some(branch)) => map::Show::Like(branch),
                (Some("like"), None) => {
                    return Err(Error::usage("--show like needs --like BRANCH"));
                }
                (None | Some("local"), None) => map::Show::Local,
                (Some("recent"), None) => map::Show::Recent,
                (Some("mine"), None) => map::Show::Mine,
                (Some(mode @ ("local" | "recent" | "mine")), Some(_)) => {
                    return Err(Error::usage(format!(
                        "--like goes with --show like, not {mode}"
                    )));
                }
                (Some(other), _) => {
                    return Err(Error::usage(format!(
                        "--show must be local, recent, mine or like, not {other:?}"
                    )));
                }
            };
            let protected = parsed.all("--protected");
            Ok(Command::Graph {
                repo: PathBuf::from(parsed.get("--repo").unwrap_or(".")),
                options: map::Options {
                    base: parsed.get("--base").unwrap_or_default().to_owned(),
                    show,
                    branches: count("--branches", 12, 0, 100)?,
                    commits: count("--commits", 50, 1, usize::MAX)?,
                    protected: if protected.is_empty() {
                        vec!["main".to_owned(), "master".to_owned()]
                    } else {
                        protected
                    },
                    pins: parsed.all("--pin"),
                    hides: parsed.all("--hide"),
                    since: parsed.get("--since").map(str::to_owned),
                    until: parsed.get("--until").map(str::to_owned),
                    threshold: fuzzy(parsed.get("--fuzzy"))?,
                },
                format: format(parsed.get("--format"), Format::Json)?,
            })
        }
        "contain" => {
            let known = ["--repo", "--base", "--fuzzy", "--comments", "--format"];
            let Some(parsed) = split(args, &known, &["--lines"])? else {
                return Ok(Command::Help);
            };
            let [a, b] = parsed.positional.as_slice() else {
                return Err(Error::usage("contain takes two revisions, A and B"));
            };
            let base = parsed
                .get("--base")
                .ok_or_else(|| Error::usage("contain needs --base"))?;
            let fuzzy = fuzzy(parsed.get("--fuzzy"))?;
            let format = format(parsed.get("--format"), Format::Json)?;
            let comments = match parsed.get("--comments") {
                None | Some("ignore") => Comments::Ignore,
                Some("count") => Comments::Count,
                Some(other) => {
                    return Err(Error::usage(format!(
                        "--comments must be ignore or count, not {other:?}"
                    )));
                }
            };
            Ok(Command::Contain {
                a: a.clone(),
                b: b.clone(),
                base: base.to_owned(),
                repo: PathBuf::from(parsed.get("--repo").unwrap_or(".")),
                fuzzy,
                comments,
                lines: parsed.switches.iter().any(|s| s == "--lines"),
                format,
            })
        }
        "outline" => {
            let Some(parsed) = split(args, &["--format"], &["--tokens"])? else {
                return Ok(Command::Help);
            };
            if parsed.positional.is_empty() {
                return Err(Error::usage("outline takes one or more files"));
            }
            Ok(Command::Outline {
                files: parsed.positional.iter().map(PathBuf::from).collect(),
                tokens: parsed.switches.iter().any(|s| s == "--tokens"),
                format: format(parsed.get("--format"), Format::Text)?,
            })
        }
        other => Err(Error::usage(format!(
            "unknown command {other:?}; see --help"
        ))),
    }
}

fn fuzzy(value: Option<&str>) -> Result<f64> {
    match value {
        None => Ok(0.9),
        Some(v) => v
            .parse()
            .ok()
            .filter(|t: &f64| (0.0..=1.0).contains(t))
            .ok_or_else(|| {
                Error::usage(format!("--fuzzy must be a number from 0 to 1, not {v:?}"))
            }),
    }
}

fn format(value: Option<&str>, default: Format) -> Result<Format> {
    match value {
        None => Ok(default),
        Some("json") => Ok(Format::Json),
        Some("text") => Ok(Format::Text),
        Some(other) => Err(Error::usage(format!(
            "--format must be json or text, not {other:?}"
        ))),
    }
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

pub const OUTLINE_SCHEMA: &str = "sundgren.outline/0";

/// One token, for `outline --tokens`.
#[derive(Debug, Serialize)]
pub struct TokenOut {
    pub kind: Kind,
    pub line: usize,
    pub end_line: usize,
    pub text: String,
}

/// One file of the `sundgren.outline/0` document. `language` is left out if sundgren does not
/// read the file's language, and then it has no items.
#[derive(Debug, Serialize)]
pub struct FileOutline {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    pub items: Vec<Item>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<TokenOut>>,
}

#[derive(Debug, Serialize)]
pub struct OutlineDocument {
    pub schema: &'static str,
    pub files: Vec<FileOutline>,
}

/// How sundgren reads each of `files`.
pub fn outline_document(files: &[PathBuf], tokens: bool) -> Result<OutlineDocument> {
    let files = files
        .iter()
        .map(|path| {
            let bytes = std::fs::read(path)
                .map_err(|e| Error::failure(format!("{}: {e}", path.display())))?;
            let source = String::from_utf8_lossy(&bytes);
            let language = Language::from_path(&path.to_string_lossy());
            let (items, tokens) = match language {
                None => (Vec::new(), None),
                Some(language) => (
                    syntax::items(language, &source),
                    tokens.then(|| {
                        syntax::lex(language, &source)
                            .iter()
                            .map(|t| TokenOut {
                                kind: t.kind,
                                line: t.line,
                                end_line: t.end_line,
                                text: t.text(&source).to_owned(),
                            })
                            .collect()
                    }),
                ),
            };
            Ok(FileOutline {
                path: path.clone(),
                language,
                items,
                tokens,
            })
        })
        .collect::<Result<_>>()?;
    Ok(OutlineDocument {
        schema: OUTLINE_SCHEMA,
        files,
    })
}

fn outline_items(out: &mut String, items: &[Item], depth: usize) {
    for item in items {
        let lines = format!("{}-{}", item.start, item.end);
        out.push_str(&format!(
            "{:indent$}{lines:<10} {} {}\n",
            "",
            item.kind,
            item.name,
            indent = 2 + 2 * depth
        ));
        outline_items(out, &item.children, depth + 1);
    }
}

/// A token as `outline --tokens` shows it: identifiers, numbers and punctuation as they are,
/// the others in brackets with their kind, newlines as ⏎, long ones shortened.
fn show_token(token: &TokenOut) -> String {
    let text = token.text.replace('\n', "⏎");
    let text = if text.chars().count() > 40 {
        format!("{}…", text.chars().take(39).collect::<String>())
    } else {
        text
    };
    match token.kind {
        Kind::Ident | Kind::Number | Kind::Punct => text,
        Kind::String => format!("[string {text}]"),
        Kind::Comment => format!("[comment {text}]"),
        Kind::Directive => format!("[directive {text}]"),
    }
}

/// The outline as text: per file, its items with their lines, nested; and the tokens if asked.
pub fn outline_text(doc: &OutlineDocument) -> String {
    let mut out = String::new();
    for file in &doc.files {
        let Some(language) = file.language else {
            out.push_str(&format!(
                "{}: not read (unknown language)\n",
                file.path.display()
            ));
            continue;
        };
        out.push_str(&format!("{} ({})\n", file.path.display(), language.name()));
        outline_items(&mut out, &file.items, 0);
        if let Some(tokens) = &file.tokens {
            out.push_str("  tokens:\n");
            let mut line = 0;
            for token in tokens {
                if token.line != line {
                    if line != 0 {
                        out.push('\n');
                    }
                    line = token.line;
                    out.push_str(&format!("  {line:>5} "));
                }
                out.push(' ');
                out.push_str(&show_token(token));
            }
            if line != 0 {
                out.push('\n');
            }
        }
    }
    out
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
            comments,
            lines,
            format,
        } => {
            // 1.0 means exact only: fuzzy matching is off above 1.
            let threshold = if *fuzzy >= 1.0 { 2.0 } else { *fuzzy };
            let options = contain::Options {
                threshold,
                comments: *comments,
                lines: *lines,
            };
            let mut doc = contain::containment_document(repo, base, a, b, options)?;
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
        Command::Outline {
            files,
            tokens,
            format,
        } => {
            let doc = outline_document(files, *tokens)?;
            let mut out = std::io::stdout().lock();
            match format {
                Format::Json => {
                    serde_json::to_writer(&mut out, &doc)
                        .map_err(|e| Error::failure(e.to_string()))?;
                    writeln!(out)?;
                }
                Format::Text => write!(out, "{}", outline_text(&doc))?,
            }
            Ok(())
        }
        Command::Graph {
            repo,
            options,
            format,
        } => {
            let mut options = options.clone();
            if options.base.is_empty() {
                options.base = git::default_branch(repo)?;
            }
            let doc = map::map_document(repo, &options)?;
            let mut out = std::io::stdout().lock();
            match format {
                Format::Json => {
                    serde_json::to_writer(&mut out, &doc)
                        .map_err(|e| Error::failure(e.to_string()))?;
                    writeln!(out)?;
                }
                Format::Text => write!(out, "{}", map::text(&doc))?,
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
