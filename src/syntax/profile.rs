//! What sundgren knows of each language, as data. The lexer and the reader apply the same rules
//! to every language; a language only says which words and characters those rules look for.
//!
//! C and C++ share one description, as do JavaScript and TypeScript: C++ and TypeScript add
//! words, and reading C or JavaScript with them changes little.

use super::Language;

/// How a block of code is delimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blocks {
    /// `{ ... }`.
    Braces,
    /// A line ending in `:` and the more indented lines after it (Python).
    Indent,
}

/// The string literals of a language, beyond `"..."` and `'...'`.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Strings {
    /// Prefix letters before a quote: `r"..."`, `b'...'`, `L"..."`.
    pub prefixes: &'static [&'static str],
    /// `"""..."""` and `'''...'''`.
    pub triple: bool,
    /// `"..."` may span lines.
    pub multiline: bool,
    /// `'` is a character literal or a lifetime (Rust), not a string.
    pub char_or_lifetime: bool,
    /// Rust raw strings `r#"..."#`.
    pub raw_hashes: bool,
    /// C++ raw strings `R"delim(...)delim"`.
    pub raw_delimited: bool,
    /// Template literals with `${...}` (JavaScript).
    pub templates: bool,
    /// Regular expression literals `/.../flags` (JavaScript).
    pub regex: bool,
}

/// One language family's words and characters.
#[derive(Debug)]
pub(crate) struct Profile {
    /// `$` is a letter in names (JavaScript).
    pub dollar_names: bool,
    pub line_comment: &'static str,
    /// `/* ... */`, and whether those nest.
    pub block_comment: bool,
    pub nested_comments: bool,
    /// Comments that document the item after them.
    pub doc_comments: &'static [&'static str],
    /// `#` at the start of a line starts a preprocessor directive.
    pub directives: bool,
    pub strings: Strings,
    pub blocks: Blocks,
    /// Words that start a definition, and the kind of item it is.
    pub definitions: &'static [(&'static str, &'static str)],
    /// Words that start a binding of a name to a value, and its kind (`const`, `let`, ...).
    pub bindings: &'static [(&'static str, &'static str)],
    /// Words before a definition that don't change what it is: `pub`, `export`, `static`, ...
    pub modifiers: &'static [&'static str],
    /// Words that start an import.
    pub imports: &'static [&'static str],
    /// Control statements: their blocks are read as if they weren't there.
    pub control: &'static [&'static str],
    /// `@decorator` lines belong to the item after them.
    pub decorators: bool,
    /// Attribute brackets that belong to the item after them: `#[...]`, `[[...]]`.
    pub attributes: &'static [(&'static str, &'static str)],
    /// C-style declarations, where the name follows a type and no keyword says what it is.
    pub declarators: bool,
    /// A statement `name = value` binds a name (Python).
    pub assignments: bool,
    /// `name!(...)` is a macro call (Rust).
    pub macro_bang: bool,
    /// Words that make a value a function: `lambda`, `function`; `=>` is one for every language.
    pub function_values: &'static [&'static str],
    /// Between the names of an item and its parent, in containment's names.
    pub separator: &'static str,
    /// Identifiers whose parentheses are not a function's parameters.
    pub not_names: &'static [&'static str],
}

const PYTHON: Profile = Profile {
    dollar_names: false,
    line_comment: "#",
    block_comment: false,
    nested_comments: false,
    doc_comments: &[],
    directives: false,
    strings: Strings {
        prefixes: &[
            "r", "R", "b", "B", "u", "U", "f", "F", "t", "T", "rb", "Rb", "rB", "RB", "br", "Br",
            "bR", "BR", "fr", "Fr", "fR", "FR", "rf", "Rf", "rF", "RF", "tr", "rt",
        ],
        triple: true,
        multiline: false,
        char_or_lifetime: false,
        raw_hashes: false,
        raw_delimited: false,
        templates: false,
        regex: false,
    },
    blocks: Blocks::Indent,
    definitions: &[("def", "function"), ("class", "class"), ("type", "type")],
    bindings: &[],
    modifiers: &["async"],
    imports: &["import", "from"],
    control: &[
        "if", "elif", "else", "for", "while", "try", "except", "finally", "with", "match", "case",
    ],
    decorators: true,
    attributes: &[],
    declarators: false,
    assignments: true,
    macro_bang: false,
    function_values: &["lambda"],
    separator: ".",
    not_names: &[],
};

