"""core.report.text_report: terminal summary rendered from the report dict."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from sundgren.core.diff import diff
from sundgren.core.model import FileCoverage, FunctionRecord, ReportWarning, Snapshot
from sundgren.core.report.json_report import build_report
from sundgren.core.report.text_report import render_text

META: dict[str, Any] = {"mode": "diff"}  # render_text never reads meta
HINTS = ("Set [run] source = <your package>.",)


def snap(*files: FileCoverage) -> Snapshot:
    return Snapshot({f.path: f for f in files}, ())


def fc(
    path: str, lines: dict[int, int], functions: tuple[FunctionRecord, ...] = ()
) -> FileCoverage:
    return FileCoverage(path, lines, {}, functions, False)


def skip_divide(**kw: Any) -> dict[str, Any]:
    fns = (FunctionRecord("add", 1, 2, 1), FunctionRecord("divide", 4, 7, 1))
    base = fc("shed/calc.py", {1: 1, 2: 1, 4: 1, 5: 1, 6: 1, 7: 1}, fns)
    patched = fc("shed/calc.py", {1: 1, 2: 1, 4: 1, 5: 0, 6: 0, 7: 0}, fns)
    d = diff(
        [snap(base)],
        [snap(patched)],
        {},
        base_outcomes={"t::test_add": "passed", "t::test_divide": "passed"},
        patched_outcomes={"t::test_add": "passed", "t::test_divide": "skipped"},
    )
    return build_report(d, META, **kw)


def test_canonical_text() -> None:
    assert render_text(skip_divide(), HINTS) == (
        "Tests with a changed outcome\n"
        "  t::test_divide: passed -> skipped\n"
        "\n"
        "Summary\n"
        "  lines      lost 3, gained 0, new hit 0, new miss 0, deleted hit 0, deleted miss 0,"
        " flaky 0, unmeasured 0\n"
        "  branches   lost 0, gained 0, new 0, deleted 0\n"
        "  functions  went dark 1, lit up 0, partial loss 0, partial gain 0, mixed 0, added 0,"
        " removed 0\n"
        "  coverage   100.0% -> 50.0%\n"
        "\n"
        "Files (top 20 by lost + gained lines)\n"
        "  shed/calc.py: lost 3\n"
        "    divide (4-7): went_dark, lost 3, gained 0\n"
    )


def test_full_report_renders_the_same_changes() -> None:
    # --full adds unchanged tests/functions/lines; the text still shows only changes.
    assert render_text(skip_divide(full=True)) == render_text(skip_divide())


def test_independent_of_key_order() -> None:
    r = skip_divide(extra_warnings=[ReportWarning("dirty_tree", "uncommitted changes")])
    reloaded = json.loads(json.dumps(r, sort_keys=True))
    assert render_text(reloaded, HINTS) == render_text(r, HINTS)


def test_warnings_then_hints() -> None:
    r = skip_divide(extra_warnings=[ReportWarning("dirty_tree", "uncommitted changes")])
    text = render_text(r, HINTS)
    assert text.endswith(
        "\nWarnings\n  [dirty_tree] uncommitted changes\n"
        "\nHints\n  - Set [run] source = <your package>.\n"
    )
    order = [text.index(h) for h in ("Tests with", "Summary", "Files (top", "Warnings", "Hints")]
    assert order == sorted(order)


def test_hints_only_when_relevant() -> None:
    assert "Hints" not in render_text(skip_divide(), HINTS)
    r = skip_divide()
    r["files"][0]["inferred_side"] = "patched"
    assert "Hints" in render_text(r, HINTS)
    assert "Hints" not in render_text(skip_divide(extra_warnings=[ReportWarning("x", "y")]))


def test_no_hints_for_empty_file_missing_on_one_side(tmp_path: Path) -> None:
    (tmp_path / "pkg").mkdir()
    (tmp_path / "pkg" / "__init__.py").write_text("")
    d = diff([snap()], [snap(fc("pkg/__init__.py", {}))], {}, base_root=tmp_path)
    assert "Hints" not in render_text(build_report(d, META, full=True), HINTS)


def test_top_files_limit_and_order() -> None:
    base = [fc(f"pkg/m{i:02}.py", {ln: 1 for ln in range(1, i + 2)}) for i in range(25)]
    patched = [fc(f.path, {ln: 0 for ln in f.lines}) for f in base]
    text = render_text(build_report(diff([snap(*base)], [snap(*patched)], {}), META))
    listed = [ln.split(":")[0].strip() for ln in text.splitlines() if ln.startswith("  pkg/")]
    assert listed == [f"pkg/m{i:02}.py" for i in range(24, 4, -1)]
    assert "  ... and 5 more\n" in text


def test_nothing_changed() -> None:
    d = diff([snap(fc("a.py", {}))], [snap(fc("a.py", {}))], {})
    text = render_text(build_report(d, META), HINTS)
    assert "Tests with" not in text
    assert "  coverage   n/a -> n/a\n" in text
    assert text.endswith("No coverage changes.\n")


def test_external_files_mentioned() -> None:
    ext = FileCoverage("/usr/lib/x.py", {1: 1}, {}, (), True)
    text = render_text(build_report(diff([snap(ext)], [snap(ext)], {}), META))
    assert "  external files (not in tree): 1\n" in text
