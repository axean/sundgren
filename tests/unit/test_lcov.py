"""lcov parse/dump: fixtures, round-trip, merge semantics, edge records."""

from __future__ import annotations

import logging
from pathlib import Path

import pytest

from sundgren.core.ingest.lcov import LcovIngestor, dump, parse
from sundgren.core.model import BranchKey, FileCoverage, FunctionRecord, Snapshot

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures" / "lcov"
ALL_FIXTURES = sorted(p.name for p in FIXTURES.glob("*.lcov"))

RS = "/home/runner/work/shed/shed_rs/src"
CRATE = "CsbDqzXfLQacH_4shed"


def _load(name: str) -> Snapshot:
    return parse((FIXTURES / name).read_text(encoding="utf-8"))


def _fn(fc: FileCoverage, name: str) -> FunctionRecord:
    (match,) = [f for f in fc.functions if f.name == name]
    return match


def test_fixture_set_is_complete() -> None:
    assert ALL_FIXTURES == [
        "coveragepy.lcov",
        "duplicate_sf.lcov",
        "lcov2_fn.lcov",
        "llvm_cov.lcov",
        "tarpaulin.lcov",
    ]


def test_coveragepy() -> None:
    snap = _load("coveragepy.lcov")
    assert snap.test_names == ()
    assert list(snap.files) == ["shed/__init__.py", "shed/calc.py"]
    init = snap.files["shed/__init__.py"]
    assert (init.lines, init.branches, init.functions) == ({}, {}, ())
    calc = snap.files["shed/calc.py"]
    assert calc.path == "shed/calc.py"
    assert calc.external is False
    assert len(calc.lines) == 22
    assert (calc.lines[1], calc.lines[6], calc.lines[31]) == (1, 0, 1)
    assert 3 not in calc.lines
    assert len(calc.branches) == 10
    assert calc.branches[BranchKey(6, "0", "jump to line 7")] is None
    assert calc.branches[BranchKey(12, "0", "jump to line 13")] == 1
    assert calc.branches[BranchKey(14, "0", "jump to line 15")] == 0
    assert calc.branches[BranchKey(30, "0", "return from function 'note'")] == 0
    assert [f.name for f in calc.functions] == [
        "add",
        "divide",
        "classify",
        "Acc.__init__",
        "Acc.push",
        "note",
    ]
    assert _fn(calc, "divide") == FunctionRecord("divide", 5, 8, 0)
    assert _fn(calc, "Acc.push") == FunctionRecord("Acc.push", 23, 26, 0)


def test_llvm_cov() -> None:
    snap = _load("llvm_cov.lcov")
    assert snap.test_names == ()
    assert list(snap.files) == [f"{RS}/lib.rs", f"{RS}/util.rs"]
    lib = snap.files[f"{RS}/lib.rs"]
    assert lib.path == f"{RS}/lib.rs"
    assert lib.external is False
    assert lib.branches == {}
    assert len(lib.lines) == 24
    assert (lib.lines[3], lib.lines[8], lib.lines[12]) == (0, 2, 0)
    assert len(lib.functions) == 6
    # Names stay mangled.
    assert _fn(lib, f"_RNv{CRATE}6divide") == FunctionRecord(f"_RNv{CRATE}6divide", 1, None, 1)
    assert _fn(lib, f"_RNv{CRATE}8classify").hits == 2
    closure = f"_RNCNvNt{CRATE}5tests11test_divide0B5_"
    assert _fn(lib, closure) == FunctionRecord(closure, 22, None, 1)
    util = snap.files[f"{RS}/util.rs"]
    assert util.functions == (FunctionRecord(f"_RNvNt{CRATE}4util5clamp", 1, None, 0),)
    assert set(util.lines.values()) == {0}


def test_tarpaulin() -> None:
    snap = _load("tarpaulin.lcov")
    assert snap.test_names == ()  # empty TN: is dropped
    (path,) = snap.files
    assert path == "/builds/shed/shed_rs/src/lib.rs"
    fc = snap.files[path]
    assert fc.lines == {1: 2, 2: 1, 3: 0, 5: 2, 8: 1, 9: 2, 10: 1, 11: 2, 12: 0, 14: 1}
    assert fc.branches == {}
    assert fc.functions == (
        FunctionRecord("divide", 1, None, 2),
        FunctionRecord("classify", 8, None, 1),
    )


def test_lcov2_fn() -> None:
    snap = _load("lcov2_fn.lcov")
    (fc,) = snap.files.values()
    assert fc.path == "/home/ci/ringbuf/src/ring.cpp"
    assert fc.functions == (
        FunctionRecord("ring::Ring::push(int)", 6, 12, 7),
        FunctionRecord("ring::Ring::pop()", 14, 21, 0),
        FunctionRecord("ring::minmax(int, int)", 23, 26, 2),
    )
    assert fc.branches[BranchKey(9, "e0", "1")] == 2
    assert fc.branches[BranchKey(15, "0", "0")] is None
    assert fc.branches[BranchKey(7, "0", "1")] == 5
    assert len(fc.lines) == 18


