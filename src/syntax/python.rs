//! Python items: `def`, `async def` and `class`, nested by indentation, decorators included.

use super::{Item, Kind, Token};

/// A logical line: its first and last token, and the indentation of the first.
struct Logical {
    first: usize,
    indent: usize,
    line: usize,
    end_line: usize,
}

/// Logical lines: physical lines joined inside brackets and after a backslash. Comments are left
/// out, so lines with only a comment are not lines.
fn logical_lines(source: &str, tokens: &[Token]) -> Vec<Logical> {
    let mut lines: Vec<Logical> = Vec::new();
    let mut depth: usize = 0;
    let mut previous: Option<&Token> = None;
    for (i, token) in tokens.iter().enumerate() {
        if token.kind == Kind::Comment {
            continue;
        }
        let continues = previous.is_some_and(|p| {
            depth > 0 || p.end_line == token.line || source[p.end..token.start].contains('\\')
        });
        if continues {
            if let Some(last) = lines.last_mut() {
                last.end_line = token.end_line;
            }
        } else {
            let line_start = source[..token.start].rfind('\n').map_or(0, |n| n + 1);
            lines.push(Logical {
                first: i,
                indent: token.start - line_start,
                line: token.line,
                end_line: token.end_line,
            });
        }
        if token.kind == Kind::Punct {
            match token.text(source) {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        previous = Some(token);
    }
    lines
}

/// An item still open, with the indentation of its header.
struct Open {
    item: Item,
    indent: usize,
}

fn close(stack: &mut Vec<Open>, top: &mut Vec<Item>) {
    if let Some(open) = stack.pop() {
        match stack.last_mut() {
            Some(parent) => parent.item.children.push(open.item),
            None => top.push(open.item),
        }
    }
}

pub fn items(source: &str, tokens: &[Token]) -> Vec<Item> {
    let text = |i: usize| tokens.get(i).map_or("", |t: &Token| t.text(source));
    let mut top = Vec::new();
    let mut stack: Vec<Open> = Vec::new();
    let mut decorated: Option<usize> = None;
    for line in logical_lines(source, tokens) {
        while stack.last().is_some_and(|o| o.indent >= line.indent) {
            close(&mut stack, &mut top);
        }
        for open in &mut stack {
            open.item.end = line.end_line;
        }
        if text(line.first) == "@" {
            decorated.get_or_insert(line.line);
            continue;
        }
        let start = decorated.take().unwrap_or(line.line);
        let keyword = if text(line.first) == "async" {
            line.first + 1
        } else {
            line.first
        };
        let kind = match text(keyword) {
            "def" => "function",
            "class" => "class",
            _ => continue,
        };
        stack.push(Open {
            item: Item {
                kind,
                name: text(keyword + 1).to_owned(),
                start,
                end: line.end_line,
                children: Vec::new(),
            },
            indent: line.indent,
        });
    }
    while !stack.is_empty() {
        close(&mut stack, &mut top);
    }
    top
}

#[cfg(test)]
mod tests;
