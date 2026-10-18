"""Path normalization and snapshot re-keying."""

from __future__ import annotations

from pathlib import Path

import pytest

from sundgren.core.ingest.lcov import parse
from sundgren.core.model import BranchKey, FileCoverage, FunctionRecord, Snapshot
from sundgren.core.normalize import normalize_path, normalize_snapshot

FIXTURES = Path(__file__).resolve().parents[1] / "fixtures" / "lcov"


@pytest.mark.parametrize(
    ("raw", "roots", "expected"),
    [
        ("/repo/src/a.py", ["/repo"], ("src/a.py", False)),
        # longest root wins, regardless of order
        ("/repo/sub/src/a.py", ["/repo", "/repo/sub"], ("src/a.py", False)),
        ("/repo/sub/src/a.py", ["/repo/sub", "/repo"], ("src/a.py", False)),
        # component boundary only
        ("/repo2/a.py", ["/repo"], ("/repo2/a.py", True)),
        ("/ci/build_x/a.rs", ["/ci/build"], ("/ci/build_x/a.rs", True)),
        # strip-prefix with and without trailing slash
        ("/ci/build/src/a.rs", ["/ci/build/"], ("src/a.rs", False)),
        ("/ci/build/src/a.rs", ["/ci/build"], ("src/a.rs", False)),
        ("shed_py/shed/calc.py", ["shed_py/"], ("shed/calc.py", False)),
        ("shed_py/shed/calc.py", ["shed_py"], ("shed/calc.py", False)),
        # relative stays relative, './' dropped
        ("shed/calc.py", ["/repo"], ("shed/calc.py", False)),
        ("./shed/calc.py", [], ("shed/calc.py", False)),
        (".\\shed\\calc.py", [], ("shed/calc.py", False)),
        # '..' resolved lexically
        ("/repo/tests/../src/a.py", ["/repo"], ("src/a.py", False)),
        ("src/../shed/a.py", [], ("shed/a.py", False)),
        ("/../repo/a.py", ["/repo"], ("a.py", False)),
        ("/repo//src///a.py", ["/repo/"], ("src/a.py", False)),
        # backslashes and drive letters
        ("C:\\ci\\repo\\src\\lib.rs", ["C:\\ci\\repo"], ("src/lib.rs", False)),
        ("c:\\ci\\repo\\src\\lib.rs", ["C:/ci/repo/"], ("src/lib.rs", False)),
        ("src\\lib.rs", [], ("src/lib.rs", False)),
        ("D:\\deps\\x.rs", ["C:/ci/repo"], ("D:/deps/x.rs", True)),
        ("C:/x.rs", ["/x"], ("C:/x.rs", True)),
        # external absolute: key is the cleaned path
        (
            "/usr/lib/python3.12/json/__init__.py",
            ["/repo"],
            ("/usr/lib/python3.12/json/__init__.py", True),
        ),
        ("/opt/../opt/lib/x.py", [], ("/opt/lib/x.py", True)),
        ("/repo", ["/repo"], ("/repo", True)),
        # escaping relative
        ("../other/x.py", ["/repo"], ("../other/x.py", True)),
        ("src/../../x.py", [], ("../x.py", True)),
        # empty roots are ignored
        ("/repo/a.py", ["", ".", "./"], ("/repo/a.py", True)),
    ],
)
def test_normalize_path(raw: str, roots: list[str], expected: tuple[str, bool]) -> None:
    assert normalize_path(raw, roots) == expected


def test_symlinked_root(tmp_path: Path) -> None:
    real = tmp_path / "real"
    (real / "src").mkdir(parents=True)
    link = tmp_path / "link"
    link.symlink_to(real, target_is_directory=True)
    # Root given via the symlink, SF reported via the real path (llvm-cov resolves symlinks).
    assert normalize_path(str(real / "src" / "a.rs"), [str(link)]) == ("src/a.rs", False)
    # Root given as the real path, SF reported via the symlink.
    assert normalize_path(str(link / "src" / "a.rs"), [str(real)]) == ("src/a.rs", False)
    assert normalize_path(str(tmp_path / "elsewhere.rs"), [str(link)]) == (
        str(tmp_path / "elsewhere.rs"),
        True,
    )


def _fc(
    path: str, lines: dict[int, int], functions: tuple[FunctionRecord, ...] = ()
) -> FileCoverage:
    return FileCoverage(path=path, lines=lines, branches={}, functions=functions, external=False)


def test_normalize_snapshot_merges_collisions() -> None:
    snap = Snapshot(
        files={
            "/repo/shed/calc.py": _fc(
                "/repo/shed/calc.py", {5: 1, 1: 1}, (FunctionRecord("f", 1, 3, 1),)
            ),
            "./shed/calc.py": _fc("./shed/calc.py", {5: 2, 2: 0}, (FunctionRecord("f", 1, 3, 2),)),
            "/usr/lib/x.py": _fc("/usr/lib/x.py", {1: 1}),
            "../up.py": _fc("../up.py", {1: 0}),
        },
        test_names=("t1", "t2"),
    )
    out = normalize_snapshot(snap, ["/repo"])
    assert out.test_names == ("t1", "t2")
    assert list(out.files) == ["../up.py", "/usr/lib/x.py", "shed/calc.py"]
    calc = out.files["shed/calc.py"]
    assert calc.path == "shed/calc.py"
    assert calc.external is False
    assert list(calc.lines.items()) == [(1, 1), (2, 0), (5, 3)]
    assert calc.functions == (FunctionRecord("f", 1, 3, 3),)
    assert out.files["/usr/lib/x.py"].external is True
    assert out.files["/usr/lib/x.py"].path == "/usr/lib/x.py"
    assert out.files["../up.py"].external is True


def test_normalize_snapshot_merges_branches() -> None:
    key = BranchKey(2, "0", "0")
    a = FileCoverage(path="/r/x.c", lines={}, branches={key: None}, functions=(), external=False)
    b = FileCoverage(path="x.c", lines={}, branches={key: 4}, functions=(), external=False)
    out = normalize_snapshot(Snapshot(files={"/r/x.c": a, "x.c": b}, test_names=()), ["/r"])
    assert out.files["x.c"].branches == {key: 4}


def test_normalize_llvm_cov_fixture() -> None:
    snap = parse((FIXTURES / "llvm_cov.lcov").read_text(encoding="utf-8"))
    out = normalize_snapshot(snap, ["/home/runner/work/shed/shed_rs/"])
    assert list(out.files) == ["src/lib.rs", "src/util.rs"]
    assert all(not fc.external for fc in out.files.values())
    raw = snap.files["/home/runner/work/shed/shed_rs/src/lib.rs"]
    assert out.files["src/lib.rs"].lines == raw.lines
    assert out.files["src/lib.rs"].functions == raw.functions


def test_normalize_snapshot_is_idempotent() -> None:
    snap = parse((FIXTURES / "tarpaulin.lcov").read_text(encoding="utf-8"))
    once = normalize_snapshot(snap, ["/builds/shed/shed_rs"])
    assert list(once.files) == ["src/lib.rs"]
    assert normalize_snapshot(once, ["/builds/shed/shed_rs"]) == once
