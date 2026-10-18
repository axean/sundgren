"""core.diff: line/branch classification, inference, flakiness, pairing, outcomes."""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from pathlib import Path

import pytest

from sundgren.core.diff import Delta, FileDelta, diff
from sundgren.core.hashing import sha256_file
from sundgren.core.ingest.lcov import parse
from sundgren.core.junit import parse_junit
from sundgren.core.linemap import LineMap, build_linemaps
from sundgren.core.model import BranchKey, FileCoverage, FunctionRecord, Snapshot
from sundgren.core.patch import parse_patch
from support import FIXTURES

P = "pkg/a.py"


def fc(
    path: str = P,
    lines: Mapping[int, int] | None = None,
    branches: Mapping[tuple[int, str, str], int | None] | None = None,
    functions: Iterable[FunctionRecord] = (),
    external: bool = False,
) -> FileCoverage:
    return FileCoverage(
        path=path,
        lines=dict(lines or {}),
        branches={BranchKey(ln, blk, br): t for (ln, blk, br), t in (branches or {}).items()},
        functions=tuple(functions),
        external=external,
    )


def snap(*files: FileCoverage) -> Snapshot:
    return Snapshot(files={f.path: f for f in files}, test_names=())


def lmap(
    old: str | None = P,
    new: str | None = P,
    deleted: Iterable[int] = (),
    added: Iterable[int] = (),
) -> LineMap:
    return LineMap(old, new, {}, frozenset(deleted), frozenset(added))


def keyed(*maps: LineMap) -> dict[str, LineMap]:
    out: dict[str, LineMap] = {}
    for m in maps:
        for p in (m.old_path, m.new_path):
            if p is not None:
                out[p] = m
    return out


def one(delta: Delta, path: str = P) -> FileDelta:
    [f] = [f for f in delta.files if f.path == path]
    return f


def cats(f: FileDelta) -> dict[tuple[int | None, int | None], tuple[str, tuple[int | None, ...]]]:
    """(line, base_line) -> (category, hits)."""
    return {(e.line, e.base_line): (e.category, e.hits) for e in f.lines}


def branch_cats(
    f: FileDelta,
) -> list[tuple[int | None, int | None, str, str, tuple[int | None, ...]]]:
    return [(b.line, b.base_line, b.branch, b.category, b.taken) for b in f.branches]


def codes(delta: Delta) -> list[str]:
    return [w.code for w in delta.warnings]


# --- classification table, one test per row ----------------------------------


def test_kept_hit_records_both_counts() -> None:
    f = one(diff([snap(fc(lines={1: 2}))], [snap(fc(lines={1: 5}))], {}))
    assert cats(f) == {(1, 1): ("kept_hit", (2, 5))}
    assert not f.changed


def test_kept_miss() -> None:
    f = one(diff([snap(fc(lines={1: 0}))], [snap(fc(lines={1: 0}))], {}))
    assert cats(f) == {(1, 1): ("kept_miss", (0, 0))}


def test_lost() -> None:
    f = one(diff([snap(fc(lines={1: 3}))], [snap(fc(lines={1: 0}))], {}))
    assert cats(f) == {(1, 1): ("lost", (3, 0))}
    assert f.stats.lost == 1 and f.changed


def test_gained() -> None:
    f = one(diff([snap(fc(lines={1: 0}))], [snap(fc(lines={1: 2}))], {}))
    assert cats(f) == {(1, 1): ("gained", (0, 2))}
    assert f.stats.gained == 1


def test_absent_on_one_side_is_unmeasured_with_warning() -> None:
    delta = diff([snap(fc(lines={1: 1, 2: 1}))], [snap(fc(lines={1: 1, 3: 0}))], {})
    assert cats(one(delta)) == {
        (1, 1): ("kept_hit", (1, 1)),
        (2, 2): ("unmeasured", (1, None)),
        (3, 3): ("unmeasured", (None, 0)),
    }
    [w] = delta.warnings
    assert w.code == "unmeasured" and w.message.startswith(f"{P}: 2 line(s)")


