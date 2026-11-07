"""How sundgren reads code, checked on a corpus of real files that are only read, never written.

Usage: uv run utils/outline_check.py LANGUAGE [--files N] [--root DIR]... [--bin PATH] [--seed N]

LANGUAGE is python, rust, typescript (with JavaScript) or c (with C++). It reports speed and
crashes, how much of the code falls inside an item, the items by kind, and names that look
misread. Where the language's own parser is installed, it compares the definitions sundgren finds
with that parser's: recall (of the parser's, how many sundgren finds) and precision (of
sundgren's, how many the parser has), with examples of each difference.

The parsers: Python's ast module; the TypeScript compiler in editors/vscode/node_modules (needs
node and `npm ci`); `rust-analyzer symbols` (`rustup component add rust-analyzer`). C and C++
have none: only the counts and the suspicious names.
"""

from __future__ import annotations

import argparse
import ast
import json
import os
import random
import re
import shutil
import subprocess
import sys
import sysconfig
import time
import warnings
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
HOME = Path.home()

EXTENSIONS = {
    "python": (".py",),
    "rust": (".rs",),
    "typescript": (".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"),
    "c": (".c", ".h", ".cc", ".cpp", ".cxx", ".hh", ".hpp", ".hxx"),
}
LINE_COMMENT = {"python": "#", "rust": "//", "typescript": "//", "c": "//"}


def default_roots(language: str) -> list[Path]:
    roots = {
        "python": [Path(sysconfig.get_paths()["stdlib"]), Path("/usr/lib/python3/dist-packages")],
        "rust": [HOME / ".cargo/registry/src"],
        "typescript": [ROOT / "editors/vscode/node_modules", ROOT / "editors/vscode/src"],
        "c": [Path("/usr/include"), Path("/usr/src"), HOME / ".cargo/registry/src"],
    }[language]
    return [r for r in roots if r.is_dir()]


@dataclass
class Item:
    kind: str
    qualified: str
    start: int
    end: int


@dataclass
class Report:
    files: int = 0
    seconds: float = 0.0
    crashes: list[str] = field(default_factory=list[str])
    lines: int = 0
    code_lines: int = 0
    code_in_items: int = 0
    kinds: Counter[str] = field(default_factory=Counter[str])
    suspicious: list[str] = field(default_factory=list[str])


def sample(roots: list[Path], extensions: tuple[str, ...], count: int, seed: int) -> list[Path]:
    """`count` files under `roots` with `extensions`, chosen at random but the same each run."""
    found: list[Path] = []
    for root in roots:
        for directory, _, names in os.walk(root):
            found += [Path(directory, n) for n in names if n.endswith(extensions)]
    found = sorted(f for f in found if f.is_file() and f.stat().st_size < 1_000_000)
    random.Random(seed).shuffle(found)
    return found[:count]


def flatten(items: list[dict[str, Any]], separator: str, prefix: str = "") -> list[Item]:
    out: list[Item] = []
    for it in items:
        name = str(it["name"])
        qualified = f"{prefix}{separator}{name}" if prefix else name
        out.append(Item(str(it["kind"]), qualified, int(it["start"]), int(it["end"])))
        children: list[dict[str, Any]] = it.get("children", [])
        out += flatten(children, separator, qualified)
    return out


def outline(binary: Path, files: list[Path], report: Report) -> dict[Path, list[dict[str, Any]]]:
    """sundgren's items of each file; a batch that fails is retried file by file."""
    out: dict[Path, list[dict[str, Any]]] = {}

    def run(batch: list[Path]) -> bool:
        p = subprocess.run(
            [binary, "outline", "--format", "json", *map(str, batch)],
            capture_output=True,
            text=True,
            timeout=600,
            check=False,
        )
        if p.returncode != 0:
            return False
        for f in json.loads(p.stdout)["files"]:
            out[Path(f["path"])] = f["items"]
        return True

    started = time.monotonic()
    for i in range(0, len(files), 100):
        batch = files[i : i + 100]
        if not run(batch):
            for f in batch:
                if not run([f]):
                    report.crashes.append(str(f))
    report.seconds = time.monotonic() - started
    return out


