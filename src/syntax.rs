//! Reading source code: tokens, and the items (functions, classes, imports, ...) of a file.
//! Hand-written, one lexer and one reader for every language; what differs between languages is
//! data (`profile.rs`). Python, Rust, C, C++, JavaScript and TypeScript; other files are not read.

mod lex;
mod profile;
mod read;

use serde::Serialize;

pub use lex::{Kind, Token, lex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Rust,
    C,
    Cpp,
    #[serde(rename = "javascript")]
    JavaScript,
    #[serde(rename = "typescript")]
    TypeScript,
}

impl Language {
    /// The language of a file, from its extension.
    pub fn from_path(path: &str) -> Option<Self> {
        let name = path.rsplit('/').next().unwrap_or(path);
        match name.rsplit_once('.')?.1 {
            "py" | "pyi" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "c" | "h" => Some(Language::C),
            "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" | "ipp" | "tpp" | "inl" => {
                Some(Language::Cpp)
            }
            "js" | "mjs" | "cjs" | "jsx" => Some(Language::JavaScript),
            "ts" | "mts" | "cts" | "tsx" => Some(Language::TypeScript),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Rust => "rust",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
        }
    }
}

/// A function, class, import, ... and the lines it spans, 1-based and inclusive. Attributes,
/// decorators and doc comments in front belong to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    /// One of [`KINDS`].
    pub kind: &'static str,
    pub name: String,
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Item>,
    /// What it calls, by name: functions, methods, macros. Not what its children call.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<String>,
}

/// Every kind of item, in every language.
pub const KINDS: [&str; 19] = [
    "function",
    "prototype",
    "class",
    "struct",
    "union",
    "enum",
    "variant",
    "field",
    "trait",
    "interface",
    "impl",
    "module",
    "type",
    "const",
    "static",
    "variable",
    "macro",
    "import",
    "call",
];

/// The items of `source`, in order, nested where the language nests them.
pub fn items(language: Language, source: &str) -> Vec<Item> {
    let tokens = lex(language, source);
    read::items(language.profile(), source, &tokens)
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
