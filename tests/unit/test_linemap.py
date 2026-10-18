from __future__ import annotations

import difflib
import random
from pathlib import Path

import pytest

from sundgren.core.linemap import LineMap, build_linemaps, identity, linemap_for
from sundgren.core.patch import FilePatch, Patch, parse_patch

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures" / "patches" / "unit"


def maps(name: str) -> dict[str, LineMap]:
    return build_linemaps(parse_patch((FIXTURES / f"{name}.patch").read_text(encoding="utf-8")))


@pytest.mark.parametrize(
    ("name", "path", "deleted", "added"),
    [
        ("single_hunk", "shed/calc.py", {10}, {10, 13}),
        ("multi_hunk", "shed/calc.py", {3, 15}, {3, 27, 28}),
        ("pure_add", "shed/calc.py", set(), {6, 7}),
        ("pure_delete", "shed/calc.py", {8, 9}, set()),
        ("new_file", "shed/util.py", set(), {1, 2, 3}),
        ("deleted_file", "shed/old.py", {1, 2, 3, 4, 5}, set()),
        ("rename_with_edits", "shed/core.py", {10}, {10}),
        ("no_newline_eof", "shed/tail.py", {3}, {3, 4}),
        ("no_newline_eof", "shed/nonl.py", {2}, {2}),
        ("format_patch", "shed/calc.py", {5}, {5}),
        ("plain_diff", "calc.py", {20}, {20}),
    ],
)
def test_changed_lines(name: str, path: str, deleted: set[int], added: set[int]) -> None:
    lm = maps(name)[path]
    assert (lm.deleted_old, lm.added_new) == (deleted, added)


@pytest.mark.parametrize(
    ("name", "to_new", "to_old"),
    [
        (
            "single_hunk",
            [*range(1, 10), None, 11, 12, *range(14, 32)],
            [*range(1, 10), None, 11, 12, None, *range(13, 31)],
        ),
        (
            "multi_hunk",
            [1, 2, None, *range(4, 15), None, *range(15, 27), *range(29, 32)],
            [1, 2, None, *range(4, 15), *range(16, 28), None, None, 28, 29, 30],
        ),
        ("pure_add", [*range(1, 6), *range(8, 33)], [*range(1, 6), None, None, *range(6, 31)]),
        ("pure_delete", [*range(1, 8), None, None, *range(8, 29)], [*range(1, 8), *range(10, 31)]),
    ],
)
def test_every_line_of_calc(name: str, to_new: list[int | None], to_old: list[int | None]) -> None:
    lm = maps(name)["shed/calc.py"]
    assert [lm.to_new(n) for n in range(1, len(to_new) + 1)] == to_new
    assert [lm.to_old(n) for n in range(1, len(to_old) + 1)] == to_old


def test_new_file() -> None:
    lm = maps("new_file")["shed/util.py"]
    assert (lm.old_path, lm.new_path) == (None, "shed/util.py")
    assert [lm.to_old(n) for n in (1, 2, 3)] == [None, None, None]


def test_deleted_file() -> None:
    lm = maps("deleted_file")["shed/old.py"]
    assert (lm.old_path, lm.new_path) == ("shed/old.py", None)
    assert [lm.to_new(n) for n in range(1, 6)] == [None] * 5


def test_rename_keyed_by_both_paths() -> None:
    m = maps("rename_with_edits")
    assert m["shed/calc.py"] is m["shed/core.py"]
    lm = m["shed/core.py"]
    assert (lm.old_path, lm.new_path) == ("shed/calc.py", "shed/core.py")
    assert (lm.to_new(9), lm.to_new(10), lm.to_new(30), lm.to_old(10)) == (9, None, 30, None)


def test_pure_rename() -> None:
    m = maps("pure_rename")
    assert list(m) == ["shed/legacy.py", "shed/old.py"]
    lm = m["shed/old.py"]
    assert not lm.is_identity
    assert (lm.to_new(7), lm.to_old(7)) == (7, 7)


def test_binary_and_mode_only_maps_are_identity_shaped() -> None:
    assert maps("binary")["shed/logo.png"] == identity("shed/logo.png")
    m = maps("multi_file")
    assert m["shed/run.sh"].is_identity
    typed = m["shed/py.typed"]
    assert (typed.old_path, typed.new_path, typed.added_new) == (None, "shed/py.typed", set())