MACRO_NAME = re.compile(r"[A-Z_][A-Z0-9_]{2,}")
# Words that are keywords in the language, so never a function's name.
KEYWORDS = {
    "python": {"if", "for", "while", "return", "else", "elif", "def", "class", "lambda"},
    "rust": {"if", "for", "while", "loop", "match", "return", "else", "fn", "impl"},
    "typescript": {"if", "for", "while", "switch", "return", "else", "do", "case", "function"},
    "c": {"if", "for", "while", "switch", "return", "sizeof", "else", "do", "case"},
}


def measure(language: str, path: Path, items: list[Item], report: Report) -> None:
    """Lines of code inside items, kinds, and names that look misread."""
    try:
        source = path.read_text("utf-8", errors="replace").split("\n")
    except OSError:
        return
    inside = [False] * (len(source) + 2)
    for it in items:
        report.kinds[it.kind] += 1
        for n in range(it.start, min(it.end, len(source)) + 1):
            inside[n] = True
        name = it.qualified.rsplit(".", 1)[-1].rsplit("::", 1)[-1]
        if it.kind in ("function", "prototype") and (
            name in KEYWORDS[language] or (language == "c" and MACRO_NAME.fullmatch(name))
        ):
            report.suspicious.append(f"{it.kind} {it.qualified}  {path}:{it.start}")
        if not name and it.kind not in ("class", "struct", "union", "enum"):
            report.suspicious.append(f"{it.kind} without a name  {path}:{it.start}")
    comment = LINE_COMMENT[language]
    for n, text in enumerate(source, 1):
        line = text.strip()
        report.lines += 1
        if not line or line.startswith((comment, "/*", "*", '"""', "'''")):
            continue
        report.code_lines += 1
        report.code_in_items += inside[n]


# The languages' own parsers ---------------------------------------------------------------------

Reference = dict[Path, list[tuple[str, str]]]