const RUST: Profile = Profile {
    dollar_names: false,
    line_comment: "//",
    block_comment: true,
    nested_comments: true,
    doc_comments: &["///", "/**"],
    directives: false,
    strings: Strings {
        prefixes: &["b", "c"],
        triple: false,
        multiline: true,
        char_or_lifetime: true,
        raw_hashes: true,
        raw_delimited: false,
        templates: false,
        regex: false,
    },
    blocks: Blocks::Braces,
    definitions: &[
        ("fn", "function"),
        ("struct", "struct"),
        ("enum", "enum"),
        ("union", "union"),
        ("trait", "trait"),
        ("impl", "impl"),
        ("mod", "module"),
        ("type", "type"),
        ("const", "const"),
        ("static", "static"),
        ("macro_rules", "macro"),
    ],
    bindings: &[],
    modifiers: &[
        "pub", "async", "unsafe", "default", "safe", "extern", "auto",
    ],
    imports: &["use"],
    control: &["if", "else", "for", "while", "loop", "match", "unsafe"],
    decorators: false,
    attributes: &[("#[", "]"), ("#![", "]")],
    declarators: false,
    assignments: false,
    macro_bang: true,
    function_values: &[],
    separator: "::",
    not_names: &[],
};

const C_FAMILY: Profile = Profile {
    dollar_names: false,
    line_comment: "//",
    block_comment: true,
    nested_comments: false,
    doc_comments: &["///", "/**", "//!", "/*!"],
    directives: true,
    strings: Strings {
        prefixes: &["L", "u", "U", "u8"],
        triple: false,
        multiline: false,
        char_or_lifetime: false,
        raw_hashes: false,
        raw_delimited: true,
        templates: false,
        regex: false,
    },
    blocks: Blocks::Braces,
    definitions: &[
        ("struct", "struct"),
        ("union", "union"),
        ("enum", "enum"),
        ("class", "class"),
        ("namespace", "module"),
        ("typedef", "type"),
        ("concept", "type"),
    ],
    bindings: &[],
    modifiers: &[
        "static",
        "inline",
        "extern",
        "const",
        "constexpr",
        "consteval",
        "constinit",
        "virtual",
        "explicit",
        "friend",
        "register",
        "volatile",
        "thread_local",
        "_Thread_local",
        "__inline",
        "__inline__",
        "__extension__",
        "mutable",
        "public",
        "private",
        "protected",
        "typename",
    ],
    imports: &["using", "import"],
    control: &["if", "else", "for", "while", "do", "switch", "try", "catch"],
    decorators: false,
    attributes: &[("[[", "]]")],
    declarators: true,
    assignments: false,
    macro_bang: false,
    function_values: &[],
    separator: "::",
    not_names: &[
        "__attribute__",
        "__attribute",
        "__declspec",
        "__asm__",
        "__asm",
        "asm",
        "alignas",
        "_Alignas",
        "noexcept",
        "throw",
        "decltype",
        "sizeof",
        "alignof",
        "requires",
    ],
};

const JS_FAMILY: Profile = Profile {
    dollar_names: true,
    line_comment: "//",
    block_comment: true,
    nested_comments: false,
    doc_comments: &["/**", "///"],
    directives: false,
    strings: Strings {
        prefixes: &[],
        triple: false,
        multiline: false,
        char_or_lifetime: false,
        raw_hashes: false,
        raw_delimited: false,
        templates: true,
        regex: true,
    },
    blocks: Blocks::Braces,
    definitions: &[
        ("function", "function"),
        ("class", "class"),
        ("interface", "interface"),
        ("enum", "enum"),
        ("namespace", "module"),
        ("module", "module"),
        ("type", "type"),
    ],
    bindings: &[("const", "const"), ("let", "variable"), ("var", "variable")],
    modifiers: &[
        "export",
        "default",
        "declare",
        "async",
        "abstract",
        "static",
        "public",
        "private",
        "protected",
        "readonly",
        "override",
        "accessor",
        "get",
        "set",
    ],
    imports: &["import"],
    control: &[
        "if", "else", "for", "while", "do", "switch", "try", "catch", "finally",
    ],
    decorators: true,
    attributes: &[],
    declarators: false,
    assignments: false,
    macro_bang: false,
    function_values: &["function"],
    separator: ".",
    not_names: &[],
};

impl Language {
    pub(crate) fn profile(self) -> &'static Profile {
        match self {
            Language::Python => &PYTHON,
            Language::Rust => &RUST,
            Language::C | Language::Cpp => &C_FAMILY,
            Language::JavaScript | Language::TypeScript => &JS_FAMILY,
        }
    }
}
