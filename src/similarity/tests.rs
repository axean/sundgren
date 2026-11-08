use super::*;

fn lines(text: &str) -> Vec<String> {
    text.lines().filter_map(normalize).collect()
}

#[test]
fn normalize_collapses_whitespace_and_drops_noise() {
    assert_eq!(normalize("  let  x =\t1; "), Some("let x = 1;".to_owned()));
    assert_eq!(normalize("   "), None);
    assert_eq!(normalize("    });"), None);
    assert_eq!(normalize("}"), None);
}

#[test]
fn line_hash_is_stable_fnv1a() {
    assert_eq!(line_hash(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(line_hash("a"), 0xaf63_dc4c_8601_ec8c);
    assert_ne!(line_hash("ab"), line_hash("ba"));
}

#[test]
fn exact_matches_use_each_line_of_b_once() {
    let a = lines("x = 1\nx = 1\ny = 2");
    let b = lines("x = 1\ny = 2");
    assert_eq!(
        find_lines(&a, &b, 2.0),
        [
            Found::Exact { at: 0 },
            Found::Missing,
            Found::Exact { at: 1 }
        ]
    );
    // Repeated lines of B are used in order.
    let found = find_lines(
        &lines("x = 1\nx = 1\nx = 1"),
        &lines("y = 2\nx = 1\nx = 1"),
        2.0,
    );
    assert_eq!(
        found,
        [
            Found::Exact { at: 1 },
            Found::Exact { at: 2 },
            Found::Missing
        ]
    );
    assert_eq!(found[0].at(), Some(1));
    assert_eq!(found[2].at(), None);
}

#[test]
fn fuzzy_matches_find_edited_lines() {
    let a = lines("result = compute_the_answer(values, scale)\nnothing like this at all");
    let b = lines("result = compute_the_answer(values, scale=2)");
    let found = find_lines(&a, &b, 0.8);
    match found[0] {
        Found::Fuzzy { score, .. } => assert!(score > 0.9 && score < 1.0, "{score}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(found[1], Found::Missing);
    // Above the threshold nothing fuzzy is accepted.
    assert_eq!(find_lines(&a, &b, 0.99)[0], Found::Missing);
}

#[test]
fn fuzzy_takes_the_best_candidate() {
    let a = lines("value = alpha + beta");
    let b = lines("value = alpha + gamma\nvalue = alpha + betas\nunrelated line here");
    let Found::Fuzzy { score, at } = find_lines(&a, &b, 0.5)[0] else {
        panic!("not fuzzy")
    };
    assert_eq!(at, 1);
    let expected = levenshtein::normalized_similarity(
        "value = alpha + beta".chars(),
        "value = alpha + betas".chars(),
    );
    assert!((score - expected).abs() < 1e-12);
}

#[test]
fn in_order_counts_the_common_subsequence() {
    let h = |s: &str| line_hash(s);
    let a = [h("a"), h("b"), h("c"), h("d")];
    let b = [h("a"), h("x"), h("c"), h("b"), h("d")];
    assert_eq!(in_order(&a, &b), 3);
    assert_eq!(in_order(&a, &[]), 0);
}

#[test]
fn edits_count_characters_changed() {
    assert_eq!(edits("size_t index", "size_t offset"), 5);
    assert_eq!(edits("same", "same"), 0);
}

#[test]
fn threads_do_not_change_the_result() {
    // Lines with shared words, so many are fuzzy matches, some exact, some missing.
    let words = [
        "alpha", "beta", "gamma", "delta", "value", "scale", "index", "count",
    ];
    let line = |i: usize| {
        format!(
            "{} = {}({}, {})",
            words[i % 8],
            words[i / 8 % 8],
            words[i / 3 % 8],
            i % 13
        )
    };
    let a: Vec<String> = (0..300).map(line).collect();
    let b: Vec<String> = (150..600).step_by(2).map(line).collect();
    let a_places = places(a.len());
    let b_places = places(b.len());
    let lines = |texts, places| Lines { texts, places };
    let (a, b) = (lines(&a, &a_places), lines(&b, &b_places));
    let single = find_lines_with(a, b, 0.8, 1);
    assert!(single.iter().any(|f| matches!(f, Found::Exact { .. })));
    assert!(single.iter().any(|f| matches!(f, Found::Fuzzy { .. })));
    assert!(single.contains(&Found::Missing));
    for threads in [2, 5, 12, 1000] {
        assert_eq!(
            find_lines_with(a, b, 0.8, threads),
            single,
            "{threads} threads"
        );
    }
}

/// Places for `count` lines: a few files and items, some lines outside items.
fn places(count: usize) -> Vec<Place> {
    (0..count)
        .map(|i| Place {
            file: Some(u32::try_from(i % 3).unwrap()),
            item: (i % 4 != 0).then(|| u32::try_from(i % 5).unwrap()),
        })
        .collect()
}

fn at(file: u32, item: Option<u32>) -> Place {
    Place {
        file: Some(file),
        item,
    }
}

#[test]
fn exact_matches_prefer_the_same_file_and_item() {
    let a = lines("x = 1");
    let b = lines("x = 1\nx = 1\nx = 1\nx = 1");
    // B: another file and item, an item of the same name elsewhere, the same file, the same file
    // and item.
    let b_places = [
        at(1, Some(1)),
        at(2, Some(0)),
        at(0, Some(3)),
        at(0, Some(0)),
    ];
    let found = |b_places: &[Place]| find_lines_placed(&a, &[at(0, Some(0))], &b, b_places, 2.0);
    assert_eq!(found(&b_places), [Found::Exact { at: 3 }]);
    assert_eq!(found(&b_places[..3]), [Found::Exact { at: 2 }]);
    assert_eq!(found(&b_places[..2]), [Found::Exact { at: 1 }]);
    assert_eq!(found(&b_places[..1]), [Found::Exact { at: 0 }]);
    // Without places, the first.
    assert_eq!(find_lines(&a, &b, 2.0), [Found::Exact { at: 0 }]);
}

#[test]
fn exact_matches_go_to_the_nearest_line_first() {
    // The first line of A has no copy in its own file; the second does. The second gets its own
    // file's copy even though the first comes first; the first takes the other one.
    let a = lines("x = 1\nx = 1");
    let b = lines("x = 1\nx = 1");
    let found = find_lines_placed(
        &a,
        &[at(5, None), at(1, None)],
        &b,
        &[at(1, None), at(2, None)],
        2.0,
    );
    assert_eq!(found, [Found::Exact { at: 1 }, Found::Exact { at: 0 }]);
}

#[test]
fn fuzzy_matches_prefer_the_nearest_copy_of_the_best_text() {
    let a = lines("result = compute_the_answer(values, scale)");
    // The same best text three times, and an equally good other text; then a worse one.
    let b = lines(
        "result = compute_the_answer(values, scale=2)\nresult = compute_the_answer(values, scale=2)\nresult = compute_the_answer(values, scale=3)\nresult = compute_the_answer(values, scale=2)\nresult = compute_the_answer(values_all, scale=2)",
    );
    let b_places = [
        at(1, None),
        at(2, Some(7)),
        at(3, None),
        at(0, Some(8)),
        at(0, Some(7)),
    ];
    let found = |a_place| match find_lines_placed(&a, &[a_place], &b, &b_places, 0.8)[0] {
        Found::Fuzzy { at, .. } => at,
        other => panic!("{other:?}"),
    };
    // The same file: line 3 (line 4 scores less, so it isn't a candidate even in the same item).
    assert_eq!(found(at(0, Some(7))), 3);
    // An item of the same name elsewhere.
    assert_eq!(found(at(9, Some(7))), 1);
    // A tied other text in the same file.
    assert_eq!(found(at(3, None)), 2);
    // Nowhere near: the first.
    assert_eq!(found(at(9, None)), 0);
}

#[test]
fn places_change_where_lines_are_found_not_how_many() {
    let words = ["alpha", "beta", "gamma", "delta", "value"];
    let line = |i: usize| format!("{} = {}({})", words[i % 5], words[i / 5 % 5], i % 7);
    let a: Vec<String> = (0..200).map(line).collect();
    let b: Vec<String> = (100..400).map(line).collect();
    let summary = |found: &[Found]| {
        found
            .iter()
            .map(|f| match f {
                Found::Exact { .. } => (0, 0),
                Found::Fuzzy { score, .. } => (1, (score * 1e9) as i64),
                Found::Missing => (2, 0),
            })
            .collect::<Vec<_>>()
    };
    let without = find_lines(&a, &b, 0.7);
    let with = find_lines_placed(&a, &places(a.len()), &b, &places(b.len()), 0.7);
    assert_eq!(summary(&with), summary(&without));
    assert_ne!(with, without);
}