def python_reference(files: list[Path]) -> Reference:
    out: Reference = {}
    warnings.simplefilter("ignore")
    for f in files:
        try:
            tree = ast.parse(f.read_text("utf-8"))
        except (SyntaxError, ValueError, UnicodeDecodeError, OSError):
            continue
        found: list[tuple[str, str]] = []

        def walk(node: ast.AST, prefix: str, found: list[tuple[str, str]] = found) -> None:
            for child in ast.iter_child_nodes(node):
                if isinstance(child, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                    qualified = f"{prefix}.{child.name}" if prefix else child.name
                    kind = "class" if isinstance(child, ast.ClassDef) else "function"
                    found.append((kind, qualified))
                    walk(child, qualified)
                else:
                    walk(child, prefix)

        walk(tree, "")
        out[f] = found
    return out


def typescript_reference(files: list[Path]) -> Reference | None:
    compiler = ROOT / "editors/vscode/node_modules/typescript"
    if shutil.which("node") is None or not compiler.is_dir():
        return None
    p = subprocess.run(
        ["node", str(ROOT / "utils/ts_declarations.cjs"), str(compiler)],
        input="\n".join(map(str, files)),
        capture_output=True,
        text=True,
        timeout=1800,
        check=True,
    )
    raw: dict[str, list[list[Any]]] = json.loads(p.stdout)
    return {Path(f): [(str(d[0]), str(d[1])) for d in decls] for f, decls in raw.items()}


RUST_KINDS = {
    "Function": "function",
    "Method": "function",
    "Struct": "struct",
    "Enum": "enum",
    "Union": "union",
    "Trait": "trait",
    "Impl": "impl",
    "Module": "module",
    "Const": "const",
    "Static": "static",
    "TypeAlias": "type",
    "Macro": "macro",
}
RA_NODE = re.compile(
    r'parent: (None|Some\((\d+)\)), label: "((?:[^"\\]|\\.)*)".*?kind: SymbolKind\((\w+)\)'
)


def rust_reference(files: list[Path]) -> Reference | None:
    if shutil.which("rust-analyzer") is None:
        return None
    out: Reference = {}
    for f in files:
        try:
            p = subprocess.run(
                ["rust-analyzer", "symbols"],
                input=f.read_text("utf-8"),
                capture_output=True,
                text=True,
                timeout=120,
                check=False,
            )
        except (OSError, UnicodeDecodeError, subprocess.TimeoutExpired):
            continue
        if p.returncode != 0:
            if "Unknown binary" in p.stderr:
                return None
            continue
        names: list[str] = []
        found: list[tuple[str, str]] = []
        for m in RA_NODE.finditer(p.stdout):
            parent, label, kind = m.group(2), m.group(3), m.group(4)
            label = label.removeprefix("impl ")
            qualified = f"{names[int(parent)]}::{label}" if parent is not None else label
            names.append(qualified)
            if kind in RUST_KINDS:
                found.append((RUST_KINDS[kind], qualified))
        out[f] = found
    return out


def compare(
    language: str,
    reference: Reference,
    ours: dict[Path, list[Item]],
) -> tuple[Counter[tuple[str, str, str]], Counter[tuple[str, str, str]], int]:
    """What the parser has and sundgren doesn't, the other way round, and how many match."""
    kinds = {k for found in reference.values() for k, _ in found} | {"prototype"}
    group = {"prototype": "function"}
    theirs: Counter[tuple[str, str, str]] = Counter()
    mine: Counter[tuple[str, str, str]] = Counter()
    for f, found in reference.items():
        for kind, qualified in found:
            theirs[(str(f), group.get(kind, kind), normal(language, qualified))] += 1
        for it in ours.get(f, []):
            if it.kind in kinds:
                mine[(str(f), group.get(it.kind, it.kind), normal(language, it.qualified))] += 1
    hits = sum((theirs & mine).values())
    return theirs - mine, mine - theirs, hits


def normal(language: str, qualified: str) -> str:
    """Names compared without whitespace, which the two sides space differently."""
    return re.sub(r"\s+", "", qualified) if language == "rust" else qualified


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    parser.add_argument("language", choices=sorted(EXTENSIONS))
    parser.add_argument("--files", type=int, default=1500, help="how many files (default 1500)")
    parser.add_argument("--root", type=Path, action="append", help="where to look for files")
    parser.add_argument("--bin", type=Path, default=ROOT / "target/release/sundgren")
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--examples", type=int, default=12, help="differences to show of each")
    args = parser.parse_args()
    language: str = args.language
    if not args.bin.is_file():
        sys.exit(f"{args.bin} is missing: cargo build --release")
    roots: list[Path] = args.root or default_roots(language)
    files = sample(roots, EXTENSIONS[language], args.files, args.seed)
    report = Report(files=len(files))
    separator = "::" if language in ("rust", "c") else "."
    raw = outline(args.bin, files, report)
    ours = {f: flatten(items, separator) for f, items in raw.items()}
    for f, items in ours.items():
        measure(language, f, items, report)

    print(f"{language}: {report.files} files under {', '.join(map(str, roots))}")
    print(f"  read in {report.seconds:.1f} s; crashes: {len(report.crashes)}")
    for crash in report.crashes[: args.examples]:
        print(f"    {crash}")
    share = 100 * report.code_in_items / max(1, report.code_lines)
    print(f"  {report.lines} lines; code lines inside an item: {share:.1f}%")
    print("  items:", ", ".join(f"{k} {n}" for k, n in report.kinds.most_common()))
    print(f"  suspicious names: {len(report.suspicious)}")
    for s in report.suspicious[: args.examples]:
        print(f"    {s}")

    references = {
        "python": lambda: python_reference(files),
        "typescript": lambda: typescript_reference(files),
        "rust": lambda: rust_reference(files),
    }
    reference = references[language]() if language in references else None
    if reference is None:
        print("  no parser of the language's own to compare with")
        return
    missed, extra, hits = compare(language, reference, ours)
    total_theirs = hits + sum(missed.values())
    total_mine = hits + sum(extra.values())
    print(
        f"  against the language's parser: recall {hits}/{total_theirs} = "
        f"{100 * hits / max(1, total_theirs):.1f}%, precision {hits}/{total_mine} = "
        f"{100 * hits / max(1, total_mine):.1f}%"
    )
    rng = random.Random(args.seed)
    for title, differences in (("missed", missed), ("extra", extra)):
        listed = list(differences.elements())
        rng.shuffle(listed)
        by_kind = Counter(kind for _, kind, _ in listed)
        print(f"  {title}: {', '.join(f'{k} {n}' for k, n in by_kind.most_common())}")
        for f, kind, qualified in listed[: args.examples]:
            print(f"    {kind} {qualified}  ({f})")


if __name__ == "__main__":
    main()
