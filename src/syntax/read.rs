//! One reader for every language: the code is split into statements and blocks (braces, or
//! indentation), and each statement is classified by the same rules; a language's profile only
//! says which words and characters they look for.
//!
//! The rules, in order, for a statement after its decorators, attributes and modifiers:
//!
//! 1. A control statement (`if`, `for`, `try`, ...) is read through: its blocks are read as if
//!    they were at the same level.
//! 2. An import makes an `import` item.
//! 3. A definition word (`fn`, `def`, `class`, `struct`, ...) makes that kind of item. Classes,
//!    traits, impls and modules hold items; structs, unions and enums hold fields or variants;
//!    functions hold only the definitions inside them. A function without a body is a
//!    `prototype`.
//! 4. A binding (`const x = ...`, Python's `x = ...`) makes a `const` or `variable`, or a
//!    `function` if its value is one (`=>`, `lambda`, `function`). In a class it is a `field`.
//! 5. A C-style declaration, where the name follows a type: a function, prototype or variable.
//! 6. A statement that is only a call, `name(...)` or `name!{...}`, makes a `call` item: often a
//!    macro or registration that defines something.
//!
//! Inside a function only definitions are items. Every item lists the names it calls.

use super::profile::{Blocks, Profile};
use super::{Item, Kind, Token, join};

/// Where statements are read: what kinds of item they can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// A file, module or namespace.
    Top,
    /// The body of a class, trait, impl, interface, struct or union.
    Members,
    /// A function body: only definitions are items.
    Function,
}

/// A statement: its code tokens `first..=last`, and the blocks it holds, as code ranges.
#[derive(Debug, Clone)]
struct Statement {
    first: usize,
    last: usize,
    blocks: Vec<(usize, usize)>,
}

/// Kinds of definition that hold fields or variants, split by `,` (Rust, TypeScript enums).
const RECORDS: [&str; 3] = ["struct", "union", "enum"];

struct Reader<'a> {
    profile: &'static Profile,
    source: &'a str,
    tokens: &'a [Token],
    /// Indices into `tokens` of the code the reader works on: no comments (but documentation
    /// strings, to keep Python's blocks whole), no directives, no `#else` branches.
    code: Vec<usize>,
    /// For each code position, the code position of its matching bracket.
    matches: Vec<Option<usize>>,
    /// Python's logical lines: code ranges, and the indentation of each.
    lines: Vec<(usize, usize, usize)>,
}

impl<'a> Reader<'a> {
    fn new(profile: &'static Profile, source: &'a str, tokens: &'a [Token]) -> Self {
        let code = code_tokens(profile, source, tokens);
        let mut reader = Reader {
            profile,
            source,
            tokens,
            code,
            matches: Vec::new(),
            lines: Vec::new(),
        };
        reader.matches = reader.matching();
        if profile.blocks == Blocks::Indent {
            reader.lines = reader.logical_lines();
        }
        reader
    }

    fn token(&self, c: usize) -> Option<&Token> {
        self.code.get(c).map(|&i| &self.tokens[i])
    }