def test_added_lines_are_new_hit_and_new_miss() -> None:
    # Lines 2-3 inserted; base line 2 moves to 4.
    delta = diff(
        [snap(fc(lines={1: 1, 2: 1}))],
        [snap(fc(lines={1: 1, 2: 1, 3: 0, 4: 1}))],
        keyed(lmap(added=[2, 3])),
    )
    f = one(delta)
    assert cats(f) == {
        (1, 1): ("kept_hit", (1, 1)),
        (2, None): ("new_hit", (None, 1)),
        (3, None): ("new_miss", (None, 0)),
        (4, 2): ("kept_hit", (1, 1)),
    }
    assert (f.stats.new_hit, f.stats.new_miss) == (1, 1)


def test_deleted_lines_are_deleted_hit_and_deleted_miss() -> None:
    delta = diff(
        [snap(fc(lines={1: 1, 2: 1, 3: 0, 4: 0}))],
        [snap(fc(lines={1: 1, 2: 0}))],
        keyed(lmap(deleted=[2, 3])),
    )
    f = one(delta)
    assert cats(f) == {
        (1, 1): ("kept_hit", (1, 1)),
        (2, 4): ("kept_miss", (0, 0)),
        (None, 2): ("deleted_hit", (1, None)),
        (None, 3): ("deleted_miss", (0, None)),
    }
    # Deleted lines (line None) sort after every patched line, by base line.
    assert [(e.line, e.base_line) for e in f.lines] == [(1, 1), (2, 4), (None, 2), (None, 3)]


def test_changed_line_is_deleted_plus_added() -> None:
    delta = diff(
        [snap(fc(lines={1: 1, 2: 1}))],
        [snap(fc(lines={1: 1, 2: 0}))],
        keyed(lmap(deleted=[2], added=[2])),
    )
    assert cats(one(delta)) == {
        (1, 1): ("kept_hit", (1, 1)),
        (2, None): ("new_miss", (None, 0)),
        (None, 2): ("deleted_hit", (1, None)),
    }


def test_shifted_lines_are_not_lost_or_gained() -> None:
    delta = diff(
        [snap(fc(lines={1: 1, 2: 0}))],
        [snap(fc(lines={3: 1, 4: 0}))],
        keyed(lmap(added=[1, 2])),
    )
    assert cats(one(delta)) == {(3, 1): ("kept_hit", (1, 1)), (4, 2): ("kept_miss", (0, 0))}


# --- branches ----------------------------------------------------------------


def test_branch_lost_gained_kept() -> None:
    base = fc(lines={1: 1}, branches={(1, "0", "0"): 2, (1, "0", "1"): 0, (1, "0", "2"): 1})
    patched = fc(lines={1: 1}, branches={(1, "0", "0"): 0, (1, "0", "1"): 4, (1, "0", "2"): 1})
    f = one(diff([snap(base)], [snap(patched)], {}))
    assert branch_cats(f) == [
        (1, 1, "0", "branch_lost", (2, 0)),
        (1, 1, "1", "branch_gained", (0, 4)),
        (1, 1, "2", "branch_kept_hit", (1, 1)),
    ]


def test_branch_none_taken() -> None:
    # '-' (None) counts as not taken in either direction.
    base = fc(lines={1: 1, 2: 1}, branches={(1, "0", "0"): 1, (2, "0", "0"): None})
    patched = fc(lines={1: 1, 2: 1}, branches={(1, "0", "0"): None, (2, "0", "0"): None})
    f = one(diff([snap(base)], [snap(patched)], {}))
    assert branch_cats(f) == [
        (1, 1, "0", "branch_lost", (1, None)),
        (2, 2, "0", "branch_kept_miss", (None, None)),
    ]


def test_branch_on_added_and_deleted_lines() -> None:
    base = fc(lines={1: 1, 2: 1}, branches={(2, "0", "0"): 1})
    patched = fc(lines={1: 1, 2: 1}, branches={(2, "0", "0"): 0})
    f = one(diff([snap(base)], [snap(patched)], keyed(lmap(deleted=[2], added=[2]))))
    assert branch_cats(f) == [
        (2, None, "0", "branch_new", (None, 0)),
        (None, 2, "0", "branch_deleted", (1, None)),
    ]