def test_quoted_paths_are_keys() -> None:
    assert set(maps("quoted_paths")) == {
        "shed/café.py",
        "shed/foo bar.py",
        "shed/run.sh",
        "shed/run\tme.sh",
    }


def test_linemap_for() -> None:
    m = maps("single_hunk")
    assert linemap_for(m, "shed/calc.py") is m["shed/calc.py"]
    assert linemap_for(m, "shed/other.py") == identity("shed/other.py")


def test_swap_new_path_wins() -> None:
    # a.py -> b.py and b.py -> a.py: each key resolves to the map that produces it.
    m = build_linemaps(Patch((FilePatch("a.py", "b.py", ()), FilePatch("b.py", "a.py", ()))))
    assert (m["a.py"].old_path, m["b.py"].old_path) == ("b.py", "a.py")


def test_offset_hunk_placed_by_old_side() -> None:
    # Second header claims +10, but git apply puts old line 10 at 11 after the first hunk.
    text = "--- a/f.py\n+++ b/f.py\n@@ -1 +1,2 @@\n a\n+b\n@@ -10 +10 @@\n c\n"
    lm = build_linemaps(parse_patch(text))["f.py"]
    assert (lm.to_new(10), lm.to_old(11), lm.to_old(10)) == (11, 10, 9)


@pytest.mark.parametrize("name", sorted(p.stem for p in FIXTURES.glob("*.patch")))
def test_explicit_pairs_match_counting_rule(name: str) -> None:
    for lm in maps(name).values():
        bare = LineMap(lm.old_path, lm.new_path, {}, lm.deleted_old, lm.added_new)
        for old, new in lm.old_to_new.items():
            assert (bare.to_new(old), lm.to_old(new)) == (new, old), (name, old)


WORDS = ["a", "b", "c", "d", "e", "", "-", "+", "-- x", "@@ -1 +1 @@", "\\ x"]


def _random_pair(rng: random.Random) -> tuple[list[str], list[str]]:
    def word() -> str:
        return rng.choice(WORDS[:3] if rng.random() < 0.8 else WORDS)

    def words(most: int) -> list[str]:
        return [word() for _ in range(rng.randint(0, most))]

    if rng.random() < 0.3:  # unrelated lists, many short matches
        return words(12), words(12)
    old = words(40)
    new = list(old)
    for _ in range(rng.randint(0, 5)):  # localized edits, like a real patch
        at = rng.randint(0, len(new))
        cut = rng.randint(0, min(3, len(new) - at))
        new[at : at + cut] = words(3)
    return old, new


@pytest.mark.parametrize("context", [0, 1, 2, 3])
def test_against_difflib(context: int) -> None:
    """Parsed maps and the bare counting rule agree with SequenceMatcher on every line."""
    for seed in range(400):
        rng = random.Random(context * 10_000 + seed)
        old, new = _random_pair(rng)
        diff = difflib.unified_diff(old, new, "a/f.py", "b/f.py", n=context, lineterm="")
        patch = parse_patch("".join(f"{line}\n" for line in diff))
        assert patch.warnings == ()
        want_new: dict[int, int | None] = dict.fromkeys(range(1, len(old) + 1))
        want_old: dict[int, int | None] = dict.fromkeys(range(1, len(new) + 1))
        for tag, i1, i2, j1, _ in difflib.SequenceMatcher(None, old, new).get_opcodes():
            if tag == "equal":
                for k in range(i2 - i1):
                    want_new[i1 + k + 1], want_old[j1 + k + 1] = j1 + k + 1, i1 + k + 1
        lm = linemap_for(build_linemaps(patch), "f.py")
        bare = LineMap("f.py", "f.py", {}, lm.deleted_old, lm.added_new)
        case = (context, seed, old, new)
        for m in (lm, bare):
            assert m.deleted_old == {n for n, v in want_new.items() if v is None}, case
            assert m.added_new == {n for n, v in want_old.items() if v is None}, case
            assert {n: m.to_new(n) for n in want_new} == want_new, case
            assert {n: m.to_old(n) for n in want_old} == want_old, case
