//! Reading source code: tokens, and the items (functions, classes, impls, ...) of a file.
//! Hand-written for Python, Rust and C; other files are not read.

mod c;
mod lex;
mod python;
mod rust;

use serde::Serialize;

pub use lex::{Kind, Token, lex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Rust,
    C,
}

impl Language {
    /// The language of a file, from its extension.
    pub fn from_path(path: &str) -> Option<Self> {
        let name = path.rsplit('/').next().unwrap_or(path);
        match name.rsplit_once('.')?.1 {
            "py" | "pyi" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "c" | "h" => Some(Language::C),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Rust => "rust",
            Language::C => "c",
        }
    }
}

/// A function, class, impl, ... and the lines it spans, 1-based and inclusive. Attributes,
/// decorators and doc comments in front belong to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    /// function, class, struct, enum, union, trait, impl, module, const, static, type or macro.
    pub kind: &'static str,
    pub name: String,
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Item>,
}

/// The items of `source`, in order, nested where the language nests them.
pub fn items(language: Language, source: &str) -> Vec<Item> {
    let tokens = lex(language, source);
    match language {
        Language::Python => python::items(source, &tokens),
        Language::Rust => rust::items(source, &tokens),
        Language::C => c::items(source, &tokens),
    }
}

/// For each token, the index of its matching bracket, if it is a bracket with a match.
fn matching(source: &str, tokens: &[Token]) -> Vec<Option<usize>> {
    let mut matches = vec![None; tokens.len()];
    let mut open: Vec<(usize, &str)> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != Kind::Punct {
            continue;
        }
        let closes = match token.text(source) {
            "(" | "[" | "{" => {
                open.push((i, token.text(source)));
                continue;
            }
            ")" => "(",
            "]" => "[",
            "}" => "{",
            _ => continue,
        };
        // Unbalanced code: drop unmatched openers until one matches.
        if let Some(depth) = open.iter().rposition(|&(_, text)| text == closes) {
            let (start, _) = open[depth];
            open.truncate(depth);
            matches[start] = Some(i);
            matches[i] = Some(start);
        }
    }
    matches
}

/// The text of `tokens` with whitespace between them collapsed to one space.
fn join(source: &str, tokens: &[Token]) -> String {
    let mut out = String::new();
    for (i, token) in tokens.iter().enumerate() {
        if i > 0 && tokens[i - 1].end < token.start {
            out.push(' ');
        }
        out.push_str(token.text(source));
    }
    out
}

#[cfg(test)]
mod tests;