def test_coverage_py_branch_ids_follow_shifted_targets() -> None:
    # touch_source: two lines inserted on top; coverage.py names arcs by target line.
    base = fc(
        lines={6: 1, 7: 1, 8: 1},
        branches={(6, "0", "jump to line 7"): 1, (6, "0", "jump to line 8"): 1},
    )
    patched = fc(
        lines={8: 1, 9: 1, 10: 1},
        branches={(8, "0", "jump to line 9"): 1, (8, "0", "jump to line 10"): 1},
    )
    f = one(diff([snap(base)], [snap(patched)], keyed(lmap(added=[1, 2]))))
    assert branch_cats(f) == [
        (8, 6, "jump to line 9", "branch_kept_hit", (1, 1)),
        (8, 6, "jump to line 10", "branch_kept_hit", (1, 1)),
    ]
    assert not f.changed


def test_branch_to_changed_target_is_deleted_plus_new() -> None:
    # Line 6 unchanged, its jump target 7 rewritten: that arc has no counterpart.
    ids = {"jump to line 7", "jump to line 8", "return from function 'f'"}
    base = fc(lines={6: 1, 7: 1, 8: 1}, branches={(6, "0", i): 1 for i in ids})
    patched = fc(lines={6: 1, 7: 0, 8: 1}, branches={(6, "0", i): 1 for i in ids})
    f = one(diff([snap(base)], [snap(patched)], keyed(lmap(deleted=[7], added=[7]))))
    assert branch_cats(f) == [
        (6, 6, "jump to line 7", "branch_deleted", (1, None)),
        (6, 6, "jump to line 7", "branch_new", (None, 1)),
        (6, 6, "jump to line 8", "branch_kept_hit", (1, 1)),
        (6, 6, "return from function 'f'", "branch_kept_hit", (1, 1)),
    ]


def test_branch_without_counterpart_line_data_is_unmeasured() -> None:
    base = fc(lines={1: 1}, branches={(1, "0", "0"): 1})
    patched = fc(lines={1: 1})  # e.g. run without branch coverage
    delta = diff([snap(base)], [snap(patched)], {})
    assert branch_cats(one(delta)) == [(1, 1, "0", "branch_unmeasured", (1, None))]
    assert codes(delta) == ["unmeasured"]


# --- files measured on one side only ----------------------------------------


def _tree(root: Path, files: Mapping[str, str]) -> Path:
    for rel, text in files.items():
        (root / rel).parent.mkdir(parents=True, exist_ok=True)
        (root / rel).write_text(text)
    return root


def test_inferred_base_when_file_exists_in_base_tree(tmp_path: Path) -> None:
    base_root = _tree(tmp_path / "b", {P: "x\n"})
    patched = fc(lines={1: 1, 2: 0}, branches={(1, "0", "0"): 1})
    delta = diff([snap()], [snap(patched)], {}, base_root=base_root)
    f = one(delta)
    assert f.inferred_side == "base" and f.base_path == P
    assert cats(f) == {(1, 1): ("gained", (0, 1)), (2, 2): ("kept_miss", (0, 0))}
    assert branch_cats(f) == [(1, 1, "0", "branch_gained", (0, 1))]
    assert codes(delta) == ["inferred_file"]
    assert (f.stats.executable_base, f.stats.hit_base) == (2, 0)


def test_inferred_patched_when_file_exists_in_patched_tree(tmp_path: Path) -> None:
    patched_root = _tree(tmp_path / "p", {P: "x\n"})
    base = fc(lines={1: 1, 2: 0})
    delta = diff([snap(base)], [snap()], {}, patched_root=patched_root)
    f = one(delta)
    assert f.inferred_side == "patched"
    assert cats(f) == {(1, 1): ("lost", (1, 0)), (2, 2): ("kept_miss", (0, 0))}


def test_nothing_executable_means_nothing_inferred(tmp_path: Path) -> None:
    init = "pkg/__init__.py"
    root = _tree(tmp_path / "b", {init: ""})
    delta = diff([snap()], [snap(fc(init))], {}, base_root=root)
    f = one(delta, init)
    assert (f.inferred_side, f.base_path, f.lines) == (None, init, ())
    assert delta.warnings == ()


