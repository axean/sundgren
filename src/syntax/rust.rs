//! Rust items: `fn`, `struct`, `enum`, `union`, `trait`, `impl`, `mod`, `const`, `static`,
//! `type` and `macro_rules!`, with their attributes, doc comments and qualifiers. Items inside
//! `impl`, `trait` and `mod` blocks are children; items inside function bodies are not read.

use super::{Item, Kind, Token, join, matching};

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    matches: Vec<Option<usize>>,
}

/// Comments that belong to the item after them.
fn doc_comment(text: &str) -> bool {
    (text.starts_with("///") && !text.starts_with("////"))
        || (text.starts_with("/**") && !text.starts_with("/***") && text != "/**/")
}

impl Parser<'_> {
    fn text(&self, i: usize) -> &str {
        self.tokens.get(i).map_or("", |t| t.text(self.source))
    }

    fn is_ident(&self, i: usize) -> bool {
        self.tokens.get(i).is_some_and(|t| t.kind == Kind::Ident)
    }

    /// Past the bracket group opening at `i`, or just past `i` if it has no match.
    fn skip_group(&self, i: usize) -> usize {
        self.matches[i].map_or(i + 1, |m| m + 1)
    }

    /// The kind of item whose keyword is at `i`, if any.
    fn keyword(&self, i: usize) -> Option<&'static str> {
        Some(match self.text(i) {
            "fn" => "function",
            "struct" => "struct",
            "enum" => "enum",
            "trait" => "trait",
            "impl" => "impl",
            "mod" => "module",
            "type" => "type",
            "static" => "static",
            "const" if !matches!(self.text(i + 1), "fn" | "unsafe" | "async" | "extern") => "const",
            "union" if self.is_ident(i + 1) => "union",
            "macro_rules" if self.text(i + 1) == "!" => "macro",
            _ => return None,
        })
    }

    /// Qualifiers before an item keyword: past the one at `i`, or None if there is none.
    fn qualifier(&self, i: usize) -> Option<usize> {
        match self.text(i) {
            "pub" if self.text(i + 1) == "(" => Some(self.skip_group(i + 1)),
            "pub" | "async" | "unsafe" | "default" | "safe" | "const" => Some(i + 1),
            "extern"
                if self
                    .tokens
                    .get(i + 1)
                    .is_some_and(|t| t.kind == Kind::String) =>
            {
                Some(i + 2)
            }
            "extern" => Some(i + 1),
            _ => None,
        }
    }

    /// The name of the item whose keyword is at `i`.
    fn name(&self, kind: &str, i: usize, body: usize) -> String {
        match kind {
            "impl" => {
                let mut first = i + 1;
                if self.text(first) == "<" {
                    first = self.past_generics(first);
                }
                let last = (first..body)
                    .find(|&k| self.text(k) == "where")
                    .unwrap_or(body);
                join(self.source, &self.tokens[first.min(last)..last])
            }
            "macro" => self.text(i + 2).to_owned(),
            "static" if self.text(i + 1) == "mut" => self.text(i + 2).to_owned(),
            _ => self.text(i + 1).to_owned(),
        }
    }

    /// Past the generic parameters `<...>` opening at `i`.
    fn past_generics(&self, i: usize) -> usize {
        let mut depth = 0;
        let mut k = i;
        while k < self.tokens.len() {
            match self.text(k) {
                "<" => depth += 1,
                // `->` in `Fn() -> T` does not close anything.
                ">" if self.text(k - 1) != "-"
                    || self.tokens[k - 1].end != self.tokens[k].start =>
                {
                    depth -= 1;
                    if depth == 0 {
                        return k + 1;
                    }
                }
                "{" | ";" => return k,
                _ => {}
            }
            k += 1;
        }
        k
    }

    /// Where the item whose keyword is at `i` ends, and its `{` body, if it has one.
    fn extent(&self, kind: &str, i: usize, end: usize) -> (usize, Option<usize>) {
        let mut k = i + 1;
        if kind == "macro" {
            // macro_rules! name { ... } or ( ... );
            k = (i + 3).min(end);
            let close = self.matches.get(k).copied().flatten().unwrap_or(k);
            let semicolon = self.text(close + 1) == ";";
            return (if semicolon { close + 1 } else { close }, None);
        }
        let until_semicolon = matches!(kind, "const" | "static" | "type");
        while k < end {
            match self.text(k) {
                "(" | "[" => k = self.skip_group(k),
                "{" if until_semicolon => k = self.skip_group(k),
                "{" => return (self.matches[k].unwrap_or(end - 1), Some(k)),
                ";" => return (k, None),
                _ => k += 1,
            }
        }
        (end - 1, None)
    }

    /// The items among tokens `i..end`.
    fn items(&self, mut i: usize, end: usize) -> Vec<Item> {
        let mut items = Vec::new();
        let mut start: Option<usize> = None;
        while i < end {
            let token = self.tokens[i];
            if token.kind == Kind::Comment {
                if doc_comment(token.text(self.source)) {
                    start.get_or_insert(i);
                }
                i += 1;
                continue;
            }
            if self.text(i) == "#" && self.text(i + 1) == "[" {
                start.get_or_insert(i);
                i = self.skip_group(i + 1);
                continue;
            }
            if self.text(i) == "#" && self.text(i + 1) == "!" && self.text(i + 2) == "[" {
                i = self.skip_group(i + 2);
                continue;
            }
            if let Some(kind) = self.keyword(i) {
                let (last, body) = self.extent(kind, i, end);
                let body_end = body.unwrap_or(last);
                let children = match (kind, body) {
                    ("impl" | "trait" | "module", Some(open)) => self.items(open + 1, last),
                    _ => Vec::new(),
                };
                items.push(Item {
                    kind,
                    name: self.name(kind, i, body_end),
                    start: self.tokens[start.unwrap_or(i)].line,
                    end: self.tokens[last.min(end - 1)].end_line,
                    children,
                });
                start = None;
                i = last + 1;
                continue;
            }
            if let Some(next) = self.qualifier(i) {
                start.get_or_insert(i);
                i = next;
                continue;
            }
            match self.text(i) {
                "{" | "(" | "[" => i = self.skip_group(i),
                _ => i += 1,
            }
            if self.text(i - 1) == ";" || self.text(i - 1) == "}" {
                start = None;
            }
        }
        items
    }
}

pub fn items(source: &str, tokens: &[Token]) -> Vec<Item> {
    let parser = Parser {
        source,
        tokens: tokens.to_vec(),
        matches: matching(source, tokens),
    };
    parser.items(0, tokens.len())
}

#[cfg(test)]
mod tests;