def test_duplicate_sf_merged() -> None:
    snap = _load("duplicate_sf.lcov")
    assert snap.test_names == ("unit", "integration")
    assert list(snap.files) == ["src/ring.c", "src/util.c"]
    ring = snap.files["src/ring.c"]
    assert ring.lines[4] == 16
    assert ring.lines[6] == 3
    assert ring.lines[11] == 2
    assert ring.lines[19] == 0
    assert ring.branches == {
        BranchKey(5, "0", "0"): 3,
        BranchKey(5, "0", "1"): 13,
        BranchKey(13, "0", "0"): 1,  # '-' + 1
        BranchKey(13, "0", "1"): 1,
        BranchKey(20, "0", "0"): None,  # '-' + '-'
        BranchKey(20, "0", "1"): None,
    }
    assert ring.functions == (
        FunctionRecord("ring_push", 4, 9, 16),
        FunctionRecord("ring_pop", 11, 17, 2),
        FunctionRecord("ring_len", 19, 22, 0),
    )
    assert snap.files["src/util.c"].lines == {1: 5, 2: 5, 3: 5}


@pytest.mark.parametrize("name", ALL_FIXTURES)
def test_round_trip(name: str) -> None:
    snap = _load(name)
    text = dump(snap)
    assert parse(text) == snap
    assert dump(parse(text)) == text


def test_merge_semantics() -> None:
    text = (
        "TN:a\nSF:x.c\nFN:1,f\nFNDA:1,f\nFN:5,7,g\nDA:1,1\nDA:2,0\n"
        "BRDA:2,0,0,-\nBRDA:2,0,1,-\nBRDA:3,0,0,4\nBRDA:3,0,1,-\nend_of_record\n"
        "TN:b\nSF:x.c\nFN:1,3,f\nFNDA:2,f\nFN:5,g\nFNDA:0,g\nFN:9,h\nDA:2,5\nDA:9,0\n"
        "BRDA:2,0,0,-\nBRDA:2,0,1,2\nBRDA:3,0,0,1\nBRDA:3,0,1,0\nend_of_record\n"
    )
    fc = parse(text).files["x.c"]
    assert fc.lines == {1: 1, 2: 5, 9: 0}
    assert fc.branches == {
        BranchKey(2, "0", "0"): None,
        BranchKey(2, "0", "1"): 2,
        BranchKey(3, "0", "0"): 5,
        BranchKey(3, "0", "1"): 0,
    }
    assert fc.functions == (
        FunctionRecord("f", 1, 3, 3),  # end filled in from the second record
        FunctionRecord("g", 5, 7, 0),  # no FNDA, then 0
        FunctionRecord("h", 9, None, None),
    )


def test_merged_dicts_are_sorted() -> None:
    fc = parse("SF:x\nDA:5,1\nend_of_record\nSF:x\nDA:2,1\nDA:9,1\nend_of_record\n").files["x"]
    assert list(fc.lines) == [2, 5, 9]


def test_repeated_records_within_one_section_are_summed() -> None:
    fc = parse("SF:x\nDA:1,2\nDA:1,3\nFN:1,f\nFNDA:1,f\nFNDA:4,f\nend_of_record\n").files["x"]
    assert fc.lines == {1: 5}
    assert fc.functions == (FunctionRecord("f", 1, None, 5),)


def test_da_checksum_is_ignored() -> None:
    fc = parse("SF:x.py\nDA:1,1,5kX8cZ4xh/H3dUOhjJ9P3w\nDA:2,0,abc\nend_of_record\n").files["x.py"]
    assert fc.lines == {1: 1, 2: 0}


@pytest.mark.parametrize(("span", "end"), [("4", None), ("4,9", 9)])
def test_function_names_with_commas(span: str, end: int | None) -> None:
    name = "std::swap<int, char>(int&, char&)"
    snap = parse(f"SF:x.cpp\nFN:{span},{name}\nFNDA:3,{name}\nend_of_record\n")
    assert snap.files["x.cpp"].functions == (FunctionRecord(name, 4, end, 3),)
    assert parse(dump(snap)) == snap


def test_branch_id_with_commas() -> None:
    snap = parse("SF:x.c\nBRDA:3,0,(a, b) == True,7\nend_of_record\n")
    assert snap.files["x.c"].branches == {BranchKey(3, "0", "(a, b) == True"): 7}
    assert parse(dump(snap)) == snap


def test_lcov22_fnl_fna() -> None:
    text = (
        "SF:x.cpp\nFNL:0,4,9\nFNA:0,3,f(int)\nFNA:0,1,f(long)\n"
        "FNL:1,12\nFNA:1,0,g(int, int)\nend_of_record\n"
    )
    fc = parse(text).files["x.cpp"]
    assert fc.functions == (
        FunctionRecord("f(int)", 4, 9, 3),
        FunctionRecord("f(long)", 4, 9, 1),
        FunctionRecord("g(int, int)", 12, None, 0),
    )


