"""Render a recorded sundgren report as an annotated source graphic (HTML + CSS, no JS)."""

from __future__ import annotations

import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from html import escape
from typing import Any

Report = Mapping[str, Any]

LINE_LABELS = {
    "lost": "lost",
    "gained": "gained",
    "kept_hit": "kept hit",
    "kept_miss": "kept miss",
    "new_hit": "new hit",
    "new_miss": "new miss",
    "deleted_hit": "deleted hit",
    "deleted_miss": "deleted miss",
    "unmeasured": "unmeasured",
    "flaky": "flaky",
}
BRANCH_LABELS = {
    "branch_lost": "lost",
    "branch_gained": "gained",
    "branch_kept_hit": "kept hit",
    "branch_kept_miss": "kept miss",
    "branch_new": "new",
    "branch_deleted": "deleted",
    "branch_flaky": "flaky",
    "branch_unmeasured": "unmeasured",
}
FUNCTION_LABELS = {
    "went_dark": "went dark",
    "lit_up": "lit up",
    "partial_loss": "partial loss",
    "partial_gain": "partial gain",
    "mixed": "mixed",
    "unchanged": "unchanged",
    "added": "added",
    "removed": "removed",
}
OUTCOMES = ("passed", "failed", "skipped", "xfailed", "xpassed", "error", "not_run")
_KEPT_LINES = {"kept_hit", "kept_miss"}
_KEPT_BRANCHES = {"branch_kept_hit", "branch_kept_miss"}
_MARK_CLASS = {" ": "sg-m-ctx", "+": "sg-m-add", "-": "sg-m-del"}

# (section, key, singular, plural, verb) for the headline chips, in display order.
_HEADLINE = (
    ("lines", "lost", "line", "lines", "lost"),
    ("lines", "gained", "line", "lines", "gained"),
    ("lines", "new_hit", "new line", "new lines", "hit"),
    ("lines", "new_miss", "new line", "new lines", "missed"),
    ("lines", "deleted_hit", "hit line", "hit lines", "deleted"),
    ("lines", "deleted_miss", "missed line", "missed lines", "deleted"),
    ("lines", "flaky", "line", "lines", "flaky"),
    ("lines", "unmeasured", "line", "lines", "unmeasured"),
    ("branches", "lost", "branch", "branches", "lost"),
    ("branches", "gained", "branch", "branches", "gained"),
    ("branches", "new", "new branch", "new branches", ""),
    ("branches", "deleted", "branch", "branches", "deleted"),
    ("functions", "went_dark", "function", "functions", "went dark"),
    ("functions", "lit_up", "function", "functions", "lit up"),
    ("functions", "partial_loss", "function", "functions", "partially lost"),
    ("functions", "partial_gain", "function", "functions", "partially gained"),
    ("functions", "mixed", "function", "functions", "mixed"),
    ("functions", "added", "function", "functions", "added"),
    ("functions", "removed", "function", "functions", "removed"),
)


@dataclass(frozen=True, slots=True)
class Row:
    base: int | None  # base line number; None for an added line
    new: int | None  # patched line number; None for a deleted line
    text: str
    mark: str  # " " context, "+" added, "-" deleted


def split_lines(text: str) -> list[str]:
    """Lines split on '\\n' only, as the patch parser does; no trailing empty line."""
    lines = text.split("\n")
    if lines[-1] == "":
        lines.pop()
    return lines


@dataclass
class Hunk:
    old_start: int
    old_count: int
    lines: list[str] = field(default_factory=list[str])


@dataclass
class FilePatch:
    old_path: str | None
    new_path: str | None
    hunks: list[Hunk] = field(default_factory=list[Hunk])


_HUNK = re.compile(r"@@ -(\d+)(?:,(\d+))? \+\d+(?:,\d+)? @@")


def _diff_path(header: str) -> str | None:
    path = header[4:].split("\t")[0].rstrip("\n")
    return None if path == "/dev/null" else path[2:]