    fn text(&self, c: usize) -> &'a str {
        self.code
            .get(c)
            .map_or("", |&i| self.tokens[i].text(self.source))
    }

    fn kind(&self, c: usize) -> Option<Kind> {
        self.token(c).map(|t| t.kind)
    }

    fn is_ident(&self, c: usize) -> bool {
        self.kind(c) == Some(Kind::Ident)
    }

    fn line(&self, c: usize) -> usize {
        self.token(c).map_or(0, |t| t.line)
    }

    fn end_line(&self, c: usize) -> usize {
        self.token(c).map_or(0, |t| t.end_line)
    }

    /// Two code tokens with nothing between them, like the `=` and `>` of `=>`.
    fn adjacent(&self, a: usize, b: usize) -> bool {
        matches!((self.token(a), self.token(b)), (Some(x), Some(y)) if x.end == y.start)
    }

    fn is_doc_string(&self, c: usize) -> bool {
        self.kind(c) == Some(Kind::Comment)
    }

    /// Past the bracket group opening at `c`, or just past `c` if it has no match.
    fn skip(&self, c: usize) -> usize {
        self.matches
            .get(c)
            .copied()
            .flatten()
            .map_or(c + 1, |m| m + 1)
    }

    fn matching(&self) -> Vec<Option<usize>> {
        let mut matches = vec![None; self.code.len()];
        let mut open: Vec<(usize, &str)> = Vec::new();
        for c in 0..self.code.len() {
            if self.kind(c) != Some(Kind::Punct) {
                continue;
            }
            let closes = match self.text(c) {
                t @ ("(" | "[" | "{") => {
                    open.push((c, t));
                    continue;
                }
                ")" => "(",
                "]" => "[",
                "}" => "{",
                _ => continue,
            };
            // Unbalanced code: drop unmatched openers until one matches.
            if let Some(depth) = open.iter().rposition(|&(_, t)| t == closes) {
                let (start, _) = open[depth];
                open.truncate(depth);
                matches[start] = Some(c);
                matches[c] = Some(start);
            }
        }
        matches
    }

    /// Past generic or template parameters `<...>` opening at `c`.
    fn past_angles(&self, c: usize, end: usize) -> usize {
        let mut depth = 0;
        let mut k = c;
        while k < end {
            match self.text(k) {
                "<" => depth += 1,
                // `->` in `Fn() -> T` closes nothing.
                ">" if !(k > 0 && self.text(k - 1) == "-" && self.adjacent(k - 1, k)) => {
                    depth -= 1;
                    if depth == 0 {
                        return k + 1;
                    }
                }
                "(" | "[" => {
                    k = self.skip(k);
                    continue;
                }
                "{" | ";" => return k,
                _ => {}
            }
            k += 1;
        }
        k
    }

    /// `=` alone at `c`: not `==`, `=>`, `<=`, `>=`, `!=`, `+=` and the like.
    fn is_assign(&self, c: usize) -> bool {
        if self.text(c) != "=" {
            return false;
        }
        let before = c > 0
            && self.adjacent(c - 1, c)
            && matches!(
                self.text(c - 1),
                "=" | "<" | ">" | "!" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | ":"
            );
        let after = self.adjacent(c, c + 1) && matches!(self.text(c + 1), "=" | ">");
        !before && !after
    }

    /// `=>` at `c`.
    fn is_arrow(&self, c: usize) -> bool {
        self.text(c) == "=" && self.text(c + 1) == ">" && self.adjacent(c, c + 1)
    }

    // Statements -----------------------------------------------------------------------------

    fn statements(&self, lo: usize, hi: usize) -> Vec<Statement> {
        match self.profile.blocks {
            Blocks::Braces => self.brace_statements(lo, hi),
            Blocks::Indent => self.indent_statements(lo, hi),
        }
    }

    /// Whether a statement can end before `next`, on a new line, without a `;`: JavaScript's
    /// automatic semicolons, and decorators on lines of their own.
    fn ends_at_newline(&self, previous: usize, next: usize) -> bool {
        if self.line(next) <= self.end_line(previous) {
            return false;
        }
        let ends = match self.kind(previous) {
            Some(Kind::Ident | Kind::Number | Kind::String) => true,
            Some(Kind::Punct) => matches!(self.text(previous), ")" | "]" | "}"),
            _ => false,
        };
        let starts = match self.kind(next) {
            Some(Kind::Ident | Kind::String | Kind::Number) => true,
            Some(Kind::Punct) => matches!(self.text(next), "@" | "#"),
            _ => false,
        };
        ends && starts
    }

    fn brace_statements(&self, lo: usize, hi: usize) -> Vec<Statement> {
        let semicolon_free = self.profile.bindings.iter().any(|&(w, _)| w == "let");
        let mut out = Vec::new();
        let mut i = lo;
        while i < hi {
            let first = i;
            let mut blocks = Vec::new();
            let mut has_assign = false;
            let mut saw_group = false;
            // Open `<` before any assignment: type arguments, whose `=` is not one.
            let mut angles = 0usize;
            let mut last = hi - 1;
            let mut k = i;
            while k < hi {
                if k > first
                    && blocks.is_empty()
                    && semicolon_free
                    && !has_assign
                    && self.ends_at_newline(k - 1, k)
                {
                    last = k - 1;
                    break;
                }
                if k > first && has_assign && semicolon_free && self.ends_at_newline(k - 1, k) {
                    last = k - 1;
                    break;
                }
                match self.text(k) {
                    ";" if self.kind(k) == Some(Kind::Punct) => {
                        last = k;
                        break;
                    }
                    "(" | "[" => {
                        saw_group |= self.text(k) == "(";
                        k = self.skip(k);
                        continue;
                    }
                    "=" if angles == 0 && self.is_assign(k) => has_assign = true,
                    "<" if !has_assign
                        && !(self.adjacent(k, k + 1) && matches!(self.text(k + 1), "=" | "<")) =>
                    {
                        angles += 1;
                    }
                    // Not the `>` of `->` or `=>`.
                    ">" if !has_assign
                        && !(k > first
                            && self.adjacent(k - 1, k)
                            && matches!(self.text(k - 1), "-" | "=")) =>
                    {
                        angles = angles.saturating_sub(1);
                    }
                    // Type parameters after a definition's name, `interface A<T = B>`: their
                    // `=` is a default, not an assignment.
                    "<" if k >= first + 2
                        && self.is_ident(k - 1)
                        && self.definition(k - 2).is_some() =>
                    {
                        k = self.past_angles(k, hi);
                        continue;
                    }
                    "{" if self.import_braces(first) => {
                        k = self.skip(k);
                        continue;
                    }
                    "{" => {
                        let Some(close) = self.matches[k] else {
                            // Never closed: the block goes on to the end.
                            blocks.push((k + 1, hi));
                            last = hi - 1;
                            break;
                        };
                        // A value, `const x = { ... };`, or a type, `f(): A & { b: B } { ... }`.
                        let typed = k > first
                            && matches!(self.text(k - 1), ":" | "&" | "|" | "(" | "<" | "is");
                        if has_assign || typed {
                            k = close + 1;
                            continue;
                        }
                        blocks.push((k + 1, close));
                        let next = close + 1;
                        let continues = matches!(self.text(next), "else" | "catch" | "finally")
                            || (self.text(next) == "while" && self.text(first) == "do");
                        if continues {
                            k = next;
                            continue;
                        }
                        let record = self.profile.declarators
                            && !saw_group
                            && (first..k).any(|d| {
                                matches!(
                                    self.text(d),
                                    "struct" | "union" | "enum" | "class" | "typedef"
                                )
                            });
                        if record {
                            // `struct x { ... } a, b;`: on to the `;`.
                            k = next;
                            while k < hi && self.text(k) != ";" {
                                k = match self.text(k) {
                                    "(" | "[" | "{" => self.skip(k),
                                    _ => k + 1,
                                };
                            }
                            last = k.min(hi - 1);
                            break;
                        }
                        last = if self.text(next) == ";" && next < hi {
                            next
                        } else {
                            close
                        };
                        break;
                    }
                    _ => {}
                }
                k += 1;
            }
            let mut start = first;
            // Inner attributes, `#![...]`, are the file's or module's, not the next item's.
            while self.text(start) == "#"
                && self.text(start + 1) == "!"
                && self.text(start + 2) == "["
            {
                start = self.skip(start + 2);
            }
            if start <= last {
                out.push(Statement {
                    first: start,
                    last: last.max(first),
                    blocks,
                });
            }
            i = last.max(first) + 1;
        }
        self.merge_decorators(out)
    }

    /// Whether braces in the statement from `first` are a list of names, not a block:
    /// `import { a } from ...`, `export { a } from ...`, `use a::{b, c};`.
    fn import_braces(&self, first: usize) -> bool {
        let word = self.text(first);
        (self.profile.imports.contains(&word) && !self.used_as_name(first, first + 2))
            || (word == "export" && matches!(self.text(first + 1), "{" | "*" | "type"))
    }

    /// Decorators and attributes on statements of their own belong to the next statement.
    fn merge_decorators(&self, statements: Vec<Statement>) -> Vec<Statement> {
        let mut out: Vec<Statement> = Vec::new();
        let mut pending: Option<usize> = None;
        for mut statement in statements {
            if let Some(first) = pending.take() {
                statement.first = first;
            }
            if self.only_prefixes(&statement) {
                pending = Some(statement.first);
                continue;
            }
            out.push(statement);
        }
        out
    }

    /// Whether a statement is nothing but decorators or attributes.
    fn only_prefixes(&self, statement: &Statement) -> bool {
        let k = self.past_decorations(statement.first, statement.last + 1);
        k > statement.first && k > statement.last
    }

    /// Python's logical lines: physical lines joined inside brackets and after `\`.
    fn logical_lines(&self) -> Vec<(usize, usize, usize)> {
        let mut lines: Vec<(usize, usize, usize)> = Vec::new();
        let mut depth = 0usize;
        for c in 0..self.code.len() {
            let token = self.tokens[self.code[c]];
            let continues = c > 0 && {
                let previous = self.tokens[self.code[c - 1]];
                // A backslash continues a line only right at its end, not in a comment, and
                // only onto the next line.
                let rest = &self.source[previous.end..token.start];
                let rest_of_line = rest.split('\n').next().unwrap_or("");
                depth > 0
                    || previous.end_line == token.line
                    || (rest_of_line.trim_end().ends_with('\\')
                        && token.line == previous.end_line + 1)
            };
            if continues {
                if let Some(last) = lines.last_mut() {
                    last.1 = c;
                }
            } else {
                let line_start = self.source[..token.start].rfind('\n').map_or(0, |n| n + 1);
                lines.push((c, c, token.start - line_start));
            }
            if token.kind == Kind::Punct {
                match token.text(self.source) {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
        }
        lines
    }

    fn indent_statements(&self, lo: usize, hi: usize) -> Vec<Statement> {
        let lines: Vec<(usize, usize, usize)> = self
            .lines
            .iter()
            .copied()
            .filter(|&(first, _, _)| first >= lo && first < hi)
            .collect();
        let mut out = Vec::new();
        let mut n = 0;
        while n < lines.len() {
            let (first, last, indent) = lines[n];
            let mut statement = Statement {
                first,
                last,
                blocks: Vec::new(),
            };
            n += 1;
            // A one-line compound statement, `def f(): return 1`: its block is after the `:`.
            let core = self.past_prefixes(first, last + 1);
            let compound =
                self.definition(core).is_some() || self.profile.control.contains(&self.text(core));
            if compound && self.text(last) != ":" {
                let mut k = core;
                while k <= last {
                    if self.text(k) == ":"
                        && k > core
                        && (self.text(k - 1) == ")"
                            || self.is_ident(k - 1)
                            || self.text(k - 1) == "]")
                    {
                        statement.blocks.push((k + 1, last + 1));
                        break;
                    }
                    k = match self.text(k) {
                        "(" | "[" | "{" => self.skip(k),
                        _ => k + 1,
                    };
                }
            }
            if self.text(last) == ":" {
                let body_start = n;
                while n < lines.len() && lines[n].2 > indent {
                    n += 1;
                }
                if n > body_start {
                    let end = lines[n - 1].1;
                    statement.blocks.push((lines[body_start].0, end + 1));
                    statement.last = end;
                }
            }
            out.push(statement);
        }
        self.merge_decorators(out)
    }

    // Prefixes ---------------------------------------------------------------------------------

    /// Whether the word at `c` is used as a name here rather than as a keyword: followed by `.`,
    /// `=`, `(`, `:`, `,`, `;`, `)`, `?` or the end.
    fn used_as_name(&self, c: usize, end: usize) -> bool {
        if matches!(self.text(c), "public" | "private" | "protected")
            && self.text(c + 1) == ":"
            && self.text(c + 2) != ":"
            && self.profile.declarators
        {
            // A C++ access label.
            return false;
        }
        c + 1 >= end
            || (matches!(
                self.text(c + 1),
                "." | "=" | "(" | ":" | "," | ";" | ")" | "?" | "!"
            ) && !(self.text(c + 1) == ":" && self.text(c + 2) == ":")
                && !(self.text(c + 1) == "!" && self.text(c) == "macro_rules")
                && !(self.text(c + 1) == "(" && self.text(c) == "pub"))
    }

    /// Past decorators and attributes at `c`.
    fn past_decorations(&self, mut c: usize, end: usize) -> usize {
        loop {
            if self.profile.decorators && self.text(c) == "@" && c + 1 < end {
                // `@name`, `@a.b(...)`, `@a.b().c(...)[0]`: names, dots, calls and indexes.
                c += 2;
                loop {
                    match self.text(c) {
                        "." if self.is_ident(c + 1) => c += 2,
                        "(" | "[" => c = self.skip(c),
                        _ => break,
                    }
                }
                continue;
            }
            let attribute = self.profile.attributes.iter().find_map(|(open, _)| {
                let parts: Vec<char> = open.chars().collect();
                let matched = parts
                    .iter()
                    .enumerate()
                    .all(|(n, ch)| self.text(c + n) == ch.to_string());
                matched.then_some(parts.len())
            });
            if let Some(length) = attribute {
                // The group is the last `[` of the opener.
                c = self.skip(c + length - 1);
                if self.text(c) == "]" {
                    c += 1;
                }
                continue;
            }
            return c;
        }
    }

    /// Past decorators, attributes, modifiers and templates: where the core of a statement
    /// starts.
    fn past_prefixes(&self, mut c: usize, end: usize) -> usize {
        loop {
            let after = self.past_decorations(c, end);
            if after > c {
                c = after;
                continue;
            }
            let word = self.text(c);
            // `extern "C" { ... }` is a block of its own, read through.
            if word == "extern" && self.kind(c + 1) == Some(Kind::String) && self.text(c + 2) == "{"
            {
                return c;
            }
            if self.kind(c) == Some(Kind::Ident)
                && self.profile.modifiers.contains(&word)
                && !self.used_as_name(c, end)
                && !(self.text(c + 1) == "<" && !self.profile.declarators)
            {
                c += 1;
                match self.text(c) {
                    "(" if word == "pub" => c = self.skip(c),
                    ":" if matches!(word, "public" | "private" | "protected") => c += 1,
                    _ => {}
                }
                if word == "extern" && self.kind(c) == Some(Kind::String) && self.text(c + 1) != "{"
                {
                    c += 1;
                }
                continue;
            }
            if word == "template" && self.text(c + 1) == "<" {
                c = self.past_angles(c + 1, end);
                continue;
            }
            // A definition word before another one is a modifier: `const fn`, `typedef struct`.
            if self.definition(c).is_some()
                && !self.used_as_name(c, end)
                && !self.used_as_name(c + 1, end)
                && (self.definition(c + 1).is_some()
                    || self.profile.modifiers.contains(&self.text(c + 1)))
                && !(self.text(c) == "enum" && self.text(c + 1) == "class")
            {
                c += 1;
                continue;
            }
            return c;
        }
    }

    fn definition(&self, c: usize) -> Option<&'static str> {
        let word = self.text(c);
        if self.kind(c) != Some(Kind::Ident) {
            return None;
        }
        self.profile
            .definitions
            .iter()
            .find(|&&(w, _)| w == word)
            .map(|&(_, kind)| kind)
    }

    fn binding(&self, c: usize) -> Option<&'static str> {
        let word = self.text(c);
        if self.kind(c) != Some(Kind::Ident) {
            return None;
        }
        self.profile
            .bindings
            .iter()
            .find(|&&(w, _)| w == word)
            .map(|&(_, kind)| kind)
    }

    // Rules ----------------------------------------------------------------------------------

    fn read(&self, lo: usize, hi: usize, scope: Scope) -> Vec<Item> {
        let mut items = Vec::new();
        for statement in self.statements(lo, hi) {
            items.extend(self.classify(&statement, scope));
        }
        items
    }

    /// The items a statement makes: usually none or one; a control statement's blocks can hold
    /// several.
    fn classify(&self, statement: &Statement, scope: Scope) -> Vec<Item> {
        let end = statement.last + 1;
        let first = statement.first;
        if self.is_doc_string(first) && first == statement.last {
            return Vec::new();
        }
        let c = self.past_prefixes(first, end);
        let word = self.text(c);
        let is_word = self.kind(c) == Some(Kind::Ident);

        // 1. Control statements and bare blocks are read through.
        // Control words come before `(` and `:` (`if (x)`, `try:`): they are names only before
        // `.`, `=`, `,` or `)`.
        let control = is_word
            && self.profile.control.contains(&word)
            && !(matches!(self.text(c + 1), "." | "," | ")") || self.is_assign(c + 1))
            && !(word == "unsafe" && self.text(c + 1) != "{");
        let extern_block = word == "extern" && self.kind(c + 1) == Some(Kind::String);
        if control || extern_block || (word == "{" && !statement.blocks.is_empty()) {
            return statement
                .blocks
                .iter()
                .flat_map(|&(lo, hi)| self.read(lo, hi, scope))
                .collect();
        }

        // 2. Imports.
        if is_word && self.profile.imports.contains(&word) && !self.used_as_name(c, end) {
            if word == "using" && self.text(c + 2) == "=" {
                return self.item(
                    "type",
                    self.text(c + 1).to_owned(),
                    statement,
                    c,
                    Vec::new(),
                );
            }
            if scope == Scope::Function {
                return Vec::new();
            }
            let name = self.import_name(c, end);
            return self.item("import", name, statement, c, Vec::new());
        }
        if self.profile.strings.templates
            && matches!(word, "*" | "{")
            && (c..end).any(|k| self.text(k) == "from")
            && self.text(first) == "export"
        {
            return self.item("import", self.import_name(c, end), statement, c, Vec::new());
        }

        // 3. Definitions.
        if let Some(kind) = self.definition(c).filter(|_| !self.used_as_name(c, end))
            && let Some(items) = self.definition_items(kind, statement, c)
        {
            return items;
        }
        if scope == Scope::Function {
            // Not an item, but blocks in it can hold definitions: callbacks, closures.
            if self.profile.blocks == Blocks::Braces {
                return self.nested_definitions(first, end);
            }
            return Vec::new();
        }

        // 4. Bindings.
        if let Some(kind) = self.binding(c).filter(|_| !self.used_as_name(c, end)) {
            let name_at = c + 1;
            let name = self.pattern_name(name_at, end);
            if let Some(members) = self.class_value(name_at, end) {
                return self.item("class", name, statement, c, members);
            }
            let kind = if self.function_value(name_at, end) {
                "function"
            } else if scope == Scope::Members {
                "field"
            } else {
                kind
            };
            return self.item(kind, name, statement, c, Vec::new());
        }
        if self.profile.assignments
            && let Some(name) = self.assignment(c, end)
        {
            let kind = if self.function_value(c, end) {
                "function"
            } else if scope == Scope::Members {
                "field"
            } else {
                "variable"
            };
            return self.item(kind, name, statement, c, Vec::new());
        }

        // 6, early: a statement that is only a call. In a class, `name(...);` declares a method;
        // only a macro, `name!(...)`, is a call there.
        if let Some(name) = self.call(c, end)
            && (scope == Scope::Top || name.ends_with('!'))
        {
            return self.item("call", name, statement, c, Vec::new());
        }

        // 5. C-style declarations.
        if self.profile.declarators {
            return self.declaration(statement, c, scope);
        }

        // Members of classes and interfaces: methods and fields.
        if scope == Scope::Members && self.is_ident(c) {
            // `name(...)`, `name<T>(...)`, `name?(...)`: a method. `new (...)`, a construct
            // signature, isn't.
            let after = if self.text(c + 1) == "?" {
                c + 2
            } else {
                c + 1
            };
            let generic = self.text(after) == "<" && self.text(self.past_angles(after, end)) == "(";
            if self.text(c) == "new" && matches!(self.text(c + 1), "(" | "<") {
                return Vec::new();
            }
            if self.text(after) == "(" || generic {
                let kind = if statement.blocks.is_empty() {
                    "prototype"
                } else {
                    "function"
                };
                let children = self.function_children(statement);
                return self.item(kind, self.text(c).to_owned(), statement, c, children);
            }
            if self.function_value(c, end) {
                return self.item(
                    "function",
                    self.text(c).to_owned(),
                    statement,
                    c,
                    Vec::new(),
                );
            }
            return self.item("field", self.text(c).to_owned(), statement, c, Vec::new());
        }
        Vec::new()
    }

    fn definition_items(
        &self,
        kind: &'static str,
        statement: &Statement,
        c: usize,
    ) -> Option<Vec<Item>> {
        let end = statement.last + 1;
        // `type` is a definition only as `type Name = ...` or `type Name<...> = ...`.
        if self.text(c) == "type"
            && !(self.is_ident(c + 1) && matches!(self.text(c + 2), "=" | "<" | "["))
            && self.profile.definitions.iter().all(|&(w, _)| w != "fn")
        {
            return None;
        }
        // C and C++: `struct x` is a type in a declaration unless a body follows.
        if self.profile.declarators && matches!(kind, "struct" | "union" | "enum" | "class") {
            let body = statement.blocks.first().map(|&(open, _)| open - 1);
            let defines = body.is_some_and(|b| {
                (c + 1..b).all(|k| {
                    self.is_ident(k)
                        || matches!(self.text(k), ":" | "," | "<" | ">" | "::")
                        || self.kind(k) == Some(Kind::Punct) && self.text(k) == ":"
                })
            });
            if !defines {
                return None;
            }
        }
        if kind == "type" && self.text(c) == "typedef" {
            return Some(self.item("type", self.typedef_name(c, end), statement, c, Vec::new()));
        }
        let name = self.definition_name(kind, c, statement);
        let body = statement.blocks.first().copied();
        let kind = if kind == "function" && body.is_none() {
            "prototype"
        } else {
            kind
        };
        let children = match (kind, body) {
            (_, None) => Vec::new(),
            ("function", Some(_)) => self.function_children(statement),
            (k, Some((lo, hi))) if RECORDS.contains(&k) && !self.profile.declarators => {
                self.record_members(k, lo, hi)
            }
            ("enum", Some((lo, hi))) => self.record_members("enum", lo, hi),
            ("module", Some((lo, hi))) => self.read(lo, hi, Scope::Top),
            ("macro" | "type" | "const" | "static", _) => Vec::new(),
            (_, Some((lo, hi))) => {
                let mut members = self.read(lo, hi, Scope::Members);
                // `Name(...);` in class `Name` is its constructor, not a call.
                for member in &mut members {
                    if member.kind == "call" && member.name == name {
                        member.kind = "prototype";
                    }
                }
                members
            }
        };
        let mut items = self.item(kind, name, statement, c, children);
        if self.profile.declarators && matches!(kind, "struct" | "union" | "enum" | "class") {
            // `typedef struct {...} name;`, `struct {...} x;`: named by the declarator.
            if let (Some(item), Some(&(_, close))) = (items.first_mut(), statement.blocks.first()) {
                let typedef = (statement.first..c).any(|d| self.text(d) == "typedef");
                let declarator = (close + 1..end).rev().find(|&d| self.is_ident(d));
                if let Some(d) = declarator
                    && (typedef || item.name.is_empty())
                {
                    item.name = self.text(d).to_owned();
                }
            }
        }
        Some(items)
    }

    /// Definitions in blocks anywhere in code positions `lo..hi`, like the body of a callback:
    /// `foo(function () { function step() {} })`.
    fn nested_definitions(&self, lo: usize, hi: usize) -> Vec<Item> {
        let mut items = Vec::new();
        let mut k = lo;
        while k < hi {
            if self.text(k) == "{"
                && self.kind(k) == Some(Kind::Punct)
                && let Some(close) = self.matches[k]
                && close < hi
            {
                items.extend(self.read(k + 1, close, Scope::Function));
                k = close + 1;
                continue;
            }
            k += 1;
        }
        items
    }

    /// The definitions inside a function's body, through control statements.
    fn function_children(&self, statement: &Statement) -> Vec<Item> {
        statement
            .blocks
            .first()
            .map_or_else(Vec::new, |&(lo, hi)| self.read(lo, hi, Scope::Function))
    }

    /// Fields of a struct or union, variants of an enum: split by `,` and `;`.
    fn record_members(&self, kind: &str, lo: usize, hi: usize) -> Vec<Item> {
        let member = if kind == "enum" { "variant" } else { "field" };
        let mut items = Vec::new();
        let mut start = lo;
        let mut k = lo;
        while k <= hi {
            if k == hi || matches!(self.text(k), "," | ";") {
                let c = self.past_prefixes(start, k);
                if c < k && self.is_ident(c) {
                    let name = if self.profile.declarators && member == "field" {
                        self.declarator_name(c, k)
                    } else {
                        self.text(c).to_owned()
                    };
                    let statement = Statement {
                        first: start,
                        last: k.saturating_sub(1).max(start),
                        blocks: Vec::new(),
                    };
                    items.extend(self.item(member, name, &statement, c, Vec::new()));
                }
                start = k + 1;
                k += 1;
                continue;
            }
            k = match self.text(k) {
                "(" | "[" | "{" => self.skip(k),
                "<" => self.past_angles(k, hi),
                _ => k + 1,
            };
        }
        items
    }

    // Names ----------------------------------------------------------------------------------

    fn definition_name(&self, kind: &str, c: usize, statement: &Statement) -> String {
        let end = statement
            .blocks
            .first()
            .map_or(statement.last + 1, |&(open, _)| open - 1);
        let mut k = c + 1;
        match kind {
            "impl" => {
                if self.text(k) == "<" {
                    k = self.past_angles(k, end);
                }
                let last = (k..end).find(|&d| self.text(d) == "where").unwrap_or(end);
                return join(self.source, &self.slice(k.min(last), last));
            }
            "macro" if self.text(k) == "!" => k += 1,
            "static" if self.text(k) == "mut" => k += 1,
            "function" if self.text(k) == "*" => k += 1,
            "enum" if matches!(self.text(k), "class" | "struct") => k += 1,
            _ => {}
        }
        if self.kind(k) == Some(Kind::String) {
            return self.text(k).trim_matches(['"', '\'']).to_owned();
        }
        if !self.is_ident(k) {
            return String::new();
        }
        // Dotted and qualified names: `namespace a.b`, `namespace a::b`.
        let mut last = k;
        while matches!(self.text(last + 1), "." | ":")
            && self.is_ident(last + 2 + usize::from(self.text(last + 1) == ":"))
            && (self.text(last + 1) == "." || self.text(last + 2) == ":")
        {
            last += if self.text(last + 1) == "." { 2 } else { 3 };
        }
        join(self.source, &self.slice(k, last + 1))
    }

    fn slice(&self, from: usize, to: usize) -> Vec<Token> {
        (from..to.min(self.code.len()))
            .map(|c| self.tokens[self.code[c]])
            .collect()
    }

    /// The name of `typedef ... name;`, or of `typedef int (*name)(int);`.
    fn typedef_name(&self, c: usize, end: usize) -> String {
        let mut k = c + 1;
        while k < end {
            if self.text(k) == "(" && self.text(k + 1) == "*" && self.is_ident(k + 2) {
                return self.text(k + 2).to_owned();
            }
            k = match self.text(k) {
                "(" | "[" | "{" => self.skip(k),
                _ => k + 1,
            };
        }
        self.declarator_name(c + 1, end)
    }

    /// The name a declaration declares: the last identifier before `=`, `[`, `:`, `(` or the
    /// end, as in `const char *names[2] = ...`.
    fn declarator_name(&self, c: usize, end: usize) -> String {
        let mut name = None;
        let mut k = c;
        while k < end {
            match self.text(k) {
                "=" | "[" | "(" | ";" | "," => break,
                ":" if self.text(k + 1) != ":" && (k == 0 || self.text(k - 1) != ":") => break,
                "<" => {
                    k = self.past_angles(k, end);
                    continue;
                }
                _ => {}
            }
            if self.is_ident(k) && !self.profile.not_names.contains(&self.text(k)) {
                name = Some(k);
            }
            k += 1;
        }
        name.map_or_else(String::new, |n| self.text(n).to_owned())
    }

    /// The name of a binding: an identifier, or a destructuring pattern.
    fn pattern_name(&self, c: usize, end: usize) -> String {
        match self.text(c) {
            "{" | "[" => {
                let close = self.skip(c);
                join(self.source, &self.slice(c, close.min(end)))
            }
            _ if self.is_ident(c) => self.text(c).to_owned(),
            _ => String::new(),
        }
    }

    /// Python's `name = ...`, `a.b = ...`, `x: int = ...`, `a, b = ...`: the name, if the
    /// statement binds one.
    fn assignment(&self, c: usize, end: usize) -> Option<String> {
        if !self.is_ident(c) {
            return None;
        }
        let mut k = c;
        while k < end {
            match self.text(k) {
                "=" if self.is_assign(k) => break,
                ":" => break,
                "(" | "[" | "{" => return None,
                _ => {}
            }
            k += 1;
        }
        if k >= end || k == c {
            return None;
        }
        if self.text(k) == ":" && !self.is_ident(k + 1) && self.text(k + 1) != "\"" {
            // A block opener, `else:`, is not an annotation.
            return None;
        }
        Some(join(self.source, &self.slice(c, k)))
    }

    /// The members of the value after the first `=` from `c`, if it is a class:
    /// `var A = class extends B { ... }`.
    fn class_value(&self, c: usize, end: usize) -> Option<Vec<Item>> {
        let assign = (c..end).find(|&k| self.is_assign(k))?;
        if self.definition(assign + 1) != Some("class") {
            return None;
        }
        let open = (assign + 1..end).find(|&k| self.text(k) == "{")?;
        let close = self.matches[open]?;
        Some(self.read(open + 1, close, Scope::Members))
    }

    /// Whether the value after the first `=` from `c` is a function: `=>`, `lambda`,
    /// `function`.
    fn function_value(&self, c: usize, end: usize) -> bool {
        let Some(assign) = (c..end).find(|&k| self.is_assign(k)) else {
            return false;
        };
        let mut k = assign + 1;
        if self.text(k) == "async" {
            k += 1;
        }
        if self.profile.function_values.contains(&self.text(k)) {
            return true;
        }
        // An arrow function: `(a, b) => ...`, `a => ...`.
        let params_end = match self.text(k) {
            "(" => self.skip(k),
            _ if self.is_ident(k) => k + 1,
            _ => return false,
        };
        let mut p = params_end;
        if self.text(p) == ":" {
            // A return type: on to the arrow.
            while p < end && !self.is_arrow(p) {
                p = match self.text(p) {
                    "(" | "[" | "{" => self.skip(p),
                    _ => p + 1,
                };
            }
        }
        self.is_arrow(p)
    }

    /// The name of what an import brings in: the module's path or string.
    fn import_name(&self, c: usize, end: usize) -> String {
        let last = if self.text(end - 1) == ";" {
            end - 1
        } else {
            end
        };
        if let Some(from) = (c..last).find(|&k| self.text(k) == "from")
            && self.kind(from + 1) == Some(Kind::String)
        {
            return self.text(from + 1).trim_matches(['"', '\'']).to_owned();
        }
        if self.kind(c + 1) == Some(Kind::String) {
            return self.text(c + 1).trim_matches(['"', '\'']).to_owned();
        }
        // Python's `from a.b import c`: the module.
        if self.text(c) == "from"
            && let Some(import) = (c..last).find(|&k| self.text(k) == "import")
        {
            return join(self.source, &self.slice(c + 1, import));
        }
        join(self.source, &self.slice(c + 1, last))
    }

    /// The callee, if the statement from `c` is only a call: `path(...)`, `path!(...)`,
    /// `path!{...}`, maybe with a `;`.
    fn call(&self, c: usize, end: usize) -> Option<String> {
        if !self.is_ident(c) {
            return None;
        }
        let mut k = c;
        while matches!(self.text(k + 1), "." | ":")
            && self.is_ident(k + 2 + usize::from(self.text(k + 1) == ":"))
        {
            if self.text(k + 1) == ":" && self.text(k + 2) != ":" {
                break;
            }
            k += if self.text(k + 1) == "." { 2 } else { 3 };
        }
        let mut open = k + 1;
        let bang = self.profile.macro_bang && self.text(open) == "!";
        if bang {
            open += 1;
        }
        if !matches!(self.text(open), "(" | "{" | "[") || (!bang && self.text(open) != "(") {
            return None;
        }
        let after = self.skip(open);
        let rest = if self.text(after) == ";" {
            after + 1
        } else {
            after
        };
        if rest != end {
            return None;
        }
        let mut name = join(self.source, &self.slice(c, k + 1));
        if bang {
            name.push('!');
        }
        Some(name)
    }

    /// C and C++ declarations without a definition word: functions, prototypes, variables.
    fn declaration(&self, statement: &Statement, c: usize, scope: Scope) -> Vec<Item> {
        let end = statement.last + 1;
        if !self.is_ident(c) && !matches!(self.text(c), "~" | "::") {
            return Vec::new();
        }
        let body = statement.blocks.first().copied();
        // No type before the name: `Name(...)` is a constructor or a macro call, `~Name()` a
        // destructor.
        let bare = self.is_ident(c) && self.text(c + 1) == "(";
        if bare && body.is_none() && scope == Scope::Members {
            return self.item("call", self.text(c).to_owned(), statement, c, Vec::new());
        }
        // A declaration names a type, then what it declares: two words at least.
        if !bare && self.text(c) != "~" && (c..end).filter(|&k| self.is_ident(k)).count() < 2 {
            return Vec::new();
        }
        let head_end = body.map_or(end, |(open, _)| open - 1);
        let initializer = (c..head_end).find(|&k| self.is_assign(k));
        let name_end = initializer.unwrap_or(head_end);
        let function = self.function_name(c, name_end);
        match (body, function) {
            (Some(_), Some(name)) if initializer.is_none() => {
                let children = self.function_children(statement);
                self.item("function", name, statement, c, children)
            }
            (Some(_), _) => Vec::new(),
            (None, Some(name)) if initializer.is_none() && scope != Scope::Function => {
                self.item("prototype", name, statement, c, Vec::new())
            }
            (None, _) if scope != Scope::Function => {
                let name = self.declarator_name(c, end);
                if name.is_empty() || name_end <= c + 1 && initializer.is_none() {
                    return Vec::new();
                }
                let kind = if scope == Scope::Members {
                    "field"
                } else {
                    "variable"
                };
                self.item(kind, name, statement, c, Vec::new())
            }
            _ => Vec::new(),
        }
    }

    /// The function name in the declaration `c..end`: the identifier before its parameter list,
    /// qualified (`Foo::bar`), with `~` and `operator==`. A macro around the return type, like
    /// `GIT_INLINE(int) name(...)`, or after the parameters, like `name(...) __THROW(...)` and
    /// `name(...) __nonnull ((1))`, is skipped; a macro around the declarator, like
    /// `__NTH (memcpy (...))`, is looked into.
    fn function_name(&self, c: usize, end: usize) -> Option<String> {
        let mut candidates: Vec<(usize, usize)> = Vec::new();
        let mut k = c;
        let mut seen_group = false;
        while k < end {
            let text = self.text(k);
            if text == ":" && seen_group && self.text(k + 1) != ":" && self.text(k - 1) != ":" {
                // A constructor's initializer list.
                break;
            }
            if text == "(" {
                if self.text(k + 1) == "*"
                    && let Some(close) = self.matches[k]
                    && let Some(name) = self.function_name(k + 2, close)
                {
                    return Some(name);
                }
                // Named by the identifier before it, or an operator: `operator==(`.
                // Not an attribute, `__nonnull ((1, 2))`: those take their list in two parentheses.
                let named = k > c
                    && self.is_ident(k - 1)
                    && !self.profile.not_names.contains(&self.text(k - 1))
                    && !(self.text(k + 1) == "("
                        && self.matches[k + 1].map(|m| m + 1) == self.matches[k]);
                if named || (k > c && self.operator_at(k).is_some()) {
                    candidates.push((k - 1, k));
                }
                seen_group = true;
                k = self.skip(k);
                continue;
            }
            k = match text {
                "[" => self.skip(k),
                "<" if k > c && self.is_ident(k - 1) => self.past_angles(k, end),
                _ => k + 1,
            };
        }
        // The last one, but a macro's name, `__THROW (...)` after the parameters, only when
        // nothing else names it.
        let macro_name = |at: usize| {
            self.text(at)
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        };
        let &(name_at, group) = candidates
            .iter()
            .rev()
            .find(|&&(at, _)| !macro_name(at))
            .or(candidates.last())?;
        // `__NTH (memcpy (...))`: the name is inside.
        if self.is_ident(group + 1) && self.text(group + 2) == "(" {
            let inner_close = self.skip(group + 2);
            if self.matches[group] == Some(inner_close) {
                return Some(self.text(group + 1).to_owned());
            }
        }
        if let Some(start) = self.operator_at(group) {
            return Some(join(self.source, &self.slice(start, group)));
        }
        // Qualified names and destructors.
        let mut start = name_at;
        if start > c && self.text(start - 1) == "~" {
            start -= 1;
        }
        while start >= c + 3 && self.text(start - 1) == ":" && self.text(start - 2) == ":" {
            let mut owner = start - 3;
            if self.text(owner) == ">" {
                // `Buffer<int>::count`: back over the template arguments.
                let mut depth = 0;
                let mut k = owner;
                loop {
                    match self.text(k) {
                        ">" => depth += 1,
                        "<" => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 || k <= c {
                        break;
                    }
                    k -= 1;
                }
                owner = k.saturating_sub(1);
            }
            if owner < c || !self.is_ident(owner) {
                break;
            }
            start = owner;
        }
        if !self.is_ident(name_at) {
            return None;
        }
        Some(join(self.source, &self.slice(start, name_at + 1)))
    }

    /// Where `operator` starts, if the parameter list at `group` belongs to an operator.
    fn operator_at(&self, group: usize) -> Option<usize> {
        (group.saturating_sub(4)..group).find(|&k| self.text(k) == "operator")
    }

    // Items ----------------------------------------------------------------------------------

    /// An item for `statement`, its core at `c`: its lines, with doc comments before it, and the
    /// names its own code calls.
    fn item(
        &self,
        kind: &'static str,
        name: String,
        statement: &Statement,
        c: usize,
        children: Vec<Item>,
    ) -> Vec<Item> {
        let mut children = children;
        if children.is_empty()
            && statement.blocks.is_empty()
            && self.profile.blocks == Blocks::Braces
            && matches!(
                kind,
                "call" | "const" | "variable" | "field" | "function" | "static"
            )
        {
            // `const f = () => { function inner() {} }`, `describe(..., () => { ... })`.
            children = self.nested_definitions(c, statement.last + 1);
        }
        let start = self.doc_start(statement.first);
        let end = self.end_line(statement.last);
        let header = if kind == "call" {
            (c..=statement.last)
                .find(|&k| matches!(self.text(k), "(" | "!" | "{"))
                .unwrap_or(c)
        } else {
            statement.blocks.first().map_or(c, |&(open, _)| open)
        };
        let calls = self.calls(header, statement.last + 1, &children);
        vec![Item {
            kind,
            name,
            start,
            end,
            children,
            calls,
        }]
    }

    /// The first line of the item whose first code token is `c`: earlier if doc comments are
    /// right before it.
    fn doc_start(&self, c: usize) -> usize {
        let Some(&index) = self.code.get(c) else {
            return 0;
        };
        let mut line = self.tokens[index].line;
        let mut i = index;
        while i > 0 {
            let previous = self.tokens[i - 1];
            let doc = previous.kind == Kind::Comment
                && self
                    .profile
                    .doc_comments
                    .iter()
                    .any(|p| previous.text(self.source).starts_with(p))
                && !previous.text(self.source).starts_with("////")
                && previous.text(self.source) != "/**/";
            if !doc {
                break;
            }
            line = previous.line;
            i -= 1;
        }
        line
    }

    /// The names called from code positions `lo..hi`, outside `children`: an identifier, or a
    /// path `a.b`, `a::b`, `a->b`, before `(`, or a Rust macro `name!`. Sorted, once each.
    fn calls(&self, lo: usize, hi: usize, children: &[Item]) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for k in lo..hi.min(self.code.len()) {
            if !self.is_ident(k) {
                continue;
            }
            let line = self.line(k);
            if children
                .iter()
                .any(|child| child.start <= line && line <= child.end)
            {
                continue;
            }
            let bang = self.profile.macro_bang
                && self.text(k + 1) == "!"
                && matches!(self.text(k + 2), "(" | "[" | "{");
            if self.text(k + 1) != "(" && !bang {
                continue;
            }
            let word = self.text(k);
            let keyword = self.profile.control.contains(&word)
                || self.profile.definitions.iter().any(|&(w, _)| w == word)
                || self.profile.modifiers.contains(&word)
                || self.profile.not_names.contains(&word)
                || matches!(
                    word,
                    "return"
                        | "await"
                        | "yield"
                        | "typeof"
                        | "not"
                        | "and"
                        | "or"
                        | "in"
                        | "is"
                        | "elif"
                        | "except"
                        | "lambda"
                        | "print"
                        | "super"
                        | "case"
                );
            if keyword || (k > 0 && self.definition(k - 1).is_some()) {
                continue;
            }
            let mut start = k;
            while start >= 2 {
                let sep = self.text(start - 1);
                // `a.b`; `a::b` and `a->b`, two-character separators.
                let two = (sep == ":" && self.text(start - 2) == ":")
                    || (sep == ">" && self.text(start - 2) == "-");
                if sep == "." && self.is_ident(start - 2) {
                    start -= 2;
                } else if two && start >= 3 && self.is_ident(start - 3) {
                    start -= 3;
                } else {
                    break;
                }
            }
            let mut name: String = self
                .slice(start, k + 1)
                .iter()
                .map(|t| t.text(self.source))
                .collect();
            for receiver in ["self.", "this.", "self->", "this->"] {
                if let Some(rest) = name.strip_prefix(receiver) {
                    name = rest.to_owned();
                }
            }
            if bang {
                name.push('!');
            }
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names.sort();
        names
    }
}

