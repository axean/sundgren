"""JUnit XML -> {test_id: outcome}."""

from __future__ import annotations

import logging
import xml.etree.ElementTree as ET

from sundgren.core.model import Outcome

_log = logging.getLogger(__name__)

# Several result elements in one testcase, or one id seen twice (pytest's failure + teardown
# error, reruns), resolve to the most severe outcome.
_SEVERITY: dict[Outcome, int] = {
    "passed": 0,
    "skipped": 1,
    "xfailed": 2,
    "xpassed": 3,
    "failed": 4,
    "error": 5,
}


def parse_junit(text: str) -> dict[str, Outcome]:
    """Outcome per test id (`classname::name`, or `name` without a classname), sorted by id."""
    try:
        root = ET.fromstring(text)
    except ET.ParseError as exc:
        raise ValueError(f"malformed JUnit XML: {exc}") from exc
    if root.tag not in ("testsuites", "testsuite"):
        raise ValueError(f"not JUnit XML: root element is <{root.tag}>")
    outcomes: dict[str, Outcome] = {}
    for case in root.iter("testcase"):
        test_id = _test_id(case)
        if test_id is None:
            _log.debug("JUnit: skipping testcase without a name: %r", case.attrib)
            continue
        outcome = _case_outcome(case)
        previous = outcomes.get(test_id)
        if previous is None or _SEVERITY[outcome] > _SEVERITY[previous]:
            outcomes[test_id] = outcome
    return dict(sorted(outcomes.items()))


def _test_id(case: ET.Element) -> str | None:
    name = case.get("name")
    if not name:
        return None
    classname = case.get("classname")
    return f"{classname}::{name}" if classname else name


def _case_outcome(case: ET.Element) -> Outcome:
    outcome: Outcome = "passed"
    for child in case:
        found = _result_outcome(child)
        if found is not None and _SEVERITY[found] > _SEVERITY[outcome]:
            outcome = found
    return outcome


def _result_outcome(elem: ET.Element) -> Outcome | None:
    # Children such as nextest's <flakyFailure>/<rerunFailure> or <system-out> carry no outcome.
    message = elem.get("message", "")
    if elem.tag == "error":
        return "error"
    if elem.tag == "failure":
        # pytest reports a strict xpass as a failure; a non-strict xpass is a plain pass.
        return "xpassed" if message.startswith("[XPASS(strict)]") else "failed"
    if elem.tag == "skipped":
        # pytest types its skips; the message is then the bare reason, which may start with "xfail".
        kind = elem.get("type")
        if kind == "pytest.xfail":
            return "xfailed"
        if kind == "pytest.skip":
            return "skipped"
        # Old pytest (2.x) reported an xpass this way.
        if message.startswith("xfail-marked test passes unexpectedly"):
            return "xpassed"
        return "xfailed" if message.startswith("xfail") else "skipped"
    return None