def parse_patch(text: str) -> list[FilePatch]:
    """The files and hunks of a plain unified diff, as the docs examples use."""
    files: list[FilePatch] = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("--- ") and i + 1 < len(lines) and lines[i + 1].startswith("+++ "):
            files.append(FilePatch(_diff_path(line), _diff_path(lines[i + 1])))
            i += 2
            continue
        m = _HUNK.match(line)
        if m and files:
            files[-1].hunks.append(Hunk(int(m[1]), int(m[2] or 1)))
        elif files and files[-1].hunks and line[:1] in (" ", "-", "+"):
            files[-1].hunks[-1].lines.append(line)
        i += 1
    return files


def align(base_text: str, fp: FilePatch | None) -> list[Row]:
    """Diff view of one file: every base and patched line once, in order."""
    base = split_lines(base_text)
    rows: list[Row] = []
    b = n = 1

    def context_until(stop: int) -> None:
        nonlocal b, n
        if stop > len(base) + 1:
            raise ValueError(f"hunk starts past the end of the base file ({len(base)} lines)")
        while b < stop:
            rows.append(Row(b, n, base[b - 1], " "))
            b, n = b + 1, n + 1

    for hunk in fp.hunks if fp is not None else ():
        context_until(hunk.old_start + (hunk.old_count == 0))
        for line in hunk.lines:
            tag, text = line[:1], line[1:]
            if tag in (" ", "-") and (b > len(base) or base[b - 1] != text):
                raise ValueError(f"patch does not match base line {b}")
            if tag == " ":
                rows.append(Row(b, n, text, " "))
                b, n = b + 1, n + 1
            elif tag == "-":
                rows.append(Row(b, None, text, "-"))
                b += 1
            else:
                rows.append(Row(None, n, text, "+"))
                n += 1
    context_until(len(base) + 1)
    return rows


def file_patch(patch_text: str, path: str, base_path: str | None) -> FilePatch | None:
    """The patch's entry for this report file, or None if the patch doesn't touch it."""
    for fp in parse_patch(patch_text):
        if fp.new_path == path or (base_path is not None and fp.old_path == base_path):
            return fp
    return None


def is_changed(f: Mapping[str, Any]) -> bool:
    """A report file with a non-kept line or branch, a flaky line or a changed function."""
    return (
        any(e["category"] not in _KEPT_LINES for e in f["lines"])
        or any(e["category"] not in _KEPT_BRANCHES for e in f["branches"])
        or any(fn["status"] != "unchanged" for fn in f["functions"])
        or bool(f["flaky"])
    )


def fmt_count(value: int | None) -> str:
    """A hit or taken count; '–' for an absent side."""
    return "–" if value is None else str(value)


def fmt_pair(pair: Sequence[int | None]) -> str:
    """'base → patched'."""
    return f"{fmt_count(pair[0])} → {fmt_count(pair[1])}"


def fmt_pct(pct: float | None) -> str:
    """Coverage percent as the text report prints it."""
    return "n/a" if pct is None else f"{pct:.1f}%"


def _plural(n: int, singular: str, plural: str) -> str:
    return singular if n == 1 else plural


def _attrs(**attrs: object) -> str:
    """HTML attributes; keys use '_' for '-', None values are dropped."""
    return "".join(
        f' {k.replace("_", "-")}="{escape(str(v))}"' for k, v in attrs.items() if v is not None
    )


def render_headline(report: Report) -> str:
    """Chips for every non-zero summary count."""
    summary = report["summary"]
    chips: list[str] = []
    for section, key, singular, plural, verb in _HEADLINE:
        n = summary[section][key]
        if n:
            label = f"{n} {_plural(n, singular, plural)} {verb}".rstrip()
            chips.append(
                f'<span class="sg-chip"{_attrs(data_summary=f"{section}.{key}", data_n=n)}>'
                f"{escape(label)}</span>"
            )
    if not chips:
        chips.append('<span class="sg-chip">no coverage change</span>')
    return f'<div class="sg-headline">{"".join(chips)}</div>'


