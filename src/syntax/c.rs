//! C items: function definitions, `struct`, `union` and `enum` definitions (typedefs too), and
//! `#define`. `extern "C" { ... }` blocks in headers are read through.

use super::{Item, Kind, Token, matching};

/// Identifiers whose parentheses are not a function's parameters.
const NOT_NAMES: [&str; 5] = ["__attribute__", "__declspec", "__asm__", "asm", "alignas"];

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    matches: Vec<Option<usize>>,
}

impl Parser<'_> {
    fn text(&self, i: usize) -> &str {
        self.tokens.get(i).map_or("", |t| t.text(self.source))
    }

    fn skip_group(&self, i: usize) -> usize {
        self.matches[i].map_or(i + 1, |m| m + 1)
    }

    /// The `#define`d name, if the directive at `i` is one.
    fn define(&self, i: usize) -> Option<String> {
        let rest = self.text(i).strip_prefix('#')?.trim_start();
        let rest = rest.strip_prefix("define")?;
        if !rest.starts_with([' ', '\t']) {
            return None;
        }
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        (!name.is_empty()).then_some(name)
    }

    /// The type whose body opens at `brace`, declared from `start`: its kind and tag, if the
    /// brace follows `struct`, `union` or `enum` (and maybe a tag).
    fn record(&self, start: usize, brace: usize) -> Option<(&'static str, Option<String>)> {
        let kind = |i: usize| match self.text(i) {
            "struct" => Some("struct"),
            "union" => Some("union"),
            "enum" => Some("enum"),
            _ => None,
        };
        if brace > start
            && let Some(kind) = kind(brace - 1)
        {
            return Some((kind, None));
        }
        if brace > start + 1
            && self.tokens[brace - 1].kind == Kind::Ident
            && let Some(kind) = kind(brace - 2)
        {
            return Some((kind, Some(self.text(brace - 1).to_owned())));
        }
        None
    }

    /// The function name in the declaration `start..brace`: the identifier before the first
    /// parameter list. In `int (*pick(int which))(int)`, a function returning a function
    /// pointer, it is inside the parentheses after `*`.
    fn function(&self, start: usize, brace: usize) -> Option<String> {
        let mut k = start;
        while k < brace {
            if self.text(k) == "("
                && self.text(k + 1) == "*"
                && let Some(close) = self.matches[k]
                && let Some(name) = self.function(k + 2, close)
            {
                return Some(name);
            }
            if self.text(k) == "(" {
                let name = k
                    .checked_sub(1)
                    .filter(|&p| p >= start && self.tokens[p].kind == Kind::Ident)
                    .map(|p| self.text(p));
                match name {
                    Some(name) if !NOT_NAMES.contains(&name) => return Some(name.to_owned()),
                    _ => {}
                }
            }
            k = match self.text(k) {
                "(" | "[" => self.skip_group(k),
                _ => k + 1,
            };
        }
        None
    }

    fn items(&self) -> Vec<Item> {
        let mut items = Vec::new();
        let mut start: Option<usize> = None;
        // Open `extern "C" {` blocks.
        let mut externs = 0;
        let mut i = 0;
        while i < self.tokens.len() {
            let token = self.tokens[i];
            match token.kind {
                Kind::Comment => {
                    i += 1;
                    continue;
                }
                Kind::Directive => {
                    if let Some(name) = self.define(i) {
                        items.push(Item {
                            kind: "macro",
                            name,
                            start: token.line,
                            end: token.end_line,
                            children: Vec::new(),
                        });
                    }
                    i += 1;
                    continue;
                }
                _ => {}
            }
            match self.text(i) {
                ";" => {
                    start = None;
                    i += 1;
                }
                "}" if externs > 0 => {
                    externs -= 1;
                    start = None;
                    i += 1;
                }
                "(" | "[" => {
                    start.get_or_insert(i);
                    i = self.skip_group(i);
                }
                "{" => {
                    let first = start.unwrap_or(i);
                    let close = self.matches[i];
                    if i == first + 2
                        && self.text(first) == "extern"
                        && self.tokens[first + 1].kind == Kind::String
                    {
                        externs += 1;
                        start = None;
                        i += 1;
                        continue;
                    }
                    let initializer = (first..i).any(|k| self.text(k) == "=");
                    if !initializer && let Some((kind, tag)) = self.record(first, i) {
                        // The declaration goes on to its `;`: `typedef struct {...} name;`.
                        let mut k = self.skip_group(i);
                        while k < self.tokens.len() && self.text(k) != ";" {
                            k = match self.text(k) {
                                "(" | "[" | "{" => self.skip_group(k),
                                _ => k + 1,
                            };
                        }
                        let declarator = (self.skip_group(i)..k)
                            .rev()
                            .find(|&d| self.tokens[d].kind == Kind::Ident);
                        let typedef = (first..i).any(|d| self.text(d) == "typedef");
                        let name = match (typedef, declarator, tag) {
                            (true, Some(d), _) | (false, Some(d), None) => self.text(d).to_owned(),
                            (_, _, Some(tag)) => tag,
                            (_, None, None) => String::new(),
                        };
                        let last = k.min(self.tokens.len() - 1);
                        items.push(Item {
                            kind,
                            name,
                            start: self.tokens[first].line,
                            end: self.tokens[last].end_line,
                            children: Vec::new(),
                        });
                        start = None;
                        i = last + 1;
                        continue;
                    }
                    if !initializer
                        && let (Some(name), Some(close)) = (self.function(first, i), close)
                    {
                        items.push(Item {
                            kind: "function",
                            name,
                            start: self.tokens[first].line,
                            end: self.tokens[close].end_line,
                            children: Vec::new(),
                        });
                        start = None;
                        i = close + 1;
                        continue;
                    }
                    // Something else, like an initializer: on to its `;`.
                    i = self.skip_group(i);
                    if !initializer {
                        start = None;
                    }
                }
                _ => {
                    start.get_or_insert(i);
                    i += 1;
                }
            }
        }
        items
    }
}

pub fn items(source: &str, tokens: &[Token]) -> Vec<Item> {
    Parser {
        source,
        tokens: tokens.to_vec(),
        matches: matching(source, tokens),
    }
    .items()
}

#[cfg(test)]
mod tests;