/// The tokens the reader reads: not comments (but documentation strings, which keep Python's
/// blocks whole), not directives, and in C and C++ only the first branch of each `#if`.
fn code_tokens(profile: &Profile, source: &str, tokens: &[Token]) -> Vec<usize> {
    let mut code = Vec::new();
    // For each open `#if`: whether it is past its first branch.
    let mut conditions: Vec<bool> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        match token.kind {
            Kind::Directive => {
                let word = directive_word(token.text(source));
                match word {
                    "if" | "ifdef" | "ifndef" => conditions.push(false),
                    "elif" | "else" | "elifdef" | "elifndef" => {
                        if let Some(past) = conditions.last_mut() {
                            *past = true;
                        }
                    }
                    "endif" => {
                        conditions.pop();
                    }
                    _ => {}
                }
            }
            Kind::Comment => {
                // A documentation string starts with its quote, maybe after a prefix: `r"""`.
                let text = token.text(source);
                let unprefixed = text.trim_start_matches(|ch: char| ch.is_ascii_alphabetic());
                let doc_string =
                    text.len() - unprefixed.len() <= 2 && unprefixed.starts_with(['"', '\'']);
                if doc_string && profile.blocks == Blocks::Indent && !conditions.iter().any(|&p| p)
                {
                    code.push(i);
                }
            }
            _ => {
                if !conditions.iter().any(|&past| past) {
                    code.push(i);
                }
            }
        }
    }
    code
}