def render_meter(report: Report) -> str:
    """Base and patched line coverage bars from summary.coverage and the root tree stats."""
    stats = report["tree"]["stats"]
    cov = report["summary"]["coverage"]
    base_pct, patched_pct = cov["base_pct"], cov["patched_pct"]
    bars: list[str] = []
    for side, pct, hit, total in (
        ("base", base_pct, stats["hit_base"], stats["executable_base"]),
        ("patched", patched_pct, stats["hit_patched"], stats["executable_patched"]),
    ):
        fill = 0.0 if pct is None else pct
        delta = ""
        if side == "patched" and base_pct is not None and pct is not None and pct != base_pct:
            lo, hi = sorted((pct, base_pct))
            kind = "drop" if pct < base_pct else "rise"
            delta = f'<span class="sg-bar-{kind}" style="left:{lo}%;width:{hi - lo:g}%"></span>'
        bars.append(
            f'<div class="sg-meter-row"{_attrs(data_side=side, data_pct=pct)}>'
            f'<span class="sg-meter-side">{side}</span>'
            f'<span class="sg-bar"><span class="sg-bar-fill" style="width:{fill:g}%"></span>'
            f"{delta}</span>"
            f'<span class="sg-meter-pct">{fmt_pct(pct)}</span>'
            f'<span class="sg-meter-lines">{hit}/{total} lines</span>'
            "</div>"
        )
    return (
        '<div class="sg-panel sg-meter"><div class="sg-panel-title">Line coverage</div>'
        f"{''.join(bars)}</div>"
    )


def render_tests(report: Report) -> str:
    """Tests whose outcome differs between base and patched."""
    tests = report["tests"]
    body: list[str] = []
    if not tests:
        body.append('<p class="sg-note">No test outcomes in this report.</p>')
    else:
        changed = [t for t in tests if t["base"] != t["patched"]]
        for t in changed:
            body.append(
                f'<div class="sg-test"{_attrs(data_test=t["id"])}>'
                f'<code class="sg-test-id">{escape(t["id"])}</code>'
                f'<span class="sg-outcomes">{_outcome(t["base"])}'
                f'<span class="sg-arrow">→</span>{_outcome(t["patched"])}</span></div>'
            )
        same = len(tests) - len(changed)
        if not changed:
            body.append(f'<p class="sg-note">No test changed outcome ({same} unchanged).</p>')
        elif same:
            body.append(
                f'<p class="sg-note" data-unchanged="{same}">'
                f"{same} other {_plural(same, 'test', 'tests')} unchanged.</p>"
            )
    return (
        '<div class="sg-panel sg-tests"><div class="sg-panel-title">Test outcomes</div>'
        f"{''.join(body)}</div>"
    )


def _outcome(outcome: str) -> str:
    return f'<span class="sg-outcome sg-o-{escape(outcome)}">{escape(outcome)}</span>'


def _lanes(spans: Iterable[tuple[int, int]]) -> list[int]:
    """Lane per span (by start order) so overlapping spans don't share a lane."""
    ends: list[int] = []  # last row of the span currently occupying each lane
    lanes: list[int] = []
    for start, end in spans:
        lane = next((i for i, last in enumerate(ends) if last < start), len(ends))
        if lane == len(ends):
            ends.append(end)
        else:
            ends[lane] = end
        lanes.append(lane)
    return lanes