def test_inferred_side_maps_lines_through_the_patch(tmp_path: Path) -> None:
    patched_root = _tree(tmp_path / "p", {P: "x\n"})
    base = fc(lines={1: 1, 2: 1}, branches={(1, "0", "jump to line 2"): 1})
    delta = diff([snap(base)], [snap()], keyed(lmap(added=[1])), patched_root=patched_root)
    f = one(delta)
    assert cats(f) == {(2, 1): ("lost", (1, 0)), (3, 2): ("lost", (1, 0))}
    assert branch_cats(f) == [(2, 1, "jump to line 3", "branch_lost", (1, 0))]


def test_inferred_side_has_no_counterpart_for_branches_to_new_lines(tmp_path: Path) -> None:
    base_root = _tree(tmp_path / "b", {P: "x\n"})
    ids = {(1, "0", "jump to line 2"): 1, (1, "0", "jump to line 3"): 0}
    patched = fc(lines={1: 1, 2: 1, 3: 0}, branches=ids)
    delta = diff([snap()], [snap(patched)], keyed(lmap(added=[2, 3])), base_root=base_root)
    assert [b[3] for b in branch_cats(one(delta))] == ["branch_new", "branch_new"]
    assert codes(delta) == ["inferred_file"]


def test_missing_from_tree_means_new_file(tmp_path: Path) -> None:
    base_root = _tree(tmp_path / "b", {})
    delta = diff([snap()], [snap(fc(lines={1: 1, 2: 0}))], {}, base_root=base_root)
    f = one(delta)
    assert f.base_path is None and f.inferred_side is None
    assert cats(f) == {(1, None): ("new_hit", (None, 1)), (2, None): ("new_miss", (None, 0))}
    assert delta.warnings == ()


def test_missing_from_tree_means_deleted_file(tmp_path: Path) -> None:
    patched_root = _tree(tmp_path / "p", {})
    delta = diff([snap(fc(lines={1: 1}))], [snap()], {}, patched_root=patched_root)
    f = one(delta)
    assert f.path == P and f.base_path == P
    assert cats(f) == {(None, 1): ("deleted_hit", (1, None))}
    assert f.source_changed


def test_root_unknown_uses_patch_new_file() -> None:
    delta = diff([snap()], [snap(fc(lines={1: 0}))], keyed(lmap(old=None)))
    f = one(delta)
    assert f.base_path is None
    assert cats(f) == {(1, None): ("new_miss", (None, 0))}
    assert delta.warnings == ()


def test_root_unknown_uses_patch_deleted_file() -> None:
    delta = diff([snap(fc(lines={1: 1}))], [snap()], keyed(lmap(new=None)))
    assert cats(one(delta)) == {(None, 1): ("deleted_hit", (1, None))}
    assert delta.warnings == ()


def test_root_unknown_without_patch_info_is_unmeasured() -> None:
    delta = diff([snap()], [snap(fc(lines={1: 1, 2: 0}))], {})
    f = one(delta)
    assert f.inferred_side is None
    assert {c for c, _ in cats(f).values()} == {"unmeasured"}
    [w] = delta.warnings
    assert w.code == "unmeasured"
    assert "not in base coverage and the base tree is unknown" in w.message


def test_unmeasured_file_with_nothing_executable_is_quiet() -> None:
    delta = diff([snap(fc("pkg/__init__.py"))], [snap()], {})
    assert one(delta, "pkg/__init__.py").lines == ()
    assert delta.warnings == ()


# --- flakiness ---------------------------------------------------------------


def test_flaky_base_line_is_excluded_from_lost() -> None:
    base = [snap(fc(lines={1: 1, 2: 1})), snap(fc(lines={1: 1, 2: 0}))]
    patched = [snap(fc(lines={1: 0, 2: 0}))]
    f = one(diff(base, patched, {}))
    assert cats(f) == {(1, 1): ("lost", (1, 0))}
    assert [(e.line, e.base_line, e.hits, e.sides) for e in f.flaky] == [(2, 2, (1, 0), ("base",))]
    assert f.stats.lost == 1 and f.stats.hit_base == 2  # counts from run 0


def test_flaky_patched_line_and_missing_file_in_a_repeat() -> None:
    base = [snap(fc(lines={1: 0}))]
    patched = [snap(fc(lines={1: 1})), snap()]
    f = one(diff(base, patched, {}))
    assert cats(f) == {}
    assert [(e.line, e.sides) for e in f.flaky] == [(1, ("patched",))]
    assert f.changed


