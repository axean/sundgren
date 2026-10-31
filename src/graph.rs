//! Lane layout: a column per commit and the lines between rows.
//!
//! Rows are commits in the order given (children before parents). Each open line waits in a
//! column for the commit it leads to. A commit takes the column of the first line waiting for it,
//! or a free one; other lines waiting for it end there. Its first parent continues in its column,
//! further parents join a line already waiting for them or start a new one.

use serde::Serialize;

/// A line from `from` in the row above to `to` in this row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Line {
    pub from: usize,
    pub to: usize,
    /// Index into the renderer's palette; a new branch line gets the next one.
    pub color: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    pub column: usize,
    pub color: usize,
    /// Lines from the row above into this one.
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Layout {
    pub rows: Vec<Row>,
    /// Lines below the last row, to parents that weren't listed. `to` is their column.
    pub tail: Vec<Line>,
    /// The most columns any row needs.
    pub columns: usize,
}

/// One open line: it waits in its column for `oid`. `from` lists where it, and any merge lines
/// joining it, were in the row above.
struct Slot<'a> {
    oid: &'a str,
    color: usize,
    from: Vec<usize>,
}

/// Lay out commits given as (oid, parents), children before parents.
pub fn layout<'a, I, P>(commits: I) -> Layout
where
    I: IntoIterator<Item = (&'a str, P)>,
    P: IntoIterator<Item = &'a str>,
{
    let mut slots: Vec<Option<Slot<'a>>> = Vec::new();
    let mut rows = Vec::new();
    let mut columns = 0;
    let mut colors = 0..;
    let mut next_color = move || colors.next().unwrap_or_default();

    for (oid, parents) in commits {
        let waiting = slots
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.oid == oid));
        let (column, color) = match waiting {
            Some(i) => (i, slots[i].as_ref().map_or(0, |s| s.color)),
            None => (free(&mut slots), next_color()),
        };

        let mut lines = Vec::new();
        for (i, slot) in slots.iter_mut().enumerate() {
            let Some(s) = slot else { continue };
            let to = if s.oid == oid { column } else { i };
            lines.extend(s.from.iter().map(|&from| Line {
                from,
                to,
                color: s.color,
            }));
            if s.oid == oid {
                *slot = None;
            } else {
                s.from = vec![i];
            }
        }

        let mut seen: Vec<&str> = Vec::new();
        for (k, parent) in parents.into_iter().enumerate() {
            if seen.contains(&parent) {
                continue;
            }
            seen.push(parent);
            if k == 0 {
                slots[column] = Some(Slot {
                    oid: parent,
                    color,
                    from: vec![column],
                });
            } else if let Some(s) = slots.iter_mut().flatten().find(|s| s.oid == parent) {
                s.from.push(column);
            } else {
                let j = free(&mut slots);
                slots[j] = Some(Slot {
                    oid: parent,
                    color: next_color(),
                    from: vec![column],
                });
            }
        }

        columns = columns.max(slots.len());
        while matches!(slots.last(), Some(None)) {
            slots.pop();
        }
        rows.push(Row {
            column,
            color,
            lines,
        });
    }

    let tail = slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
        .flat_map(|(i, s)| {
            s.from.iter().map(move |&from| Line {
                from,
                to: i,
                color: s.color,
            })
        })
        .collect();
    Layout {
        rows,
        tail,
        columns,
    }
}

/// The first free column, adding one if none is.
fn free(slots: &mut Vec<Option<Slot<'_>>>) -> usize {
    if let Some(i) = slots.iter().position(Option::is_none) {
        return i;
    }
    slots.push(None);
    slots.len() - 1
}

#[cfg(test)]
mod tests;
