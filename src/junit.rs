//! JUnit XML -> {test_id: outcome}.

use roxmltree::{Document, Node, ParsingOptions};

use crate::model::{Outcome, Outcomes};

/// Several result elements in one testcase, or one id seen twice (pytest's failure + teardown
/// error, reruns), resolve to the most severe outcome.
fn severity(o: Outcome) -> u8 {
    match o {
        Outcome::Passed | Outcome::NotRun => 0,
        Outcome::Skipped => 1,
        Outcome::Xfailed => 2,
        Outcome::Xpassed => 3,
        Outcome::Failed => 4,
        Outcome::Error => 5,
    }
}

/// The attribute without a namespace.
fn attr<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.namespace().is_none() && a.name() == name)
        .map(|a| a.value())
}

fn is(node: Node<'_, '_>, name: &str) -> bool {
    node.tag_name().namespace().is_none() && node.tag_name().name() == name
}

/// Outcome per test id (`classname::name`, or `name` without a classname). Testcases without a
/// name are dropped.
pub fn parse_junit(text: &str) -> Result<Outcomes, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let opt = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(text, opt)
        .map_err(|e| format!("malformed JUnit XML: {e}"))?;
    let root = doc.root_element();
    if !(is(root, "testsuites") || is(root, "testsuite")) {
        let tag = root.tag_name();
        let name = match tag.namespace() {
            Some(ns) => format!("{{{ns}}}{}", tag.name()),
            None => tag.name().to_string(),
        };
        return Err(format!("not JUnit XML: root element is <{name}>"));
    }
    let mut outcomes = Outcomes::new();
    for case in root.descendants().filter(|n| is(*n, "testcase")) {
        let Some(name) = attr(case, "name").filter(|n| !n.is_empty()) else {
            continue;
        };
        let id = match attr(case, "classname") {
            Some(c) if !c.is_empty() => format!("{c}::{name}"),
            _ => name.to_string(),
        };
        let outcome = case
            .children()
            .filter(|n| n.is_element())
            .filter_map(result_outcome)
            .fold(Outcome::Passed, |a, b| {
                if severity(b) > severity(a) { b } else { a }
            });
        match outcomes.get(&id) {
            Some(&prev) if severity(outcome) <= severity(prev) => {}
            _ => {
                outcomes.insert(id, outcome);
            }
        }
    }
    Ok(outcomes)
}

fn result_outcome(elem: Node<'_, '_>) -> Option<Outcome> {
    // Children such as nextest's <flakyFailure>/<rerunFailure> or <system-out> carry no outcome.
    let message = attr(elem, "message").unwrap_or("");
    if is(elem, "error") {
        return Some(Outcome::Error);
    }
    if is(elem, "failure") {
        // pytest reports a strict xpass as a failure; a non-strict xpass is a plain pass.
        return Some(if message.starts_with("[XPASS(strict)]") {
            Outcome::Xpassed
        } else {
            Outcome::Failed
        });
    }
    if is(elem, "skipped") {
        // pytest types its skips; the message is then the bare reason, which may start with
        // "xfail".
        return Some(match attr(elem, "type") {
            Some("pytest.xfail") => Outcome::Xfailed,
            Some("pytest.skip") => Outcome::Skipped,
            // Old pytest (2.x) reported an xpass this way.
            _ if message.starts_with("xfail-marked test passes unexpectedly") => Outcome::Xpassed,
            _ if message.starts_with("xfail") => Outcome::Xfailed,
            _ => Outcome::Skipped,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcomes(xml: &str) -> Vec<(String, &'static str)> {
        parse_junit(xml)
            .unwrap()
            .into_iter()
            .map(|(k, v)| (k, v.as_str()))
            .collect()
    }

    fn case(body: &str) -> &'static str {
        let xml = format!("<testsuite><testcase classname='c' name='t'>{body}</testcase></testsuite>");
        outcomes(&xml)[0].1
    }

    #[test]
    fn outcomes_and_ids() {
        let xml = r#"<testsuites><testsuite>
            <testcase classname="c" name="a"/>
            <testcase classname="c" name="b"><skipped type="pytest.xfail" message="x"/></testcase>
            <testcase classname="c" name="b"><failure message="boom"/></testcase>
            <testcase name="d"><skipped message="xfail reason"/><system-out/></testcase>
            <testcase classname="" name="e"><failure message="[XPASS(strict)] ok"/></testcase>
            <testcase classname="c"><error/></testcase>
            <testcase classname="c" name=""><error/></testcase>
        </testsuite></testsuites>"#;
        let want = [
            ("c::a", "passed"),
            ("c::b", "failed"),
            ("d", "xfailed"),
            ("e", "xpassed"),
        ];
        let got = outcomes(xml);
        let got: Vec<(&str, &str)> = got.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn result_elements() {
        assert_eq!(case(""), "passed");
        assert_eq!(case("<error/>"), "error");
        assert_eq!(case("<failure/>"), "failed");
        assert_eq!(case("<skipped/>"), "skipped");
        assert_eq!(case("<skipped type='pytest.skip' message='xfail-ish reason'/>"), "skipped");
        assert_eq!(case("<skipped type='pytest.xfail' message='because'/>"), "xfailed");
        assert_eq!(case("<skipped message='xfail-marked test passes unexpectedly'/>"), "xpassed");
        assert_eq!(case("<flakyFailure/><rerunFailure/>"), "passed");
        assert_eq!(case("<failure/><error/>"), "error");
        assert_eq!(case("<skipped/><failure/>"), "failed");
    }

    #[test]
    fn nested_suites_and_bom() {
        let xml = "\u{feff}<testsuites><testsuite><testsuite>\
                   <testcase name='x'/></testsuite></testsuite></testsuites>";
        assert_eq!(outcomes(xml), [("x".to_string(), "passed")]);
    }

    #[test]
    fn duplicate_ids_take_most_severe() {
        let xml = "<testsuite><testcase name='x'><error/></testcase>\
                   <testcase name='x'/></testsuite>";
        assert_eq!(outcomes(xml), [("x".to_string(), "error")]);
    }

    #[test]
    fn namespaced_children_are_ignored() {
        let xml = "<testsuite xmlns:n='u'><testcase name='x'><n:failure/></testcase>\
                   <n:testcase name='y'/></testsuite>";
        assert_eq!(outcomes(xml), [("x".to_string(), "passed")]);
    }

    #[test]
    fn errors() {
        assert!(parse_junit("").unwrap_err().starts_with("malformed JUnit XML: "));
        assert!(parse_junit("<testsuite>").unwrap_err().starts_with("malformed JUnit XML: "));
        assert_eq!(
            parse_junit("<coverage/>").unwrap_err(),
            "not JUnit XML: root element is <coverage>"
        );
        assert_eq!(
            parse_junit("<a:x xmlns:a='u'/>").unwrap_err(),
            "not JUnit XML: root element is <{u}x>"
        );
    }
}