def test_flaky_branch() -> None:
    base = [
        snap(fc(lines={1: 1}, branches={(1, "0", "0"): 1, (1, "0", "1"): 1})),
        snap(fc(lines={1: 1}, branches={(1, "0", "0"): 0, (1, "0", "1"): 1})),
    ]
    patched = [snap(fc(lines={1: 1}, branches={(1, "0", "0"): 0, (1, "0", "1"): 0}))]
    f = one(diff(base, patched, {}))
    assert [(b.branch, b.category) for b in f.branches] == [
        ("0", "branch_flaky"),
        ("1", "branch_lost"),
    ]


# --- pairing, renames, external files ---------------------------------------


def test_rename_pairs_old_and_new_path() -> None:
    rename = lmap(old="pkg/old.py", new="pkg/new.py")
    delta = diff(
        [snap(fc("pkg/old.py", {1: 1, 2: 1}))],
        [snap(fc("pkg/new.py", {1: 1, 2: 0}))],
        keyed(rename),
    )
    [f] = delta.files
    assert (f.path, f.base_path) == ("pkg/new.py", "pkg/old.py")
    assert cats(f) == {(1, 1): ("kept_hit", (1, 1)), (2, 2): ("lost", (1, 0))}
    assert not f.source_changed  # pure rename, roots unknown


def test_rename_with_edits() -> None:
    rename = lmap(old="pkg/old.py", new="pkg/new.py", deleted=[2], added=[2, 3])
    delta = diff(
        [snap(fc("pkg/old.py", {1: 1, 2: 1, 3: 0}))],
        [snap(fc("pkg/new.py", {1: 1, 2: 0, 3: 1, 4: 1}))],
        keyed(rename),
    )
    [f] = delta.files
    assert cats(f) == {
        (1, 1): ("kept_hit", (1, 1)),
        (2, None): ("new_miss", (None, 0)),
        (3, None): ("new_hit", (None, 1)),
        (4, 3): ("gained", (0, 1)),
        (None, 2): ("deleted_hit", (1, None)),
    }
    assert f.source_changed


def test_rename_with_only_base_measured(tmp_path: Path) -> None:
    patched_root = _tree(tmp_path / "p", {"pkg/new.py": "x\n"})
    rename = lmap(old="pkg/old.py", new="pkg/new.py")
    delta = diff(
        [snap(fc("pkg/old.py", {1: 1}))], [snap()], keyed(rename), patched_root=patched_root
    )
    [f] = delta.files
    assert (f.path, f.base_path, f.inferred_side) == ("pkg/new.py", "pkg/old.py", "patched")
    assert cats(f) == {(1, 1): ("lost", (1, 0))}


def test_patch_says_deleted_but_snapshot_has_it() -> None:
    delta = diff([snap(fc(lines={1: 1}))], [snap(fc(lines={1: 0}))], keyed(lmap(new=None)))
    assert cats(one(delta)) == {(1, 1): ("lost", (1, 0))}


def test_colliding_base_file_is_ignored_with_warning() -> None:
    rename = lmap(old="pkg/old.py", new="pkg/new.py")
    delta = diff(
        [snap(fc("pkg/old.py", {1: 1}), fc("pkg/new.py", {1: 1}))],
        [snap(fc("pkg/new.py", {1: 1}))],
        keyed(rename),
    )
    assert [(f.path, f.base_path) for f in delta.files] == [("pkg/new.py", "pkg/old.py")]
    assert codes(delta) == ["unpaired_file"]


def test_external_files_are_counted_not_listed() -> None:
    ext = fc("/usr/lib/python3/os.py", {1: 1}, external=True)
    delta = diff([snap(ext, fc(lines={1: 1}))], [snap(ext, fc(lines={1: 1}))], {})
    assert delta.external_files == 1
    assert [f.path for f in delta.files] == [P]
    assert [c.name for c in delta.tree.children] == ["pkg"]


def test_files_sorted_and_tree_covers_unchanged_files() -> None:
    files = [fc("b.py", {1: 1}), fc("a/z.py", {1: 1}), fc("a/c.py", {1: 1})]
    delta = diff([snap(*files)], [snap(*files)], {})
    assert [f.path for f in delta.files] == ["a/c.py", "a/z.py", "b.py"]
    assert [c.name for c in delta.tree.children] == ["a", "b.py"]
    assert delta.tree.stats.executable_base == 3


