use super::*;

fn suite(cases: &[&str]) -> String {
    format!("<testsuite name=\"s\">{}</testsuite>", cases.concat())
}

fn outcomes(xml: &str) -> Vec<(String, &'static str)> {
    let got = parse_junit(xml).unwrap();
    got.into_iter().map(|(k, v)| (k, v.as_str())).collect()
}

fn want(pairs: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn fixture(name: &str) -> Vec<(String, &'static str)> {
    let path = format!("{}/tests/fixtures/junit/{name}", env!("CARGO_MANIFEST_DIR"));
    outcomes(&std::fs::read_to_string(path).unwrap())
}

#[test]
fn pytest_fixture() {
    // pytest 9.1.1 with --junitxml and --continue-on-collection-errors.
    let m = "tests.test_outcomes";
    let ids = [
        ("tests.test_broken_import".to_string(), "error"), // collection error
        (format!("{m}.TestCalc::test_method"), "passed"),
        (format!("{m}.TestCalc::test_method_fail"), "failed"),
        (format!("{m}.TestOuter.TestInner::test_nested"), "passed"),
        (format!("{m}::test_fail"), "failed"),
        (format!("{m}::test_fail_and_teardown_error"), "error"),
        (format!("{m}::test_param[-1-neg]"), "passed"),
        (format!("{m}::test_param[0-zero]"), "passed"),
        (format!("{m}::test_param[5-pos]"), "passed"),
        (format!("{m}::test_pass"), "passed"),
        (format!("{m}::test_setup_error"), "error"),
        (format!("{m}::test_skip"), "skipped"),
        (format!("{m}::test_skip_imperative"), "skipped"),
        (format!("{m}::test_teardown_error"), "error"),
        (format!("{m}::test_xfail"), "xfailed"),
        (format!("{m}::test_xfail_imperative"), "xfailed"),
        (format!("{m}::test_xpass_nonstrict"), "passed"), // looks like a pass
        (format!("{m}::test_xpass_strict"), "xpassed"),
    ];
    assert_eq!(fixture("pytest.xml"), ids);
}

#[test]
fn nextest_fixture() {
    let ids = [
        ("shed-rs::calc_it::classify_negative", "passed"), // <flakyFailure> only
        ("shed-rs::tests::test_add", "passed"),
        ("shed-rs::tests::test_classify", "skipped"),
        ("shed-rs::tests::test_divide", "failed"),
    ];
    assert_eq!(fixture("nextest.xml"), want(&ids));
}

#[test]
fn classname_fallback_and_nesting() {
    let xml = suite(&[
        "<testcase classname=\"pkg.mod\" name=\"t1\"/>",
        "<testcase classname=\"\" name=\"t2\"/>",
        "<testcase name=\"t3\"/>",
    ]);
    let ids = [
        ("pkg.mod::t1", "passed"),
        ("t2", "passed"),
        ("t3", "passed"),
    ];
    assert_eq!(outcomes(&xml), want(&ids));
    let inner = suite(&["<testcase classname=\"b\" name=\"t2\"><failure/></testcase>"]);
    let xml = format!(
        "<testsuites><testsuite name=\"o\"><testcase classname=\"a\" name=\"t1\"/>\
             {inner}</testsuite></testsuites>"
    );
    assert_eq!(
        outcomes(&xml),
        want(&[("a::t1", "passed"), ("b::t2", "failed")])
    );
}

#[test]
fn outcome_mapping() {
    for (child, outcome) in [
        ("<failure message=\"boom\"/>", "failed"),
        ("<error/>", "error"),
        ("<skipped message=\"no\"/>", "skipped"),
        ("<skipped/>", "skipped"),
        (
            "<skipped type=\"pytest.xfail\" message=\"known bug\"/>",
            "xfailed",
        ),
        (
            "<skipped type=\"pytest.skip\" message=\"xfails on windows\"/>",
            "skipped",
        ),
        ("<skipped message=\"xfail: reason\"/>", "xfailed"),
        (
            "<skipped message=\"xfail-marked test passes unexpectedly\"/>",
            "xpassed",
        ),
        (
            "<failure message=\"[XPASS(strict)] must fail\"/>",
            "xpassed",
        ),
        ("<system-out>log</system-out><properties/>", "passed"),
        ("<rerunFailure/><rerunFailure/>", "passed"),
        (
            "<skipped message=\"x\"/><error message=\"teardown\"/>",
            "error",
        ),
    ] {
        let xml = suite(&[&format!(
            "<testcase classname=\"c\" name=\"t\">{child}</testcase>"
        )]);
        assert_eq!(outcomes(&xml), want(&[("c::t", outcome)]), "{child}");
    }
}

#[test]
fn duplicate_id_resolves_to_most_severe() {
    for (first, second, outcome) in [
        ("", "<error/>", "error"),
        ("<error/>", "", "error"),
        ("<failure/>", "<error/>", "error"),
        ("<failure/>", "", "failed"),
        ("", "<skipped/>", "skipped"),
        ("<skipped/>", "<skipped type=\"pytest.xfail\"/>", "xfailed"),
    ] {
        let xml = suite(&[
            &format!("<testcase classname=\"c\" name=\"t\">{first}</testcase>"),
            &format!("<testcase classname=\"c\" name=\"t\">{second}</testcase>"),
        ]);
        assert_eq!(outcomes(&xml), want(&[("c::t", outcome)]));
    }
}

#[test]
fn empty_bom_and_nameless() {
    assert!(outcomes("<?xml version=\"1.0\" encoding=\"utf-8\"?><testsuites/>").is_empty());
    let xml = format!(
        "\u{feff}<?xml version=\"1.0\"?>{}",
        suite(&["<testcase name=\"t\"/>"])
    );
    assert_eq!(outcomes(&xml), want(&[("t", "passed")]));
    let xml = suite(&[
        "<testcase classname='c'><failure/></testcase>",
        "<testcase classname='c' name=''/>",
        "<testcase classname='c' name='t'/>",
    ]);
    assert_eq!(outcomes(&xml), want(&[("c::t", "passed")]));
}

#[test]
fn namespaced_elements_are_ignored() {
    let xml = "<testsuite xmlns:n='u'><testcase name='x'><n:failure/></testcase>\
                   <n:testcase name='y'/></testsuite>";
    assert_eq!(outcomes(xml), want(&[("x", "passed")]));
}

#[test]
fn invalid_input() {
    for text in ["", "<testsuite><testcase name='t'>", "not xml at all"] {
        let err = parse_junit(text).unwrap_err();
        assert!(err.starts_with("malformed JUnit XML: "), "{err}");
    }
    assert!(parse_junit(&suite(&["<testcase name='t\0'/>"])).is_err());
    assert_eq!(
        parse_junit("<coverage/>").unwrap_err(),
        "not JUnit XML: root element is <coverage>"
    );
    assert_eq!(
        parse_junit("<a:x xmlns:a='u'/>").unwrap_err(),
        "not JUnit XML: root element is <{u}x>"
    );
}
