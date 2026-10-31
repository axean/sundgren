//! One lexer for Python, Rust and C: what differs is comment, string and directive syntax.
//! Punctuation is one character per token. It never fails: unknown or broken input still comes
//! out as tokens.

use serde::Serialize;

use super::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Identifiers and keywords; Rust lifetimes too.
    Ident,
    Number,
    /// String and character literals, with their prefixes and quotes.
    String,
    Comment,
    /// A C preprocessor line, continuation lines included.
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

struct Lexer<'a> {
    bytes: &'a [u8],
    i: usize,
    line: usize,
    language: Language,
}

/// Bytes of identifiers. Every byte of a non-ASCII character counts, so tokens always end on a
/// character boundary.
fn ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

impl Lexer<'_> {
    fn peek(&self, k: usize) -> u8 {
        self.bytes.get(self.i + k).copied().unwrap_or(0)
    }

    fn at_end(&self) -> bool {
        self.i >= self.bytes.len()
    }

    fn bump(&mut self) {
        if let Some(&b) = self.bytes.get(self.i) {
            if b == b'\n' {
                self.line += 1;
            }
            self.i += 1;
        }
    }

    fn bump_n(&mut self, n: usize) {
        for _ in 0..n {
            self.bump();
        }
    }

    fn starts_with(&self, s: &[u8]) -> bool {
        self.bytes[self.i..].starts_with(s)
    }

    /// Up to the end of the line, not including it.
    fn rest_of_line(&mut self) {
        while !self.at_end() && self.peek(0) != b'\n' {
            self.bump();
        }
    }

    /// A block comment `/* ... */`; nested in Rust.
    fn block_comment(&mut self) {
        let nests = self.language == Language::Rust;
        let mut depth = 0;
        while !self.at_end() {
            if self.starts_with(b"/*") {
                depth += 1;
                self.bump_n(2);
                if !nests && depth > 1 {
                    depth = 1;
                }
            } else if self.starts_with(b"*/") {
                self.bump_n(2);
                depth -= 1;
                if depth == 0 {
                    return;
                }
            } else {
                self.bump();
            }
        }
    }

    /// A quoted literal from its opening quote; backslash escapes the next byte. Ends at the
    /// closing quote, or before a newline if it cannot span lines.
    fn quoted(&mut self, quote: u8, multiline: bool) {
        self.bump();
        while !self.at_end() {
            match self.peek(0) {
                b'\\' => self.bump_n(2),
                b'\n' if !multiline => return,
                b if b == quote => {
                    self.bump();
                    return;
                }
                _ => self.bump(),
            }
        }
    }

    /// A Python triple-quoted string from its opening quotes.
    fn triple_quoted(&mut self, quote: u8) {
        let close = [quote; 3];
        self.bump_n(3);
        while !self.at_end() {
            if self.starts_with(&close) {
                self.bump_n(3);
                return;
            }
            if self.peek(0) == b'\\' {
                self.bump();
            }
            self.bump();
        }
    }

    /// A Rust raw string from its `r`: `r#"..."#` with any number of `#`, no escapes.
    fn raw_string(&mut self) {
        self.bump();
        let mut hashes = 0;
        while self.peek(0) == b'#' {
            hashes += 1;
            self.bump();
        }
        self.bump();
        while !self.at_end() {
            if self.peek(0) == b'"' && (1..=hashes).all(|k| self.peek(k) == b'#') {
                self.bump_n(1 + hashes);
                return;
            }
            self.bump();
        }
    }

    /// A number: digits, letters (hex, suffixes), `_`, a decimal point before a digit, and a
    /// sign after an exponent.
    fn number(&mut self) {
        let start = self.i;
        while !self.at_end() {
            let b = self.peek(0);
            let hex = self.bytes[start..self.i].starts_with(b"0x")
                || self.bytes[start..self.i].starts_with(b"0X");
            let part = (ident_byte(b) && b < 0x80)
                || (b == b'.' && self.peek(1).is_ascii_digit())
                || ((b == b'+' || b == b'-')
                    && !hex
                    && matches!(self.bytes[self.i - 1], b'e' | b'E')
                    && self.peek(1).is_ascii_digit());
            if !part {
                return;
            }
            self.bump();
        }
    }

    fn ident(&mut self) {
        while !self.at_end() && ident_byte(self.peek(0)) {
            self.bump();
        }
    }

    /// The length in bytes of the character at `self.i + k`.
    fn char_len(&self, k: usize) -> usize {
        match self.peek(k) {
            0..0x80 => 1,
            0xc0..0xe0 => 2,
            0xe0..0xf0 => 3,
            _ => 4,
        }
    }

    /// At `'` in Rust: a character literal, or a lifetime or label (an identifier).
    fn rust_quote(&mut self) -> Kind {
        let after = 1 + self.char_len(1);
        if self.peek(1) == b'\\' || (self.peek(1) != b'\'' && self.peek(after) == b'\'') {
            self.quoted(b'\'', false);
            Kind::String
        } else {
            self.bump();
            self.ident();
            Kind::Ident
        }
    }

    /// At an identifier: it, or a string with this prefix.
    fn ident_or_string(&mut self) -> Kind {
        let start = self.i;
        self.ident();
        let prefix = &self.bytes[start..self.i];
        let next = self.peek(0);
        match self.language {
            Language::Python => {
                let is_prefix = prefix.len() <= 2
                    && prefix.iter().all(|b| b"rRbBuUfFtT".contains(b))
                    && (next == b'"' || next == b'\'');
                if is_prefix {
                    self.python_string();
                    return Kind::String;
                }
            }
            Language::Rust => {
                let raw = matches!(prefix, b"r" | b"br" | b"cr");
                if raw && (next == b'"' || (next == b'#' && matches!(self.peek(1), b'"' | b'#'))) {
                    self.i -= 1;
                    self.raw_string();
                    return Kind::String;
                }
                if prefix == b"r" && next == b'#' && ident_byte(self.peek(1)) {
                    self.bump();
                    self.ident();
                    return Kind::Ident;
                }
                if matches!(prefix, b"b" | b"c") && next == b'"' {
                    self.quoted(b'"', true);
                    return Kind::String;
                }
                if prefix == b"b" && next == b'\'' {
                    self.quoted(b'\'', false);
                    return Kind::String;
                }
            }
            Language::C => {
                if matches!(prefix, b"L" | b"u" | b"U" | b"u8") && (next == b'"' || next == b'\'') {
                    self.quoted(next, false);
                    return Kind::String;
                }
            }
        }
        Kind::Ident
    }

    /// A Python string from its opening quote.
    fn python_string(&mut self) {
        let quote = self.peek(0);
        if self.peek(1) == quote && self.peek(2) == quote {
            self.triple_quoted(quote);
        } else {
            self.quoted(quote, false);
        }
    }

    /// A C preprocessor line from its `#`, with continuation lines; it stops before a comment.
    fn directive(&mut self) {
        while !self.at_end() {
            match self.peek(0) {
                b'\n' => return,
                b'\\' if self.peek(1) == b'\n' => self.bump_n(2),
                b'\\' if self.peek(1) == b'\r' && self.peek(2) == b'\n' => self.bump_n(3),
                b'/' if matches!(self.peek(1), b'/' | b'*') => return,
                _ => self.bump(),
            }
        }
    }

    /// The token at `self.i`, which is not whitespace.
    fn token(&mut self, line_start: bool) -> Kind {
        let b = self.peek(0);
        let language = self.language;
        match b {
            b'#' if language == Language::Python => {
                self.rest_of_line();
                Kind::Comment
            }
            b'#' if language == Language::C && line_start => {
                self.directive();
                Kind::Directive
            }
            b'/' if language != Language::Python && self.peek(1) == b'/' => {
                self.rest_of_line();
                Kind::Comment
            }
            b'/' if language != Language::Python && self.peek(1) == b'*' => {
                self.block_comment();
                Kind::Comment
            }
            b'"' | b'\'' if language == Language::Python => {
                self.python_string();
                Kind::String
            }
            b'\'' if language == Language::Rust => self.rust_quote(),
            b'"' => {
                self.quoted(b'"', language == Language::Rust);
                Kind::String
            }
            b'\'' => {
                self.quoted(b'\'', false);
                Kind::String
            }
            b'0'..=b'9' => {
                self.number();
                Kind::Number
            }
            _ if ident_byte(b) => self.ident_or_string(),
            _ => {
                self.bump();
                Kind::Punct
            }
        }
    }
}

/// The tokens of `source`. Whitespace, and backslash line continuations, are left out.
pub fn lex(language: Language, source: &str) -> Vec<Token> {
    let mut lexer = Lexer {
        bytes: source.as_bytes(),
        i: 0,
        line: 1,
        language,
    };
    let mut tokens = Vec::new();
    let mut line_start = true;
    while !lexer.at_end() {
        let b = lexer.peek(0);
        if b == b'\n' {
            line_start = true;
            lexer.bump();
            continue;
        }
        if b.is_ascii_whitespace() {
            lexer.bump();
            continue;
        }
        if b == b'\\' && (lexer.peek(1) == b'\n' || lexer.starts_with(b"\\\r\n")) {
            lexer.bump_n(if lexer.peek(1) == b'\n' { 2 } else { 3 });
            continue;
        }
        let (start, line) = (lexer.i, lexer.line);
        let kind = lexer.token(line_start);
        line_start = false;
        let end_line = if lexer.bytes[start..lexer.i].ends_with(b"\n") {
            lexer.line - 1
        } else {
            lexer.line
        };
        tokens.push(Token {
            kind,
            start,
            end: lexer.i,
            line,
            end_line,
        });
    }
    tokens
}

#[cfg(test)]
mod tests;
