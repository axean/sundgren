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
        [Found::Exact, Found::Missing, Found::Exact]
    );
}

#[test]
fn fuzzy_matches_find_edited_lines() {
    let a = lines("result = compute_the_answer(values, scale)\nnothing like this at all");
    let b = lines("result = compute_the_answer(values, scale=2)");
    let found = find_lines(&a, &b, 0.8);
    match found[0] {
        Found::Fuzzy { score } => assert!(score > 0.9 && score < 1.0, "{score}"),
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
    let Found::Fuzzy { score } = find_lines(&a, &b, 0.5)[0] else {
        panic!("not fuzzy")
    };
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
