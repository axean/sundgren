//! JUnit XML -> {test_id: outcome}. Port of `sundgren/core/junit.py`.

use std::collections::BTreeMap;
use std::fmt;

use roxmltree::{Document, Node, ParsingOptions};

use crate::model::Outcome;
use crate::pyint::py_repr;

/// Malformed XML or not JUnit. Displays as the prototype's ValueError message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunitError(pub String);

impl fmt::Display for JunitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JunitError {}

/// Several result elements in one testcase, or one id seen twice (pytest's failure + teardown
/// error, reruns), resolve to the most severe outcome.
fn severity(o: Outcome) -> u8 {
    match o {
        Outcome::Passed => 0,
        Outcome::Skipped => 1,
        Outcome::Xfailed => 2,
        Outcome::Xpassed => 3,
        Outcome::Failed => 4,
        Outcome::Error => 5,
        Outcome::NotRun => 0,
    }
}

/// ElementTree's tag: `name`, or `{namespace}name`.
fn tag(node: Node<'_, '_>) -> String {
    let t = node.tag_name();
    match t.namespace() {
        Some(ns) => format!("{{{ns}}}{}", t.name()),
        None => t.name().to_string(),
    }
}

/// `elem.get(name)`: the attribute without a namespace.
fn attr<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.namespace().is_none() && a.name() == name)
        .map(|a| a.value())
}

fn is(node: Node<'_, '_>, name: &str) -> bool {
    node.tag_name().namespace().is_none() && node.tag_name().name() == name
}

/// Outcome per test id (`classname::name`, or `name` without a classname), sorted by id.
pub fn parse_junit(text: &str) -> Result<BTreeMap<String, Outcome>, JunitError> {
    parse_junit_logged(text, &mut |_| {})
}

/// `parse_junit`, sending debug messages (testcases without a name) to `log`.
pub fn parse_junit_logged(
    text: &str,
    log: &mut dyn FnMut(String),
) -> Result<BTreeMap<String, Outcome>, JunitError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let opt = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(text, opt)
        .map_err(|e| JunitError(format!("malformed JUnit XML: {e}")))?;
    let root = doc.root_element();
    if !(is(root, "testsuites") || is(root, "testsuite")) {
        return Err(JunitError(format!(
            "not JUnit XML: root element is <{}>",
            tag(root)
        )));
    }
    let mut outcomes: BTreeMap<String, Outcome> = BTreeMap::new();
    for case in root.descendants().filter(|n| is(*n, "testcase")) {
        let Some(test_id) = test_id(case) else {
            log(format!(
                "JUnit: skipping testcase without a name: {}",
                attrib_repr(case)
            ));
            continue;
        };
        let outcome = case_outcome(case);
        match outcomes.get(&test_id) {
            Some(&prev) if severity(outcome) <= severity(prev) => {}
            _ => {
                outcomes.insert(test_id, outcome);
            }
        }
    }
    Ok(outcomes)
}

/// `repr(elem.attrib)`.
fn attrib_repr(node: Node<'_, '_>) -> String {
    let items: Vec<String> = node
        .attributes()
        .map(|a| {
            let key = match a.namespace() {
                Some(ns) => format!("{{{ns}}}{}", a.name()),
                None => a.name().to_string(),
            };
            format!("{}: {}", py_repr(&key), py_repr(a.value()))
        })
        .collect();
    format!("{{{}}}", items.join(", "))
}

fn test_id(case: Node<'_, '_>) -> Option<String> {
    let name = attr(case, "name").filter(|n| !n.is_empty())?;
    Some(match attr(case, "classname") {
        Some(c) if !c.is_empty() => format!("{c}::{name}"),
        _ => name.to_string(),
    })
}

fn case_outcome(case: Node<'_, '_>) -> Outcome {
    let mut outcome = Outcome::Passed;
    for child in case.children().filter(|n| n.is_element()) {
        if let Some(found) = result_outcome(child)
            && severity(found) > severity(outcome)
        {
            outcome = found;
        }
    }
    outcome
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

    #[test]
    fn outcomes() {
        let xml = r#"<testsuites><testsuite>
            <testcase classname="c" name="a"/>
            <testcase classname="c" name="b"><skipped type="pytest.xfail" message="x"/></testcase>
            <testcase classname="c" name="b"><failure message="boom"/></testcase>
            <testcase name="d"><skipped message="xfail reason"/><system-out/></testcase>
            <testcase classname="" name="e"><failure message="[XPASS(strict)] ok"/></testcase>
            <testcase classname="c"><error/></testcase>
        </testsuite></testsuites>"#;
        let mut logs = Vec::new();
        let got = parse_junit_logged(xml, &mut |m| logs.push(m)).unwrap();
        let got: Vec<_> = got.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            got,
            [
                ("c::a", "passed"),
                ("c::b", "failed"),
                ("d", "xfailed"),
                ("e", "xpassed")
            ]
        );
        assert_eq!(
            logs,
            ["JUnit: skipping testcase without a name: {'classname': 'c'}"]
        );
    }

    #[test]
    fn errors() {
        assert!(
            parse_junit("")
                .unwrap_err()
                .0
                .starts_with("malformed JUnit XML: ")
        );
        assert_eq!(
            parse_junit("<coverage/>").unwrap_err().0,
            "not JUnit XML: root element is <coverage>"
        );
        assert_eq!(
            parse_junit("<a:x xmlns:a='u'/>").unwrap_err().0,
            "not JUnit XML: root element is <{u}x>"
        );
    }
}