def render_file(f: Mapping[str, Any], rows: Sequence[Row]) -> tuple[str, set[str]]:
    """Annotated listing of one report file; also returns the legend keys it uses."""
    used: set[str] = set()
    path = f["path"]
    by_new = {r.new: i for i, r in enumerate(rows) if r.new is not None}
    by_old = {r.base: i for i, r in enumerate(rows) if r.base is not None}
    deleted_rows = {r.base: i for i, r in enumerate(rows) if r.new is None}

    def row_of(new: int | None, base: int | None, what: str) -> int:
        index = by_new.get(new) if new is not None else deleted_rows.get(base)
        if index is None:
            raise ValueError(f"{path}: {what} at line {new}/base {base} is not in the source")
        return index

    line_at: dict[int, dict[str, Any]] = {}
    entries = [{**e} for e in f["lines"]]
    entries += [{**e, "category": "flaky"} for e in f["flaky"]]
    for e in entries:
        i = row_of(e["line"], e["base_line"], "line entry")
        if i in line_at:
            raise ValueError(f"{path}: two line entries on row {i + 1}")
        line_at[i] = e
    branches_at: dict[int, list[Mapping[str, Any]]] = {}
    for br in f["branches"]:
        branches_at.setdefault(row_of(br["line"], br["base_line"], "branch"), []).append(br)

    functions: list[tuple[int, int, Mapping[str, Any]]] = []
    for fn in f["functions"]:
        index = by_old if fn["status"] == "removed" else by_new
        if fn["start"] not in index or fn["end"] not in index:
            raise ValueError(f"{path}: function {fn['name']} span is not in the source")
        functions.append((index[fn["start"]], index[fn["end"]], fn))
    functions.sort(key=lambda t: (t[0], -t[1]))
    lanes = _lanes((s, e) for s, e, _ in functions)
    starting: dict[int, list[int]] = {}
    for k, (s, _, _) in enumerate(functions):
        starting.setdefault(s, []).append(k)

    has_branches = bool(f["branches"])
    used |= {f"fn:{fn['status']}" for _, _, fn in functions}
    out: list[str] = []
    display: list[int] = []  # display row of each source row; function headers add rows
    head: dict[int, int] = {}  # display row of each function's header
    for i, row in enumerate(rows):
        for k in starting.get(i, []):
            head[k] = len(out)
            out.append(f'<div class="sg-row sg-fnrow">{_badge(functions[k][2])}</div>')
        display.append(len(out))
        out.append(_line_row(row, line_at.get(i), branches_at.get(i, []), has_branches, used))
    bars = [
        f'<span class="sg-fnbar sg-f-{fn["status"]}" style="--sg-i:{head[k]};'
        f'--sg-n:{display[e] - head[k] + 1};--sg-lane:{lane}"></span>'
        for k, ((_, e, fn), lane) in enumerate(zip(functions, lanes, strict=True))
    ]
    style = f"--sg-lanes:{max(lanes, default=-1) + 1}"
    grid = "sg-lines sg-has-branches" if has_branches else "sg-lines"
    note = "" if has_branches else '<span class="sg-note">no branch data in this report</span>'
    html = (
        f'<figure class="sg-file"{_attrs(data_path=path)}>'
        f"<figcaption><code>{escape(path)}</code>{note}</figcaption>"
        f'<div class="sg-scroll"><div class="{grid}" style="{style}">'
        f"{''.join(bars)}{''.join(out)}</div></div></figure>"
    )
    return html, used


def _line_row(
    row: Row,
    e: Mapping[str, Any] | None,
    branches: Sequence[Mapping[str, Any]],
    has_branches: bool,
    used: set[str],
) -> str:
    """One source line: patch mark, number, hits, category, branch markers, code."""
    cat = e["category"] if e is not None else None
    classes = ["sg-row", _MARK_CLASS[row.mark]]
    if cat is not None:
        classes.append(f"sg-k-{cat}")
        used.add(f"line:{cat}")
    if row.mark != " ":
        used.add(f"mark:{row.mark}")
    number = row.new if row.new is not None else row.base
    cells = [
        f'<span class="sg-mark">{"−" if row.mark == "-" else row.mark.strip()}</span>',
        f'<span class="sg-ln">{number}</span>',
        f'<span class="sg-hits">{fmt_pair(e["hits"]) if e is not None else ""}</span>',
        '<span class="sg-cat">'
        f"{LINE_LABELS[cat] if cat is not None and cat not in _KEPT_LINES else ''}</span>",
    ]
    if has_branches:
        marks: list[str] = []
        for br in branches:
            used.add(f"branch:{br['category']}")
            label = BRANCH_LABELS[br["category"]]
            tip = f"{br['branch']}: {label}, taken {fmt_pair(br['taken'])}"
            taken = ",".join(map(fmt_count, br["taken"]))
            marks.append(
                f'<span class="sg-br sg-b-{br["category"]}"{_attrs(title=tip, data_taken=taken)}>'
                "</span>"
            )
        cells.append(f'<span class="sg-brs">{"".join(marks)}</span>')
    cells.append(
        f'<span class="sg-code"><span class="sg-src">{escape(row.text.rstrip(chr(13)))}</span>'
        f"{_branch_note(branches)}</span>"
    )
    hits = None if e is None else ",".join(map(fmt_count, e["hits"]))
    return (
        f'<div class="{" ".join(classes)}"'
        f"{_attrs(data_line=row.new, data_base_line=row.base, data_cat=cat, data_hits=hits)}>"
        f"{''.join(cells)}</div>"
    )


