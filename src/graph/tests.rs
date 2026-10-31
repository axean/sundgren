use super::*;

fn run(commits: &[(&'static str, &[&'static str])]) -> Layout {
    layout(
        commits
            .iter()
            .map(|&(oid, parents)| (oid, parents.iter().copied())),
    )
}

fn line(from: usize, to: usize, color: usize) -> Line {
    Line { from, to, color }
}

#[test]
fn straight_history_is_one_column() {
    let l = run(&[("c", &["b"]), ("b", &["a"]), ("a", &[])]);
    assert_eq!(l.columns, 1);
    assert!(l.rows.iter().all(|r| r.column == 0 && r.color == 0));
    assert_eq!(l.rows[0].lines, []);
    assert_eq!(l.rows[1].lines, [line(0, 0, 0)]);
    assert_eq!(l.rows[2].lines, [line(0, 0, 0)]);
    assert_eq!(l.tail, []);
}

#[test]
fn merge_opens_a_column_and_closes_it_at_the_fork() {
    // m merges f into main; f and b both branch from a.
    let l = run(&[("m", &["b", "f"]), ("f", &["a"]), ("b", &["a"]), ("a", &[])]);
    assert_eq!(l.columns, 2);
    let columns: Vec<_> = l.rows.iter().map(|r| r.column).collect();
    assert_eq!(columns, [0, 1, 0, 0]);
    // f's line starts at m and leans into column 1.
    assert_eq!(l.rows[1].lines, [line(0, 0, 0), line(0, 1, 1)]);
    assert_eq!(l.rows[1].color, 1);
    assert_eq!(l.rows[2].lines, [line(0, 0, 0), line(1, 1, 1)]);
    // Both lines wait for a; the one from column 1 ends there.
    assert_eq!(l.rows[3].lines, [line(0, 0, 0), line(1, 0, 1)]);
    assert_eq!(l.tail, []);
}

#[test]
fn merge_parent_already_waited_for_joins_that_line() {
    // Two merges of the same commit x: the second joins x's open line.
    let l = run(&[
        ("m2", &["m1", "x"]),
        ("m1", &["a", "x"]),
        ("x", &["a"]),
        ("a", &[]),
    ]);
    assert_eq!(l.columns, 2);
    assert_eq!(l.rows[1].lines, [line(0, 0, 0), line(0, 1, 1)]);
    // m1's merge line and the open line both arrive at x.
    assert_eq!(
        l.rows[2].lines,
        [line(0, 0, 0), line(1, 1, 1), line(0, 1, 1)]
    );
}

#[test]
fn freed_column_is_reused() {
    let l = run(&[
        ("m2", &["m1", "y"]),
        ("y", &["m1"]),
        ("m1", &["a", "x"]),
        ("x", &["a"]),
        ("a", &[]),
    ]);
    assert_eq!(l.columns, 2);
    let columns: Vec<_> = l.rows.iter().map(|r| r.column).collect();
    assert_eq!(columns, [0, 1, 0, 1, 0]);
    let colors: Vec<_> = l.rows.iter().map(|r| r.color).collect();
    assert_eq!(colors, [0, 1, 0, 2, 0]);
}

#[test]
fn unlisted_parents_go_to_the_tail() {
    let l = run(&[("m", &["b", "f"]), ("b", &["a"])]);
    assert_eq!(l.tail, [line(0, 0, 0), line(1, 1, 1)]);
}

#[test]
fn several_tips_each_get_a_column() {
    // Two unrelated roots listed together, e.g. from --all.
    let l = run(&[("b", &["a"]), ("y", &["x"]), ("a", &[]), ("x", &[])]);
    let columns: Vec<_> = l.rows.iter().map(|r| r.column).collect();
    assert_eq!(columns, [0, 1, 0, 1]);
    assert_eq!(l.columns, 2);
}

#[test]
fn duplicate_parents_count_once() {
    let l = run(&[("m", &["a", "a"]), ("a", &[])]);
    assert_eq!(l.columns, 1);
    assert_eq!(l.rows[1].lines, [line(0, 0, 0)]);
}
