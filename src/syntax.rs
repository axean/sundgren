//! Reading source code: tokens, and the items (functions, classes, imports, ...) of a file.
//! Python, Rust, C, C++, JavaScript and TypeScript, parsed with tree-sitter; other files are not
//! read. One engine for every language (`tree.rs`); a language is a grammar and a query.

mod tree;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Identifiers and keywords.
    Ident,
    Number,
    /// String, character and regular expression literals.
    String,
    /// Comments, and strings that only document: a string that is a whole statement.
    Comment,
    /// A C preprocessor line.
    Directive,
    Punct,
}

/// A token: bytes `start..end` of the source, on lines `line..=end_line` (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub end_line: usize,
}

impl Token {
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

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
    /// TypeScript with JSX: its own grammar, shown as TypeScript.
    #[serde(rename = "typescript")]
    Tsx,
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
            "ts" | "mts" | "cts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
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
            Language::TypeScript | Language::Tsx => "typescript",
        }
    }

    /// Between the names of an item and its parent.
    pub fn separator(self) -> &'static str {
        match self {
            Language::Rust | Language::C | Language::Cpp => "::",
            _ => ".",
        }
    }
}

/// A function, class, import, ... and the lines it spans, 1-based and inclusive. Attributes,
/// decorators and doc comments right before it belong to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    /// One of [`KINDS`].
    pub kind: &'static str,
    pub name: String,
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Item>,
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
    tree::items(language, source)
}

/// The tokens of `source`, in order. Whitespace is not a token.
pub fn tokens(language: Language, source: &str) -> Vec<Token> {
    tree::tokens(language, source)
}

#[cfg(test)]
mod tests;