def _branch_note(branches: Sequence[Mapping[str, Any]]) -> str:
    """'2 branches lost' for the non-kept branches on a line."""
    counts: dict[str, int] = {}
    for br in branches:
        if br["category"] not in _KEPT_BRANCHES:
            counts[br["category"]] = counts.get(br["category"], 0) + 1
    parts = [
        f"{n} {_plural(n, 'branch', 'branches')} {BRANCH_LABELS[cat]}" for cat, n in counts.items()
    ]
    return f'<span class="sg-brnote">{escape(", ".join(parts))}</span>' if parts else ""


def _badge(fn: Mapping[str, Any]) -> str:
    lines = fn["lines"]
    parts = [f"lines {fn['start']}–{fn['end']}"]
    if lines["lost"]:
        parts.append(f"lost {lines['lost']} of {lines['executable']}")
    if lines["gained"]:
        parts.append(f"gained {lines['gained']} of {lines['executable']}")
    if not lines["lost"] and not lines["gained"]:
        parts.append(f"{lines['executable']} body {_plural(lines['executable'], 'line', 'lines')}")
    return (
        f'<span class="sg-fn sg-f-{fn["status"]}"'
        f"{_attrs(data_fn=fn['name'], data_status=fn['status'])}>"
        f'<span class="sg-fn-name">{escape(fn["name"])}</span>'
        f'<span class="sg-fn-status">{FUNCTION_LABELS[fn["status"]]}</span>'
        f'<span class="sg-fn-span">{escape(" · ".join(parts))}</span></span>'
    )


def render_legend(used: set[str]) -> str:
    """Legend for the markers that appear in the graphic, plus how to read the counts."""
    keys = [
        *(
            (f"line:{k}", f'<span class="sg-sw sg-k-{k}"></span>{v}')
            for k, v in LINE_LABELS.items()
        ),
        *(
            (f"branch:{k}", f'<span class="sg-br sg-b-{k}"></span>branch {v}')
            for k, v in BRANCH_LABELS.items()
        ),
        *(
            (f"fn:{k}", f'<span class="sg-fnsw sg-f-{k}"></span>function {v}')
            for k, v in FUNCTION_LABELS.items()
        ),
        ("mark:+", '<span class="sg-mk">+</span>line added by the patch'),
        ("mark:-", '<span class="sg-mk">−</span>line deleted by the patch'),
    ]
    items = [f'<span class="sg-li">{item}</span>' for key, item in keys if key in used]
    return (
        '<div class="sg-legend">'
        f"{''.join(items)}"
        '<span class="sg-li sg-li-read">counts: hits base → patched, – = absent</span></div>'
    )


def render(report: Report, base_sources: Mapping[str, str], patch_text: str) -> str:
    """The whole graphic: headline, coverage meter, test outcomes, annotated files, legend."""
    files: list[str] = []
    used: set[str] = set()
    for f in report["files"]:
        if not is_changed(f):
            continue
        base_path = f["base_path"]
        base_text = "" if base_path is None else base_sources[base_path]
        rows = align(base_text, file_patch(patch_text, f["path"], base_path))
        html, keys = render_file(f, rows)
        files.append(html)
        used |= keys
    if not files:
        files.append('<p class="sg-note">No file changed coverage.</p>')
    return (
        '<div class="sg">'
        f"{render_headline(report)}"
        f'<div class="sg-panels">{render_meter(report)}{render_tests(report)}</div>'
        f"{''.join(files)}"
        f"{render_legend(used)}"
        "</div>"
    )


def changed_paths(report: Report) -> list[str]:
    """Base paths of the files the graphic shows (what the caller must read)."""
    return [f["base_path"] for f in report["files"] if is_changed(f) and f["base_path"] is not None]
