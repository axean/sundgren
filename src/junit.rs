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
    let doc =
        Document::parse_with_options(text, opt).map_err(|e| format!("malformed JUnit XML: {e}"))?;
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
mod tests;