def test_summary_records_are_dropped() -> None:
    a = parse("SF:x\nDA:1,1\nLF:99\nLH:42\nFNF:7\nFNH:1\nBRF:3\nBRH:0\nend_of_record\n")
    b = parse("SF:x\nDA:1,1\nend_of_record\n")
    assert a == b


def test_unknown_records_are_logged_and_ignored(caplog: pytest.LogCaptureFixture) -> None:
    text = "SF:x\nVER:abc\nMCDC:3,2,t,1,0,a && b\nDA:1,1\nend_of_record\nDA:2,1\nweird\n"
    with caplog.at_level(logging.DEBUG, logger="sundgren.core.ingest.lcov"):
        snap = parse(text)
    assert snap == parse("SF:x\nDA:1,1\nend_of_record\n")
    messages = [r.getMessage() for r in caplog.records]
    assert any("MCDC" in m for m in messages)
    assert any("outside SF" in m for m in messages)
    assert any("'weird'" in m for m in messages)
    assert not any("VER" in m for m in messages)


def test_fnda_without_fn_is_dropped(caplog: pytest.LogCaptureFixture) -> None:
    with caplog.at_level(logging.DEBUG, logger="sundgren.core.ingest.lcov"):
        fc = parse("SF:x\nFNDA:1,ghost\nend_of_record\n").files["x"]
    assert fc.functions == ()
    assert any("ghost" in r.getMessage() for r in caplog.records)


def test_missing_end_of_record_and_crlf() -> None:
    text = "TN:t\r\nSF:a\r\nDA:1,1\r\nSF:b\r\nDA:2,0\r\n\r\n"
    snap = parse(text)
    assert snap.test_names == ("t",)
    assert snap.files["a"].lines == {1: 1}
    assert snap.files["b"].lines == {2: 0}


def test_leading_bom_is_ignored() -> None:
    snap = parse("\ufeffTN:t\nSF:a.py\nDA:1,1\nend_of_record\nSF:b.py\nDA:1,0\nend_of_record\n")
    assert snap.test_names == ("t",)
    assert list(snap.files) == ["a.py", "b.py"]


@pytest.mark.parametrize("sep", ["\x0b", "\x0c", "\x1c", "\x85", "\u2028", "\u2029"])
def test_only_newline_ends_a_record(sep: str) -> None:
    path = f"src/a{sep}b.py"
    snap = parse(f"SF:{path}\nFN:1,f{sep}g\nDA:1,1\nend_of_record\n")
    assert list(snap.files) == [path]
    assert snap.files[path].functions == (FunctionRecord(f"f{sep}g", 1, None, None),)
    assert parse(dump(snap)) == snap


def test_test_names_deduplicated_in_first_seen_order() -> None:
    text = "TN:z\nSF:a\nend_of_record\nTN:\nTN:a\nTN:z\nSF:a\nend_of_record\n"
    assert parse(text).test_names == ("z", "a")


def test_windows_sf_path_kept_raw() -> None:
    snap = parse("SF:C:\\ci\\repo\\src\\lib.rs\nDA:1,1\nend_of_record\n")
    assert list(snap.files) == ["C:\\ci\\repo\\src\\lib.rs"]


@pytest.mark.parametrize(
    "record",
    ["DA:1", "DA:x,1", "BRDA:1,0,1", "BRDA:1,0,0,x", "FN:x,f", "FN:4", "FNDA:1", "FNA:9,1,f"],
)
def test_malformed_record_raises(record: str) -> None:
    with pytest.raises(ValueError, match=r"lcov line 2: malformed"):
        parse(f"SF:x\n{record}\nend_of_record\n")


def test_dump_format() -> None:
    snap = Snapshot(
        files={
            "b.c": FileCoverage(
                path="b.c",
                lines={3: 0, 1: 2},
                branches={BranchKey(1, "0", "1"): None, BranchKey(1, "0", "0"): 2},
                functions=(FunctionRecord("f", 1, 4, 2), FunctionRecord("g", 6, None, None)),
                external=False,
            ),
            "a.c": FileCoverage(path="a.c", lines={}, branches={}, functions=(), external=False),
        },
        test_names=("unit", "e2e"),
    )
    assert dump(snap) == (
        "TN:unit\nTN:e2e\n"
        "SF:a.c\nend_of_record\n"
        "SF:b.c\nFN:1,4,f\nFN:6,g\nFNDA:2,f\nFNF:2\nFNH:1\n"
        "BRDA:1,0,0,2\nBRDA:1,0,1,-\nBRF:2\nBRH:1\n"
        "DA:1,2\nDA:3,0\nLF:2\nLH:1\n"
        "end_of_record\n"
    )
    assert parse(dump(snap)) == snap


def test_dump_empty() -> None:
    assert dump(Snapshot(files={}, test_names=())) == ""
    assert parse("") == Snapshot(files={}, test_names=())


def test_ingestor() -> None:
    text = (FIXTURES / "llvm_cov.lcov").read_text(encoding="utf-8")
    assert LcovIngestor().ingest(text) == parse(text)