def test_needs_a_snapshot_per_side() -> None:
    with pytest.raises(ValueError):
        diff([], [snap()], {})


# --- sha256 and source_changed ----------------------------------------------


def test_sha256_from_roots(tmp_path: Path) -> None:
    base_root = _tree(tmp_path / "b", {P: "same\n"})
    patched_root = _tree(tmp_path / "p", {P: "same\n"})
    d = diff(
        [snap(fc(lines={1: 1}))],
        [snap(fc(lines={1: 1}))],
        {},
        base_root=base_root,
        patched_root=patched_root,
    )
    f = one(d)
    digest = sha256_file(base_root / P)
    assert f.sha256 == (digest, digest) and not f.source_changed
    (patched_root / P).write_text("changed\n")
    d = diff(
        [snap(fc(lines={1: 1}))],
        [snap(fc(lines={1: 1}))],
        {},
        base_root=base_root,
        patched_root=patched_root,
    )
    assert one(d).source_changed


def test_source_changed_from_linemap_when_roots_unknown() -> None:
    d = diff([snap(fc(lines={1: 1}))], [snap(fc(lines={2: 1}))], keyed(lmap(added=[1])))
    f = one(d)
    assert f.sha256 == (None, None) and f.source_changed


# --- test outcomes -----------------------------------------------------------


def test_outcomes_fill_not_run() -> None:
    d = diff(
        [snap()],
        [snap()],
        {},
        base_outcomes={"t::a": "passed", "t::b": "passed"},
        patched_outcomes={"t::a": "skipped", "t::c": "failed"},
    )
    assert [(t.id, t.base, t.patched) for t in d.tests] == [
        ("t::a", "passed", "skipped"),
        ("t::b", "passed", "not_run"),
        ("t::c", "not_run", "failed"),
    ]


def test_outcomes_missing() -> None:
    assert diff([snap()], [snap()], {}).tests == ()
    d = diff([snap()], [snap()], {}, base_outcomes={"t": "passed"})
    assert d.tests == () and codes(d) == ["tests_one_side"]


# --- real coverage.py output -------------------------------------------------

RUNS = FIXTURES / "shed_py_runs"
SHED = FIXTURES / "shed_py"
CALC = "shed/calc.py"


def test_real_skip_divide(monkeypatch: pytest.MonkeyPatch) -> None:
    from sundgren.core import symbols

    monkeypatch.delitem(symbols._REGISTRY, ".py", raising=False)  # spans from lcov FN
    run = RUNS / "skip_divide"
    patch = parse_patch((FIXTURES / "patches" / "skip_divide.patch").read_text())
    d = diff(
        [parse((run / "base.lcov").read_text())],
        [parse((run / "patched.lcov").read_text())],
        build_linemaps(patch),
        base_root=SHED,
        patched_root=SHED,
        base_outcomes=parse_junit((run / "base.xml").read_text()),
        patched_outcomes=parse_junit((run / "patched.xml").read_text()),
    )
    assert [(t.id, t.base, t.patched) for t in d.tests if t.base != t.patched] == [
        ("tests.test_calc::test_divide", "passed", "skipped")
    ]
    assert [f.path for f in d.files if f.changed] == [CALC]
    f = one(d, CALC)
    assert not f.source_changed and f.sha256[0] == sha256_file(SHED / CALC)
    changed = {k: v for k, v in cats(f).items() if not v[0].startswith("kept_")}
    assert changed == {(ln, ln): ("lost", (1, 0)) for ln in (6, 7, 8)}
    assert [b for b in branch_cats(f) if b[3] != "branch_kept_hit"] == [
        (6, 6, "jump to line 7", "branch_lost", (1, None)),
        (6, 6, "jump to line 8", "branch_lost", (1, None)),
    ]
    funcs = [(fn.name, fn.status, fn.start, fn.end, fn.lost) for fn in f.functions]
    assert funcs == [
        ("add", "unchanged", 2, 3, 0),
        ("divide", "went_dark", 5, 8, 3),
        ("classify", "unchanged", 10, 15, 0),
    ]
    assert d.warnings == ()