/// The word of a directive: `define` for `#  define X 1`.
fn directive_word(text: &str) -> &str {
    let rest = text.trim_start_matches('#').trim_start();
    let end = rest
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Items of directives: `#define` makes a macro, `#include` an import.
fn directive_items(source: &str, tokens: &[Token]) -> Vec<Item> {
    let mut items = Vec::new();
    for token in tokens.iter().filter(|t| t.kind == Kind::Directive) {
        let text = token.text(source);
        let word = directive_word(text);
        let rest = text
            .trim_start_matches('#')
            .trim_start()
            .get(word.len()..)
            .unwrap_or("");
        let (kind, name) = match word {
            "define" if rest.starts_with([' ', '\t']) => {
                let name: String = rest
                    .trim_start()
                    .chars()
                    .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                    .collect();
                ("macro", name)
            }
            "include" | "import" | "include_next" => {
                let target = rest.trim();
                let target = target.split("//").next().unwrap_or(target).trim();
                ("import", target.to_owned())
            }
            _ => continue,
        };
        if name.is_empty() {
            continue;
        }
        items.push(Item {
            kind,
            name,
            start: token.line,
            end: token.end_line,
            children: Vec::new(),
            calls: Vec::new(),
        });
    }
    items
}

/// The items of a file, from its tokens.
pub fn items(profile: &'static Profile, source: &str, tokens: &[Token]) -> Vec<Item> {
    let reader = Reader::new(profile, source, tokens);
    let mut items = reader.read(0, reader.code.len(), Scope::Top);
    if profile.directives {
        // Directives not inside an item, in line order with the rest.
        let inside =
            |line: usize, items: &[Item]| items.iter().any(|i| i.start <= line && line <= i.end);
        let extra: Vec<Item> = directive_items(source, tokens)
            .into_iter()
            .filter(|d| !inside(d.start, &items))
            .collect();
        items.extend(extra);
        items.sort_by_key(|i| (i.start, i.end));
    }
    items
}

#[cfg(test)]
mod tests;
